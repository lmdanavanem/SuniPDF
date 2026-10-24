//! Read-only decimal expressions for quantity reports. No calls, assignments or I/O.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    Neg(Box<Node>),
    Binary(u8, Box<Node>, Box<Node>),
}
pub(super) struct CompiledFormula {
    column: FormulaColumn,
    node: Node,
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
        })
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
            _ => Err(bad("formula accepts decimals, quantity, [numeric fields], parentheses and + - * / only")),
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

fn evaluate(
    node: &Node,
    row: &QuantityRow,
    expected_unit: Option<QuantityUnit>,
) -> Result<(i128, Option<QuantityUnit>), FormulaError> {
    let result = match node {
        Node::Number(value) => (*value, None),
        Node::Field(name) => match row.fields.custom.get(name) {
            Some(TypedValue::Number { value }) => {
                (value.units().map_err(|_| FormulaError::Overflow)?, None)
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
                b'*' if ua.is_none() || ub.is_none() => (
                    divide_round(
                        a.checked_mul(b).ok_or(FormulaError::Overflow)?,
                        DECIMAL_SCALE,
                    )?,
                    ua.or(ub),
                ),
                b'/' if ub.is_none() || ua == ub => (
                    divide_round(
                        a.checked_mul(DECIMAL_SCALE).ok_or(FormulaError::Overflow)?,
                        b,
                    )?,
                    if ua == ub { None } else { ua },
                ),
                _ => return Err(FormulaError::UnitMismatch),
            }
        }
    };
    if result.0.abs() > DECIMAL_LIMIT {
        return Err(FormulaError::Overflow);
    }
    Ok(result)
}
