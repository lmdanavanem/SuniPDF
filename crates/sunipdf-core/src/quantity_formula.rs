//! Read-only decimal expressions with one bounded unit-conversion builtin; no I/O.

use super::{
    bad, decimal_units, format_units, short, QuantityRow, QuantityUnit, TypedValue, DECIMAL_LIMIT,
    DECIMAL_SCALE,
};
use crate::EngineError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormulaColumn {
    pub name: String,
    pub expression: String,
    /// Exact required source unit when using `quantity`; no implicit unit conversion.
    pub quantity_unit: Option<QuantityUnit>,
    pub rounding_decimals: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormulaError {
    MissingField,
    NonNumericField,
    MissingQuantity,
    UnverifiedQuantity,
    UnitMismatch,
    DivisionByZero,
    Overflow,
}
impl FormulaError {
    pub fn id(self) -> &'static str {
        match self {
            Self::MissingField => "missing_field",
            Self::NonNumericField => "non_numeric_field",
            Self::MissingQuantity => "missing_quantity",
            Self::UnverifiedQuantity => "unverified_quantity",
            Self::UnitMismatch => "unit_mismatch",
            Self::DivisionByZero => "division_by_zero",
            Self::Overflow => "overflow",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormulaValue {
    pub value: Option<String>,
    pub unit: Option<QuantityUnit>,
    pub error: Option<FormulaError>,
}

#[derive(Debug)]
enum Node {
    Number(i128),
    Field(String),
    Quantity,
    Convert(Box<Node>, QuantityUnit),
    Neg(Box<Node>),
    Binary(u8, Box<Node>, Box<Node>),
}
pub(super) struct CompiledFormula {
    column: FormulaColumn,
    node: Node,
    uses_unit_conversion: bool,
}

impl FormulaColumn {
    pub fn validate(&self) -> Result<(), EngineError> {
        self.compile().map(|_| ())
    }
    pub fn evaluate(&self, row: &QuantityRow) -> Result<FormulaValue, EngineError> {
        Ok(self.compile()?.evaluate(row))
    }
    pub(super) fn compile(&self) -> Result<CompiledFormula, EngineError> {
        if self.name.trim() != self.name
            || self.name.is_empty()
            || !short(&self.name, 64)
            || self.expression.is_empty()
            || self.expression.len() > 512
            || self.rounding_decimals > 6
        {
            return Err(bad("invalid formula name, expression bounds or rounding"));
        }
        let mut parser = Parser {
            text: self.expression.as_bytes(),
            at: 0,
            tokens: 0,
            uses_quantity: false,
            uses_unit_conversion: false,
        };
        let node = parser.expression(0)?;
        parser.space();
        if parser.at != parser.text.len() || (parser.uses_quantity && self.quantity_unit.is_none())
        {
            return Err(bad(
                "formula requires complete arithmetic and an explicit quantity unit",
            ));
        }
        Ok(CompiledFormula {
            column: self.clone(),
            node,
            uses_unit_conversion: parser.uses_unit_conversion,
        })
    }
    pub(super) fn uses_unit_conversion(&self) -> bool {
        self.compile()
            .is_ok_and(|compiled| compiled.uses_unit_conversion)
    }
}

impl CompiledFormula {
    pub(super) fn evaluate(&self, row: &QuantityRow) -> FormulaValue {
        match evaluate(&self.node, row, self.column.quantity_unit) {
            Ok((value, unit)) => {
                let divisor = 10i128.pow(6 - u32::from(self.column.rounding_decimals));
                let rounded = divide_round(value, divisor).expect("positive bounded divisor");
                FormulaValue {
                    value: Some(format_units(
                        rounded,
                        u32::from(self.column.rounding_decimals),
                        false,
                    )),
                    unit,
                    error: None,
                }
            }
            Err(error) => FormulaValue {
                value: None,
                unit: None,
                error: Some(error),
            },
        }
    }
}

struct Parser<'a> {
    text: &'a [u8],
    at: usize,
    tokens: usize,
    uses_quantity: bool,
    uses_unit_conversion: bool,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self.text.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn token(&mut self) -> Result<(), EngineError> {
        self.tokens += 1;
        if self.tokens > 128 {
            return Err(bad("formula exceeds 128 tokens"));
        }
        Ok(())
    }
    fn punctuation(&mut self, expected: u8) -> Result<(), EngineError> {
        self.space();
        if self.text.get(self.at) != Some(&expected) {
            return Err(bad("conversion requires convert(expression, unit)"));
        }
        self.token()?;
        self.at += 1;
        Ok(())
    }
    fn conversion(&mut self, depth: usize) -> Result<Node, EngineError> {
        self.at += b"convert".len();
        self.punctuation(b'(')?;
        let inner = self.expression(depth + 1)?;
        self.punctuation(b',')?;
        self.space();
        self.token()?;
        let start = self.at;
        while self
            .text
            .get(self.at)
            .is_some_and(u8::is_ascii_alphanumeric)
        {
            self.at += 1;
        }
        let id = std::str::from_utf8(&self.text[start..self.at])
            .map_err(|_| bad("invalid conversion unit"))?;
        let unit = QuantityUnit::from_id(id)
            .filter(|unit| geometric_family(*unit).is_some())
            .ok_or_else(|| {
                bad("conversion requires mm, cm, m, in or ft, optionally suffixed 2 or 3")
            })?;
        self.punctuation(b')')?;
        self.uses_unit_conversion = true;
        Ok(Node::Convert(Box::new(inner), unit))
    }
    fn expression(&mut self, depth: usize) -> Result<Node, EngineError> {
        let mut node = self.product(depth + 1)?;
        loop {
            self.space();
            let Some(op @ (b'+' | b'-')) = self.text.get(self.at).copied() else {
                break;
            };
            self.token()?;
            self.at += 1;
            node = Node::Binary(op, Box::new(node), Box::new(self.product(depth + 1)?));
        }
        Ok(node)
    }
    fn product(&mut self, depth: usize) -> Result<Node, EngineError> {
        let mut node = self.atom(depth + 1)?;
        loop {
            self.space();
            let Some(op @ (b'*' | b'/')) = self.text.get(self.at).copied() else {
                break;
            };
            self.token()?;
            self.at += 1;
            node = Node::Binary(op, Box::new(node), Box::new(self.atom(depth + 1)?));
        }
        Ok(node)
    }
    fn atom(&mut self, depth: usize) -> Result<Node, EngineError> {
        if depth > 32 {
            return Err(bad("formula nesting exceeds 32 levels"));
        }
        self.space();
        self.token()?;
        match self.text.get(self.at).copied() {
            Some(b'-') => { self.at += 1; Ok(Node::Neg(Box::new(self.atom(depth + 1)?))) }
            Some(b'+') => { self.at += 1; self.atom(depth + 1) }
            Some(b'(') => {
                self.at += 1; let node = self.expression(depth + 1)?; self.space();
                if self.text.get(self.at) != Some(&b')') { return Err(bad("formula has an unclosed parenthesis")); }
                self.at += 1; Ok(node)
            }
            Some(b'[') => {
                self.at += 1; let start = self.at;
                while self.text.get(self.at).is_some_and(|c| *c != b']') { self.at += 1; }
                if self.text.get(self.at) != Some(&b']') { return Err(bad("formula has an unclosed field reference")); }
                let name = std::str::from_utf8(&self.text[start..self.at]).map_err(|_| bad("invalid formula field"))?;
                if name.is_empty() || name.trim() != name || !short(name, 64) || name.contains('[') {
                    return Err(bad("invalid formula field reference"));
                }
                self.at += 1; Ok(Node::Field(name.to_owned()))
            }
            Some(b'0'..=b'9') => {
                let start = self.at;
                while self.text.get(self.at).is_some_and(|c| c.is_ascii_digit() || *c == b'.') { self.at += 1; }
                let text = std::str::from_utf8(&self.text[start..self.at]).map_err(|_| bad("invalid decimal"))?;
                Ok(Node::Number(decimal_units(text)?))
            }
            Some(b'q') if self.text[self.at..].starts_with(b"quantity") => {
                self.at += 8; self.uses_quantity = true; Ok(Node::Quantity)
            }
            Some(b'c') if self.text[self.at..].starts_with(b"convert") => self.conversion(depth),
            _ => Err(bad("formula accepts decimals, quantity, [numeric fields], convert(expression, unit), parentheses and + - * / only")),
        }
    }
}

fn divide_round(numerator: i128, denominator: i128) -> Result<i128, FormulaError> {
    if denominator == 0 {
        return Err(FormulaError::DivisionByZero);
    }
    let sign = if (numerator < 0) != (denominator < 0) {
        -1
    } else {
        1
    };
    let n = numerator.checked_abs().ok_or(FormulaError::Overflow)?;
    let d = denominator.checked_abs().ok_or(FormulaError::Overflow)?;
    let half = d / 2 + d % 2;
    let rounded = (n / d)
        .checked_add(i128::from(n % d >= half))
        .ok_or(FormulaError::Overflow)?;
    rounded.checked_mul(sign).ok_or(FormulaError::Overflow)
}

fn geometric_family(unit: QuantityUnit) -> Option<([QuantityUnit; 3], usize)> {
    use QuantityUnit::*;
    [
        [Millimetres, SquareMillimetres, CubicMillimetres],
        [Centimetres, SquareCentimetres, CubicCentimetres],
        [Metres, SquareMetres, CubicMetres],
        [Inches, SquareInches, CubicInches],
        [Feet, SquareFeet, CubicFeet],
    ]
    .into_iter()
    .find_map(|family| {
        family
            .iter()
            .position(|candidate| *candidate == unit)
            .map(|index| (family, index + 1))
    })
}

fn combined_unit(
    left: Option<QuantityUnit>,
    right: Option<QuantityUnit>,
    divide: bool,
) -> Result<Option<QuantityUnit>, FormulaError> {
    let (left, right) = match (left, right) {
        (left, None) => return Ok(left),
        (None, right) if !divide => return Ok(right),
        (Some(left), Some(right)) if divide && left == right => return Ok(None),
        (Some(left), Some(right)) => (left, right),
        _ => return Err(FormulaError::UnitMismatch),
    };
    let (family, left_power) = geometric_family(left).ok_or(FormulaError::UnitMismatch)?;
    let (right_family, right_power) = geometric_family(right).ok_or(FormulaError::UnitMismatch)?;
    if family != right_family {
        return Err(FormulaError::UnitMismatch);
    }
    let power = if divide {
        left_power.checked_sub(right_power)
    } else {
        left_power.checked_add(right_power)
    }
    .filter(|power| (1..=3).contains(power))
    .ok_or(FormulaError::UnitMismatch)?;
    Ok(Some(family[power - 1]))
}

fn convert_units(
    value: i128,
    source: Option<QuantityUnit>,
    target: QuantityUnit,
) -> Result<i128, FormulaError> {
    let (source_family, source_power) = source
        .and_then(geometric_family)
        .ok_or(FormulaError::UnitMismatch)?;
    let (target_family, target_power) =
        geometric_family(target).ok_or(FormulaError::UnitMismatch)?;
    if source_power != target_power {
        return Err(FormulaError::UnitMismatch);
    }
    // Exact international length definitions in micrometres; powers preserve area/volume.
    let micrometres = |unit| match unit {
        QuantityUnit::Millimetres => 1_000i128,
        QuantityUnit::Centimetres => 10_000,
        QuantityUnit::Metres => 1_000_000,
        QuantityUnit::Inches => 25_400,
        QuantityUnit::Feet => 304_800,
        _ => unreachable!("geometric families start with a length unit"),
    };
    let numerator = micrometres(source_family[0])
        .checked_pow(source_power as u32)
        .and_then(|factor| value.checked_mul(factor))
        .ok_or(FormulaError::Overflow)?;
    let denominator = micrometres(target_family[0])
        .checked_pow(target_power as u32)
        .ok_or(FormulaError::Overflow)?;
    divide_round(numerator, denominator)
}

fn evaluate(
    node: &Node,
    row: &QuantityRow,
    expected_unit: Option<QuantityUnit>,
) -> Result<(i128, Option<QuantityUnit>), FormulaError> {
    let result = match node {
        Node::Number(value) => (*value, None),
        Node::Field(name) => match row.fields.custom.get(name) {
            Some(TypedValue::Number { value, unit }) => {
                (value.units().map_err(|_| FormulaError::Overflow)?, *unit)
            }
            Some(_) => return Err(FormulaError::NonNumericField),
            None => return Err(FormulaError::MissingField),
        },
        Node::Quantity => {
            if row
                .measurement_validity
                .is_some_and(|validity| !validity.is_current())
            {
                return Err(FormulaError::UnverifiedQuantity);
            }
            let quantity = row.quantity.as_ref().ok_or(FormulaError::MissingQuantity)?;
            if Some(quantity.unit) != expected_unit {
                return Err(FormulaError::UnitMismatch);
            }
            let scaled = quantity.value * DECIMAL_SCALE as f64;
            if !scaled.is_finite() || scaled.abs() > DECIMAL_LIMIT as f64 {
                return Err(FormulaError::Overflow);
            }
            (scaled.round() as i128, Some(quantity.unit))
        }
        Node::Neg(inner) => {
            let (value, unit) = evaluate(inner, row, expected_unit)?;
            (-value, unit)
        }
        Node::Convert(inner, target) => {
            let (value, source) = evaluate(inner, row, expected_unit)?;
            (convert_units(value, source, *target)?, Some(*target))
        }
        Node::Binary(op, left, right) => {
            let (a, ua) = evaluate(left, row, expected_unit)?;
            let (b, ub) = evaluate(right, row, expected_unit)?;
            match op {
                b'+' | b'-' if ua == ub => (
                    if *op == b'+' {
                        a.checked_add(b)
                    } else {
                        a.checked_sub(b)
                    }
                    .ok_or(FormulaError::Overflow)?,
                    ua,
                ),
                b'*' => {
                    let unit = combined_unit(ua, ub, false)?;
                    (
                        divide_round(
                            a.checked_mul(b).ok_or(FormulaError::Overflow)?,
                            DECIMAL_SCALE,
                        )?,
                        unit,
                    )
                }
                b'/' => {
                    let unit = combined_unit(ua, ub, true)?;
                    (
                        divide_round(
                            a.checked_mul(DECIMAL_SCALE).ok_or(FormulaError::Overflow)?,
                            b,
                        )?,
                        unit,
                    )
                }
                _ => return Err(FormulaError::UnitMismatch),
            }
        }
    };
    if result.0.abs() > DECIMAL_LIMIT {
        return Err(FormulaError::Overflow);
    }
    Ok(result)
}
