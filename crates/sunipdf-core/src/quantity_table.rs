//! Snapshot-bound quantities, user-declared prices and bounded report-only expressions.

#[path = "quantity_formula.rs"]
mod formula;
pub use formula::{FormulaColumn, FormulaError, FormulaValue};
pub type FormulaValues = BTreeMap<(usize, usize), Vec<FormulaValue>>;

use crate::annot_write::{load_annot, store_annot, touch};
use crate::annotation_register::{
    self, AnnotationRegister, RegisterFilter, RegisterRow, RegisterSort,
};
use crate::measurement::{MeasurementKind, MeasurementUnit};
use crate::security::{self, lopdf_err};
use crate::{Document, EngineError};
use lopdf::{Dictionary, Object};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

const MAX_RECORD_BYTES: usize = 64 * 1024;
const MAX_METADATA_BYTES: usize = 4 * 1024 * 1024;
const DECIMAL_SCALE: i128 = 1_000_000;
const DECIMAL_LIMIT: i128 = 1_000_000_000_000 * DECIMAL_SCALE;
const KEY: &[u8] = b"SuniPDFQuantity";

fn bad(message: &str) -> EngineError {
    EngineError::Other(format!("quantity table: {message}"))
}
fn short(value: &str, max: usize) -> bool {
    value.len() <= max && !value.chars().any(char::is_control)
}

fn decimal_units(value: &str) -> Result<i128, EngineError> {
    if value.is_empty() || value.len() > 22 {
        return Err(bad("decimal exceeds supported bounds"));
    }
    let (negative, digits) = value
        .strip_prefix('-')
        .map_or((false, value), |s| (true, s));
    let (whole, frac) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || frac.len() > 6
        || !frac.bytes().all(|c| c.is_ascii_digit())
        || (digits.contains('.') && frac.is_empty())
    {
        return Err(bad(
            "decimal requires plain digits with at most six fractional digits",
        ));
    }
    let whole = whole.parse::<i128>().map_err(|_| bad("decimal overflow"))?;
    let fractional = if frac.is_empty() {
        0
    } else {
        frac.parse::<i128>().map_err(|_| bad("decimal overflow"))?
            * 10i128.pow(6 - frac.len() as u32)
    };
    let units = whole
        .checked_mul(DECIMAL_SCALE)
        .and_then(|n| n.checked_add(fractional))
        .filter(|n| *n <= DECIMAL_LIMIT)
        .ok_or_else(|| bad("decimal exceeds one trillion"))?;
    Ok(if negative { -units } else { units })
}

fn format_units(units: i128, precision: u32, trim: bool) -> String {
    let scale = 10i128.pow(precision);
    let sign = if units < 0 { "-" } else { "" };
    let units = units.abs();
    if precision == 0 {
        return format!("{sign}{units}");
    }
    let mut text = format!(
        "{sign}{}.{:0width$}",
        units / scale,
        units % scale,
        width = precision as usize
    );
    if trim {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DecimalValue(String);
impl DecimalValue {
    pub fn parse(value: &str) -> Result<Self, EngineError> {
        Ok(Self(format_units(decimal_units(value)?, 6, true)))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    fn units(&self) -> Result<i128, EngineError> {
        decimal_units(&self.0)
    }
}
impl TryFrom<String> for DecimalValue {
    type Error = EngineError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}
impl From<DecimalValue> for String {
    fn from(value: DecimalValue) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityDimension {
    Count,
    Length,
    Area,
    Angle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum QuantityUnit {
    #[serde(rename = "each")]
    Each,
    #[serde(rename = "mm")]
    Millimetres,
    #[serde(rename = "cm")]
    Centimetres,
    #[serde(rename = "m")]
    Metres,
    #[serde(rename = "in")]
    Inches,
    #[serde(rename = "ft")]
    Feet,
    #[serde(rename = "mm2")]
    SquareMillimetres,
    #[serde(rename = "cm2")]
    SquareCentimetres,
    #[serde(rename = "m2")]
    SquareMetres,
    #[serde(rename = "in2")]
    SquareInches,
    #[serde(rename = "ft2")]
    SquareFeet,
    #[serde(rename = "deg")]
    Degrees,
}
impl QuantityUnit {
    pub const ALL: [Self; 12] = [
        Self::Each,
        Self::Millimetres,
        Self::Centimetres,
        Self::Metres,
        Self::Inches,
        Self::Feet,
        Self::SquareMillimetres,
        Self::SquareCentimetres,
        Self::SquareMetres,
        Self::SquareInches,
        Self::SquareFeet,
        Self::Degrees,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Each => "each",
            Self::Millimetres => "mm",
            Self::Centimetres => "cm",
            Self::Metres => "m",
            Self::Inches => "in",
            Self::Feet => "ft",
            Self::SquareMillimetres => "mm2",
            Self::SquareCentimetres => "cm2",
            Self::SquareMetres => "m2",
            Self::SquareInches => "in2",
            Self::SquareFeet => "ft2",
            Self::Degrees => "deg",
        }
    }
    pub fn from_id(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|u| u.id() == value)
    }
    pub fn dimension(self) -> QuantityDimension {
        match self {
            Self::Each => QuantityDimension::Count,
            Self::Degrees => QuantityDimension::Angle,
            Self::SquareMillimetres
            | Self::SquareCentimetres
            | Self::SquareMetres
            | Self::SquareInches
            | Self::SquareFeet => QuantityDimension::Area,
            _ => QuantityDimension::Length,
        }
    }
    fn factor(self) -> f64 {
        match self {
            Self::Millimetres => 0.001,
            Self::Centimetres => 0.01,
            Self::Inches => 0.0254,
            Self::Feet => 0.3048,
            Self::SquareMillimetres => 0.000001,
            Self::SquareCentimetres => 0.0001,
            Self::SquareInches => 0.0254 * 0.0254,
            Self::SquareFeet => 0.3048 * 0.3048,
            _ => 1.0,
        }
    }
    fn measurement(unit: MeasurementUnit, kind: MeasurementKind) -> Self {
        match (unit, kind) {
            (_, MeasurementKind::Angle) => Self::Degrees,
            (MeasurementUnit::Millimetres, MeasurementKind::Area) => Self::SquareMillimetres,
            (MeasurementUnit::Centimetres, MeasurementKind::Area) => Self::SquareCentimetres,
            (MeasurementUnit::Metres, MeasurementKind::Area) => Self::SquareMetres,
            (MeasurementUnit::Inches, MeasurementKind::Area) => Self::SquareInches,
            (MeasurementUnit::Feet, MeasurementKind::Area) => Self::SquareFeet,
            (MeasurementUnit::Millimetres, _) => Self::Millimetres,
            (MeasurementUnit::Centimetres, _) => Self::Centimetres,
            (MeasurementUnit::Metres, _) => Self::Metres,
            (MeasurementUnit::Inches, _) => Self::Inches,
            (MeasurementUnit::Feet, _) => Self::Feet,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityValue {
    pub value: f64,
    pub unit: QuantityUnit,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitPrice {
    pub amount: DecimalValue,
    pub currency: String,
    pub per_unit: QuantityUnit,
    pub rounding_decimals: u8,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TypedValue {
    Text { value: String },
    Date { value: String },
    Number { value: DecimalValue },
    Choice { value: String, options: Vec<String> },
}
impl TypedValue {
    fn sort_key(&self) -> Option<(u8, i128, &str)> {
        Some(match self {
            Self::Number { value } => (0, value.units().ok()?, ""),
            Self::Date { value } => (1, 0, value.as_str()),
            Self::Text { value } if !value.trim().is_empty() => (2, 0, value.as_str()),
            Self::Text { .. } => return None,
            Self::Choice { value, .. } => (3, 0, value.as_str()),
        })
    }
    fn schema(&self) -> Value {
        match self {
            Self::Text { .. } => json!({"type":"text"}),
            Self::Date { .. } => json!({"type":"date"}),
            Self::Number { .. } => json!({"type":"number"}),
            Self::Choice { options, .. } => json!({"type":"choice","options":options}),
        }
    }
    fn display(&self) -> &str {
        match self {
            Self::Text { value } | Self::Date { value } | Self::Choice { value, .. } => value,
            Self::Number { value } => value.as_str(),
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityFields {
    pub unit_price: Option<UnitPrice>,
    #[serde(deserialize_with = "read_custom")]
    pub custom: BTreeMap<String, TypedValue>,
}

fn read_custom<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, TypedValue>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = BTreeMap<String, TypedValue>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("at most 16 uniquely named typed columns")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, TypedValue>()? {
                if values.len() >= 16 || values.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate or excessive typed columns",
                    ));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(Visitor)
}
impl QuantityFields {
    pub fn from_json(bytes: &[u8]) -> Result<Self, EngineError> {
        let value: Self = parse_json(bytes)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), EngineError> {
        if let Some(p) = &self.unit_price {
            if p.amount.units()? < 0
                || p.currency.len() != 3
                || !p.currency.bytes().all(|c| c.is_ascii_uppercase())
                || p.rounding_decimals > 4
                || p.per_unit == QuantityUnit::Degrees
            {
                return Err(bad(
                    "invalid nonnegative price, currency, basis unit or rounding",
                ));
            }
        }
        if self.custom.len() > 16 {
            return Err(bad("at most 16 typed fields per annotation"));
        }
        for (key, value) in &self.custom {
            if key.trim() != key || key.is_empty() || !short(key, 64) {
                return Err(bad("invalid typed column name"));
            }
            match value {
                TypedValue::Text { value } => {
                    if !short(value, 1024) {
                        return Err(bad("text field exceeds bounds"));
                    }
                }
                TypedValue::Date { value } => {
                    if value.len() != 10
                        || chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                            .ok()
                            .is_none_or(|d| d.format("%Y-%m-%d").to_string() != *value)
                    {
                        return Err(bad("date must be a real YYYY-MM-DD date"));
                    }
                }
                TypedValue::Number { value } => {
                    value.units()?;
                }
                TypedValue::Choice { value, options } => {
                    let distinct: std::collections::HashSet<_> = options.iter().collect();
                    if options.is_empty()
                        || options.len() > 32
                        || distinct.len() != options.len()
                        || !options.contains(value)
                        || options
                            .iter()
                            .any(|s| s.trim() != s || s.is_empty() || !short(s, 64))
                    {
                        return Err(bad("invalid choice value or options"));
                    }
                }
            }
        }
        Ok(())
    }
}

fn parse_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, EngineError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(bad("record exceeds 64 KiB"));
    }
    serde_json::from_slice(bytes).map_err(|e| bad(&e.to_string()))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoneyTotal {
    pub amount: String,
    pub currency: String,
    pub rounding_decimals: u8,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityRow {
    pub register: RegisterRow,
    pub quantity: Option<QuantityValue>,
    pub fields: QuantityFields,
    pub line_total: Option<MoneyTotal>,
    pub editable: bool,
}
impl QuantityRow {
    pub fn from_json(bytes: &[u8]) -> Result<Self, EngineError> {
        let row: Self = parse_json(bytes)?;
        row.fields.validate()?;
        if row.register.page == 0
            || row
                .quantity
                .as_ref()
                .is_some_and(|q| !q.value.is_finite() || q.value < 0.0)
        {
            return Err(bad("invalid expected row"));
        }
        Ok(row)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredFields {
    version: u8,
    fields: QuantityFields,
}

fn stored_fields(
    pdf: &lopdf::Document,
    dict: &Dictionary,
    budget: &mut usize,
) -> Result<QuantityFields, EngineError> {
    let Ok(raw) = dict.get(KEY) else {
        return Ok(QuantityFields::default());
    };
    let bytes = pdf
        .dereference(raw)
        .and_then(|(_, o)| o.as_str())
        .map_err(lopdf_err)?;
    *budget = budget
        .checked_add(bytes.len())
        .ok_or_else(|| bad("metadata overflow"))?;
    if *budget > MAX_METADATA_BYTES {
        return Err(bad("quantity metadata exceeds 4 MiB"));
    }
    let stored: StoredFields = parse_json(bytes)?;
    if stored.version != 1 {
        return Err(bad("unsupported quantity metadata version"));
    }
    stored.fields.validate()?;
    Ok(stored.fields)
}

fn line_total(
    quantity: Option<&QuantityValue>,
    price: Option<&UnitPrice>,
) -> Result<Option<MoneyTotal>, EngineError> {
    let Some(price) = price else {
        return Ok(None);
    };
    let quantity =
        quantity.ok_or_else(|| bad("unit price requires a count or supported measurement"))?;
    if quantity.unit.dimension() != price.per_unit.dimension()
        || quantity.unit == QuantityUnit::Degrees
    {
        return Err(bad("price basis has a different quantity dimension"));
    }
    let converted = quantity.value * quantity.unit.factor() / price.per_unit.factor();
    if !converted.is_finite() || !(0.0..=1e12).contains(&converted) {
        return Err(bad("quantity exceeds supported pricing bounds"));
    }
    // Geometry remains floating point. Pricing quantizes its explicit basis to six decimals,
    // then all multiplication and rounding use checked integer arithmetic (half away from zero).
    let quantity_units = decimal_units(&format!("{converted:.6}"))?;
    let product = quantity_units
        .checked_mul(price.amount.units()?)
        .ok_or_else(|| bad("price multiplication overflow"))?;
    let divisor = 10i128.pow(12 - u32::from(price.rounding_decimals));
    let rounded = product
        .checked_add(divisor / 2)
        .ok_or_else(|| bad("price rounding overflow"))?
        / divisor;
    Ok(Some(MoneyTotal {
        amount: format_units(rounded, u32::from(price.rounding_decimals), false),
        currency: price.currency.clone(),
        rounding_decimals: price.rounding_decimals,
    }))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QuantityGrouping {
    #[default]
    None,
    CountGroup,
    Discipline,
    BusinessStatus,
}
impl QuantityGrouping {
    pub const ALL: [Self; 4] = [
        Self::None,
        Self::CountGroup,
        Self::Discipline,
        Self::BusinessStatus,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::CountGroup => "count_group",
            Self::Discipline => "discipline",
            Self::BusinessStatus => "business_status",
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct QuantityOptions {
    pub filter: RegisterFilter,
    pub sort: RegisterSort,
    pub descending: bool,
    pub group_by: QuantityGrouping,
    /// Read-only report configuration, never stored as executable PDF metadata.
    pub formulas: Vec<FormulaColumn>,
}

impl QuantityOptions {
    fn compiled_formulas(&self) -> Result<Vec<formula::CompiledFormula>, EngineError> {
        let mut names = std::collections::HashSet::new();
        if self.formulas.len() > 8 {
            return Err(bad("at most eight formula columns"));
        }
        self.formulas
            .iter()
            .map(|column| {
                if !names.insert(&column.name) {
                    return Err(bad("duplicate formula column name"));
                }
                column.compile()
            })
            .collect()
    }
    pub fn validate_formulas(&self) -> Result<(), EngineError> {
        self.compiled_formulas().map(|_| ())
    }
    pub fn formulas_from_json(bytes: &[u8]) -> Result<Vec<FormulaColumn>, EngineError> {
        let formulas = parse_json(bytes)?;
        let options = Self {
            formulas,
            ..Self::default()
        };
        options.validate_formulas()?;
        Ok(options.formulas)
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct CostSubtotal {
    pub total: MoneyTotal,
    pub priced_rows: usize,
    pub complete: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct QuantitySubtotal {
    pub group: Option<String>,
    pub unit: QuantityUnit,
    pub quantity: f64,
    pub rows: usize,
    pub priced_rows: usize,
    pub missing_prices: usize,
    pub costs: Vec<CostSubtotal>,
}
#[derive(Debug, Clone, Serialize)]
pub struct QuantitySummary {
    pub matched_rows: usize,
    pub quantity_rows: usize,
    pub unquantified_rows: usize,
    pub groups: Vec<QuantitySubtotal>,
}
#[derive(Debug, Clone)]
pub struct QuantityExport {
    pub json: Value,
    pub csv: String,
}
#[derive(Debug, Clone)]
pub struct QuantityTable {
    register: AnnotationRegister,
    rows: Vec<QuantityRow>,
    can_export: bool,
    metadata_bytes: usize,
    metadata_sizes: HashMap<(usize, usize), usize>,
}
#[derive(Clone)]
pub struct QuantityTarget {
    source: Arc<Vec<u8>>,
    row: Arc<QuantityRow>,
}

impl std::fmt::Debug for QuantityTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QuantityTarget")
            .field("page", &self.row.register.page)
            .field("index", &self.row.register.index)
            .field("annotation_id", &self.row.register.annotation_id)
            .finish_non_exhaustive()
    }
}

impl QuantityTable {
    pub fn rows(&self) -> &[QuantityRow] {
        &self.rows
    }
    pub fn source_sha256(&self) -> &str {
        &self.register.source_sha256
    }
    pub fn page_count(&self) -> usize {
        self.register.page_count
    }
    pub fn can_export(&self) -> bool {
        self.can_export
    }
    pub fn target(&self, row: &QuantityRow) -> Result<QuantityTarget, EngineError> {
        if !row.editable || !self.rows.iter().any(|r| r == row) {
            return Err(bad("row is not an editable member of this snapshot"));
        }
        Ok(QuantityTarget {
            source: self.register.source_snapshot.clone(),
            row: Arc::new(row.clone()),
        })
    }
    pub fn select(&self, options: &QuantityOptions) -> Vec<&QuantityRow> {
        let by_key: HashMap<_, _> = self
            .rows
            .iter()
            .map(|r| ((r.register.page, r.register.index), r))
            .collect();
        let query = options.filter.query.to_lowercase();
        let ordinary: std::collections::HashSet<_> = self
            .register
            .select(&options.filter, options.sort, options.descending)
            .into_iter()
            .map(|r| (r.page, r.index))
            .collect();
        let mut filter = options.filter.clone();
        filter.query.clear();
        let mut rows: Vec<_> = self
            .register
            .select(&filter, options.sort, options.descending)
            .into_iter()
            .filter_map(|r| by_key.get(&(r.page, r.index)).copied())
            .filter(|row| {
                query.is_empty()
                    || ordinary.contains(&(row.register.page, row.register.index))
                    || row.fields.custom.iter().any(|(key, value)| {
                        key.to_lowercase().contains(&query)
                            || value.display().to_lowercase().contains(&query)
                    })
            })
            .collect();
        if let Some(name) = &options.filter.custom_sort {
            rows.sort_by(|a, b| {
                annotation_register::custom_order(
                    a.fields.custom.get(name).and_then(TypedValue::sort_key),
                    b.fields.custom.get(name).and_then(TypedValue::sort_key),
                    options.descending,
                )
                .then_with(|| {
                    (a.register.page, a.register.index).cmp(&(b.register.page, b.register.index))
                })
            });
        }
        rows
    }
    pub fn summary(&self, options: &QuantityOptions) -> Result<QuantitySummary, EngineError> {
        let rows = self.select(options);
        let mut buckets: BTreeMap<(Option<String>, QuantityUnit), Vec<&QuantityRow>> =
            BTreeMap::new();
        let quantity_rows = rows.iter().filter(|row| row.quantity.is_some()).count();
        for row in &rows {
            let Some(q) = row
                .quantity
                .as_ref()
                .filter(|q| q.unit != QuantityUnit::Degrees)
            else {
                continue;
            };
            let group = match options.group_by {
                QuantityGrouping::None => None,
                QuantityGrouping::CountGroup => row.register.count_group.clone(),
                QuantityGrouping::Discipline => row.register.fields.discipline.clone(),
                QuantityGrouping::BusinessStatus => row.register.fields.business_status.clone(),
            };
            buckets.entry((group, q.unit)).or_default().push(row);
        }
        let mut groups = Vec::new();
        for ((group, unit), bucket) in buckets {
            let mut quantity = 0.0;
            let mut prices: BTreeMap<(String, u8), (i128, usize)> = BTreeMap::new();
            let mut priced_rows = 0;
            for row in &bucket {
                quantity += row.quantity.as_ref().expect("quantity bucket").value;
                if !quantity.is_finite() {
                    return Err(bad("quantity sum overflow"));
                }
                if let Some(total) = &row.line_total {
                    let amount = money_units(total)?;
                    let entry = prices
                        .entry((total.currency.clone(), total.rounding_decimals))
                        .or_default();
                    entry.0 = entry
                        .0
                        .checked_add(amount)
                        .ok_or_else(|| bad("cost sum overflow"))?;
                    entry.1 += 1;
                    priced_rows += 1;
                }
            }
            let costs = prices
                .into_iter()
                .map(
                    |((currency, rounding_decimals), (amount, n))| CostSubtotal {
                        total: MoneyTotal {
                            amount: format_units(amount, u32::from(rounding_decimals), false),
                            currency,
                            rounding_decimals,
                        },
                        priced_rows: n,
                        complete: n == bucket.len(),
                    },
                )
                .collect();
            groups.push(QuantitySubtotal {
                group,
                unit,
                quantity,
                rows: bucket.len(),
                priced_rows,
                missing_prices: bucket.len() - priced_rows,
                costs,
            });
        }
        Ok(QuantitySummary {
            matched_rows: rows.len(),
            quantity_rows,
            unquantified_rows: rows.len() - quantity_rows,
            groups,
        })
    }
    /// Computes once per report/filter revision; result rows keep their source identities.
    pub fn formula_values(&self, options: &QuantityOptions) -> Result<FormulaValues, EngineError> {
        let compiled = options.compiled_formulas()?;
        if compiled.is_empty() {
            return Ok(BTreeMap::new());
        }
        Ok(self
            .select(options)
            .into_iter()
            .map(|row| {
                (
                    (row.register.page, row.register.index),
                    compiled
                        .iter()
                        .map(|formula| formula.evaluate(row))
                        .collect(),
                )
            })
            .collect())
    }
    pub fn json(&self, options: &QuantityOptions) -> Result<Value, EngineError> {
        if !self.can_export {
            return Err(EngineError::SecurityRestricted);
        }
        let rows = self.select(options);
        let compiled = options.compiled_formulas()?;
        let formula_rows: Vec<_> = rows
            .iter()
            .filter(|_| !compiled.is_empty())
            .map(|row| {
                json!({
                    "page":row.register.page, "index":row.register.index,
                    "values":options.formulas.iter().zip(&compiled).map(|(column, compiled)|
                        (column.name.clone(), compiled.evaluate(row))).collect::<BTreeMap<_, _>>()
                })
            })
            .collect();
        let formula_complete = formula_rows.iter().all(|row| {
            row["values"]
                .as_object()
                .is_some_and(|values| values.values().all(|value| value["error"].is_null()))
        });
        Ok(
            json!({"schema":"sunipdf-quantity-table","version":1,"complete":true,"source_sha256":self.source_sha256(),"page_count":self.page_count(),"filter":options.filter,"sort":options.sort.id(),"descending":options.descending,"group_by":options.group_by.id(),"pricing_quantity_decimals":6,"money_rounding":"half_away_from_zero_per_line","unit_conversion":"price_basis_only","rows":rows,"summary":self.summary(options)?,"formulas":options.formulas,"formula_arithmetic":"decimal_6_places_half_away_from_zero_each_operation","formula_evaluation_complete":formula_complete,"formula_rows":formula_rows}),
        )
    }
    pub fn csv(&self, options: &QuantityOptions) -> Result<String, EngineError> {
        if !self.can_export {
            return Err(EngineError::SecurityRestricted);
        }
        let rows = self.select(options);
        let compiled = options.compiled_formulas()?;
        let keys: std::collections::BTreeSet<_> = rows
            .iter()
            .flat_map(|r| r.fields.custom.keys().cloned())
            .collect();
        let mut header: Vec<String> = [
            "record_type",
            "source_sha256",
            "page",
            "index",
            "annotation_id",
            "kind",
            "count_group",
            "discipline",
            "business_status",
            "quantity",
            "unit",
            "unit_price",
            "price_unit",
            "currency",
            "rounding_decimals",
            "line_total",
            "group",
            "rows",
            "priced_rows",
            "missing_prices",
            "subtotal_complete",
            "export_options",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        for key in &keys {
            header.push(format!("custom:{key}:type"));
            header.push(format!("custom:{key}:value"));
        }
        for column in &options.formulas {
            header.extend([
                format!("formula:{}:value", column.name),
                format!("formula:{}:unit", column.name),
                format!("formula:{}:error", column.name),
            ]);
        }
        let width = header.len();
        let line = |mut values: Vec<String>| {
            values.resize(width, String::new());
            values
                .iter()
                .map(|s| annotation_register::csv_cell(s))
                .collect::<Vec<_>>()
                .join(",")
                + "\r\n"
        };
        let mut out = line(header);
        let mut formula_complete = true;
        for row in rows {
            let r = &row.register;
            let p = row.fields.unit_price.as_ref();
            let mut values = vec![
                "row".into(),
                self.source_sha256().into(),
                r.page.to_string(),
                r.index.to_string(),
                r.annotation_id.clone().unwrap_or_default(),
                r.kind.clone(),
                r.count_group.clone().unwrap_or_default(),
                r.fields.discipline.clone().unwrap_or_default(),
                r.fields.business_status.clone().unwrap_or_default(),
                row.quantity
                    .as_ref()
                    .map(|q| q.value.to_string())
                    .unwrap_or_default(),
                row.quantity
                    .as_ref()
                    .map(|q| q.unit.id().into())
                    .unwrap_or_default(),
                p.map(|p| p.amount.as_str().into()).unwrap_or_default(),
                p.map(|p| p.per_unit.id().into()).unwrap_or_default(),
                p.map(|p| p.currency.clone()).unwrap_or_default(),
                p.map(|p| p.rounding_decimals.to_string())
                    .unwrap_or_default(),
                row.line_total
                    .as_ref()
                    .map(|v| v.amount.clone())
                    .unwrap_or_default(),
            ];
            values.resize(22, String::new());
            for key in &keys {
                if let Some(v) = row.fields.custom.get(key) {
                    values.push(v.schema()["type"].as_str().unwrap().into());
                    values.push(v.display().into());
                } else {
                    values.extend([String::new(), String::new()]);
                }
            }
            for compiled in &compiled {
                let value = compiled.evaluate(row);
                formula_complete &= value.error.is_none();
                values.extend([
                    value.value.unwrap_or_default(),
                    value
                        .unit
                        .map(|unit| unit.id().to_owned())
                        .unwrap_or_default(),
                    value
                        .error
                        .map(|error| error.id().to_owned())
                        .unwrap_or_default(),
                ]);
            }
            out.push_str(&line(values));
        }
        for group in self.summary(options)?.groups {
            let mut values = vec![String::new(); 22];
            values[0] = "quantity_subtotal".into();
            values[1] = self.source_sha256().into();
            values[9] = group.quantity.to_string();
            values[10] = group.unit.id().into();
            values[16] = group.group.clone().unwrap_or_default();
            values[17] = group.rows.to_string();
            values[18] = group.priced_rows.to_string();
            values[19] = group.missing_prices.to_string();
            out.push_str(&line(values));
            for cost in group.costs {
                let mut values = vec![String::new(); 22];
                values[0] = "cost_subtotal".into();
                values[1] = self.source_sha256().into();
                values[10] = group.unit.id().into();
                values[13] = cost.total.currency;
                values[14] = cost.total.rounding_decimals.to_string();
                values[15] = cost.total.amount;
                values[16] = group.group.clone().unwrap_or_default();
                values[18] = cost.priced_rows.to_string();
                values[20] = cost.complete.to_string();
                out.push_str(&line(values));
            }
        }
        let mut metadata = vec![String::new(); 22];
        metadata[0] = "metadata".into();
        metadata[1] = self.source_sha256().into();
        metadata[21]=json!({"version":1,"filter":options.filter,"sort":options.sort.id(),"descending":options.descending,"group_by":options.group_by.id(),"pricing_quantity_decimals":6,"money_rounding":"half_away_from_zero_per_line","unit_conversion":"price_basis_only","formulas":options.formulas,"formula_arithmetic":"decimal_6_places_half_away_from_zero_each_operation","formula_evaluation_complete":formula_complete}).to_string();
        out.push_str(&line(metadata));
        Ok(out)
    }
}

fn money_units(total: &MoneyTotal) -> Result<i128, EngineError> {
    let digits = total.amount.replace('.', "");
    digits
        .parse::<i128>()
        .map_err(|_| bad("invalid derived cost"))
}

impl Document<'_> {
    pub fn quantity_table(&self) -> Result<QuantityTable, EngineError> {
        if self.page_count() > 1000 {
            return Err(bad("at most 1000 pages are supported"));
        }
        let bytes = self.snapshot()?;
        if bytes.len() > 128 * 1024 * 1024 {
            return Err(bad("document exceeds 128 MiB"));
        }
        let pdf = security::load(&bytes, self.password())?;
        let register = self.annotation_register_from_pdf(bytes, &pdf)?;
        let mut measurements = HashMap::new();
        for page in 0..self.page_count() {
            for record in self.measurement_records_from_pdf(page, &pdf)? {
                measurements.insert((page + 1, record.index), record);
            }
        }
        let mut names = HashMap::new();
        for r in &register.rows {
            if let Some(id) = &r.annotation_id {
                *names.entry(id.clone()).or_insert(0usize) += 1;
            }
        }
        let pages = pdf.get_pages();
        let mut budget = 0;
        let mut schemas = BTreeMap::new();
        let mut rows = Vec::with_capacity(register.rows.len());
        let mut metadata_sizes = HashMap::new();
        for r in &register.rows {
            let page_id = pages[&(r.page as u32)];
            let (_, dict) = load_annot(&pdf, page_id, r.index)?;
            let before = budget;
            let fields = stored_fields(&pdf, &dict, &mut budget)?;
            metadata_sizes.insert((r.page, r.index), budget - before);
            for (key, value) in &fields.custom {
                if schemas
                    .insert(key.clone(), value.schema())
                    .is_some_and(|previous| previous != value.schema())
                {
                    return Err(bad("conflicting definitions for a typed custom column"));
                }
            }
            if schemas.len() > 16 {
                return Err(bad("at most 16 typed columns per document"));
            }
            let measured = measurements.get(&(r.page, r.index));
            if measured.is_some() && r.count_group.is_some() {
                return Err(bad("annotation cannot be both a count and a measurement"));
            }
            let quantity = if r.is_reply {
                None
            } else if r.count_group.is_some() {
                Some(QuantityValue {
                    value: 1.0,
                    unit: QuantityUnit::Each,
                })
            } else if let Some(record) = measured {
                if record.annotation_id != r.annotation_id {
                    return Err(bad("measurement identity differs from register"));
                }
                Some(QuantityValue {
                    value: record.measurement.value()?,
                    unit: QuantityUnit::measurement(
                        record.measurement.unit,
                        record.measurement.kind,
                    ),
                })
            } else {
                None
            };
            if quantity
                .as_ref()
                .is_some_and(|q| !q.value.is_finite() || q.value < 0.0)
            {
                return Err(bad("quantity is not finite and nonnegative"));
            }
            let unique = r
                .annotation_id
                .as_ref()
                .is_some_and(|id| !id.is_empty() && id.len() <= 256 && names.get(id) == Some(&1));
            if quantity.is_some() && !unique {
                return Err(bad("quantity annotation identity is missing or ambiguous"));
            }
            let total = line_total(quantity.as_ref(), fields.unit_price.as_ref())?;
            let editable = self.properties().can_annotate
                && unique
                && !r.is_reply
                && annotation_register::editable(&dict).is_ok();
            rows.push(QuantityRow {
                register: r.clone(),
                quantity,
                fields,
                line_total: total,
                editable,
            });
        }
        Ok(QuantityTable {
            register,
            rows,
            can_export: self.properties().can_copy,
            metadata_bytes: budget,
            metadata_sizes,
        })
    }

    pub fn quantity_target(
        &self,
        page: usize,
        expected: &QuantityRow,
    ) -> Result<QuantityTarget, EngineError> {
        if expected.register.page != page.checked_add(1).ok_or_else(|| bad("invalid page"))? {
            return Err(bad("expected row belongs to another page"));
        }
        self.quantity_table()?.target(expected)
    }

    pub fn export_quantity_table(
        &self,
        table: &QuantityTable,
        options: &QuantityOptions,
    ) -> Result<QuantityExport, EngineError> {
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        if !crate::annot_rich::same_document(
            &table.register.source_snapshot,
            &self.snapshot()?,
            self.password(),
        )? {
            return Err(bad("table is stale; refresh before exporting"));
        }
        Ok(QuantityExport {
            json: table.json(options)?,
            csv: table.csv(options)?,
        })
    }

    pub fn set_quantity_fields(
        &mut self,
        target: &QuantityTarget,
        fields: &QuantityFields,
    ) -> Result<(), EngineError> {
        self.change_quantity_fields(target, Some(fields))
    }
    pub fn clear_quantity_fields(&mut self, target: &QuantityTarget) -> Result<(), EngineError> {
        self.change_quantity_fields(target, None)
    }

    fn change_quantity_fields(
        &mut self,
        target: &QuantityTarget,
        fields: Option<&QuantityFields>,
    ) -> Result<(), EngineError> {
        if !self.properties().can_annotate {
            return Err(EngineError::SecurityRestricted);
        }
        if !crate::annot_rich::same_document(&target.source, &self.snapshot()?, self.password())? {
            return Err(bad("table is stale; refresh before editing"));
        }
        let current = self.quantity_table()?;
        let row = current
            .rows
            .iter()
            .find(|row| {
                row.register.page == target.row.register.page
                    && row.register.index == target.row.register.index
            })
            .ok_or_else(|| bad("annotation not found"))?;
        if row != target.row.as_ref() || !row.editable {
            return Err(bad("expected annotation changed or is not editable"));
        }
        if let Some(fields) = fields {
            fields.validate()?;
            line_total(row.quantity.as_ref(), fields.unit_price.as_ref())?;
            let mut keys = std::collections::HashSet::new();
            for other in &current.rows {
                if other.register.page == row.register.page
                    && other.register.index == row.register.index
                {
                    continue;
                }
                for (key, value) in &fields.custom {
                    if other
                        .fields
                        .custom
                        .get(key)
                        .is_some_and(|other| other.schema() != value.schema())
                    {
                        return Err(bad("conflicting definitions for a typed custom column"));
                    }
                }
            }
            for other in &current.rows {
                if other.register.page != row.register.page
                    || other.register.index != row.register.index
                {
                    keys.extend(other.fields.custom.keys());
                }
            }
            keys.extend(fields.custom.keys());
            if keys.len() > 16 {
                return Err(bad("at most 16 typed columns per document"));
            }
        }
        let encoded = fields
            .map(|fields| {
                serde_json::to_vec(&StoredFields {
                    version: 1,
                    fields: fields.clone(),
                })
                .map_err(|e| bad(&e.to_string()))
            })
            .transpose()?;
        if encoded
            .as_ref()
            .is_some_and(|bytes| bytes.len() > MAX_RECORD_BYTES)
        {
            return Err(bad("record exceeds 64 KiB"));
        }
        if current.metadata_bytes - current.metadata_sizes[&(row.register.page, row.register.index)]
            + encoded.as_ref().map_or(0, Vec::len)
            > MAX_METADATA_BYTES
        {
            return Err(bad("quantity metadata exceeds 4 MiB"));
        }
        let page = row.register.page - 1;
        let index = row.register.index;
        let expected_id = row.register.annotation_id.clone();
        self.edit_annotation_pdf(page, |pdf, page, _| {
            let (slot, mut dict) = load_annot(pdf, page, index)?;
            annotation_register::editable(&dict)?;
            if crate::annot_write::text_of(pdf, &dict, b"NM") != expected_id {
                return Err(bad("annotation identity changed"));
            }
            stored_fields(pdf, &dict, &mut 0)?;
            if let Some(bytes) = encoded {
                dict.set("SuniPDFQuantity", Object::string_literal(bytes));
            } else {
                dict.remove(KEY);
            }
            touch(&mut dict);
            store_annot(pdf, page, slot, dict)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn price(amount: &str, unit: QuantityUnit) -> UnitPrice {
        UnitPrice {
            amount: DecimalValue::parse(amount).unwrap(),
            currency: "CHF".into(),
            per_unit: unit,
            rounding_decimals: 2,
        }
    }

    #[test]
    fn quantity_table_decimal_pricing_converts_basis_and_rounds_per_line() {
        for bad in [
            "",
            "NaN",
            "1e2",
            "+1",
            "1.",
            " 1",
            "1.1234567",
            "1000000000001",
        ] {
            assert!(DecimalValue::parse(bad).is_err(), "{bad}");
        }
        assert_eq!(DecimalValue::parse("001.230000").unwrap().as_str(), "1.23");
        assert_eq!(DecimalValue::parse("-0.000000").unwrap().as_str(), "0");
        for (value, unit) in [
            (10.0, QuantityUnit::Metres),
            (1000.0, QuantityUnit::Centimetres),
            (10.0 / 0.3048, QuantityUnit::Feet),
        ] {
            assert_eq!(
                line_total(
                    Some(&QuantityValue { value, unit }),
                    Some(&price("2.5", QuantityUnit::Metres))
                )
                .unwrap()
                .unwrap()
                .amount,
                "25.00"
            );
        }
        assert_eq!(
            line_total(
                Some(&QuantityValue {
                    value: 10000.0,
                    unit: QuantityUnit::SquareCentimetres
                }),
                Some(&price("2.345", QuantityUnit::SquareMetres))
            )
            .unwrap()
            .unwrap()
            .amount,
            "2.35"
        );
        assert_eq!(
            line_total(
                Some(&QuantityValue {
                    value: 1.0,
                    unit: QuantityUnit::Each
                }),
                Some(&price("0", QuantityUnit::Each))
            )
            .unwrap()
            .unwrap()
            .amount,
            "0.00"
        );
        assert!(line_total(None, Some(&price("1", QuantityUnit::Each))).is_err());
        assert!(line_total(
            Some(&QuantityValue {
                value: 2.0,
                unit: QuantityUnit::Metres
            }),
            Some(&price("1", QuantityUnit::SquareMetres))
        )
        .is_err());
        assert!(line_total(
            Some(&QuantityValue {
                value: f64::INFINITY,
                unit: QuantityUnit::Each
            }),
            Some(&price("1", QuantityUnit::Each))
        )
        .is_err());
        assert!(line_total(
            Some(&QuantityValue {
                value: 2.0,
                unit: QuantityUnit::Each
            }),
            None
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn quantity_table_typed_import_rejects_duplicates_invalid_dates_and_formulas() {
        let valid=br#"{"unit_price":null,"custom":{"Date":{"type":"date","value":"2028-02-29"},"Number":{"type":"number","value":"0"},"Choice":{"type":"choice","value":"A","options":["A","B"]},"Text":{"type":"text","value":""}}}"#;
        let fields = QuantityFields::from_json(valid).unwrap();
        assert_eq!(fields.custom.len(), 4);
        for raw in [
            r#"{"unit_price":null,"custom":{"A":{"type":"text","value":"x"},"A":{"type":"number","value":"1"}}}"#,
            r#"{"unit_price":null,"custom":{"A":{"type":"formula","value":"1+1"}}}"#,
            r#"{"unit_price":null,"custom":{"A":{"type":"date","value":"2027-02-29"}}}"#,
            r#"{"unit_price":null,"custom":{"A":{"type":"choice","value":"X","options":["A"]}}}"#,
            r#"{"unit_price":null,"custom":{"A":{"type":"choice","value":"A","options":["A","A"]}}}"#,
            r#"{"unit_price":null,"custom":{},"script":"run"}"#,
        ] {
            assert!(QuantityFields::from_json(raw.as_bytes()).is_err(), "{raw}");
        }
        let mut negative = QuantityFields {
            unit_price: Some(price("-1", QuantityUnit::Each)),
            ..Default::default()
        };
        assert!(negative.validate().is_err());
        negative.unit_price.as_mut().unwrap().amount = DecimalValue::parse("0").unwrap();
        negative.validate().unwrap();
    }

    fn row(index: usize, unit: QuantityUnit, value: f64, priced: Option<UnitPrice>) -> QuantityRow {
        let register = RegisterRow {
            page: 1,
            index,
            annotation_id: Some(format!("test-{index}")),
            kind: "Stamp".into(),
            author: None,
            modified: None,
            contents: None,
            subject: None,
            rect: None,
            parent: None,
            is_reply: false,
            hidden: false,
            locked: false,
            review_status: None,
            fields: Default::default(),
            count_group: Some("=unsafe".into()),
        };
        let quantity = Some(QuantityValue { value, unit });
        let line_total = line_total(quantity.as_ref(), priced.as_ref()).unwrap();
        QuantityRow {
            register,
            quantity,
            fields: QuantityFields {
                unit_price: priced,
                ..Default::default()
            },
            line_total,
            editable: true,
        }
    }

    fn test_table(rows: Vec<QuantityRow>) -> QuantityTable {
        QuantityTable {
            register: AnnotationRegister {
                source_sha256: "revision-a".into(),
                page_count: 1,
                rows: rows.iter().map(|row| row.register.clone()).collect(),
                source_snapshot: Arc::new(Vec::new()),
            },
            rows,
            can_export: true,
            metadata_bytes: 0,
            metadata_sizes: HashMap::new(),
        }
    }
    fn formula(expression: &str) -> FormulaColumn {
        FormulaColumn {
            name: "Result".into(),
            expression: expression.into(),
            quantity_unit: Some(QuantityUnit::Metres),
            rounding_decimals: 2,
        }
    }

    #[test]
    fn custom_numeric_sort_is_numeric_stable_and_missing_last_in_both_directions() {
        let mut rows: Vec<_> = (0..6)
            .map(|index| row(index, QuantityUnit::Each, 1.0, None))
            .collect();
        for (row, number) in rows.iter_mut().zip([
            Some("10"),
            Some("2"),
            None,
            Some("-1"),
            Some("2.00"),
            Some("0"),
        ]) {
            if let Some(number) = number {
                row.fields.custom.insert(
                    "Amount".into(),
                    TypedValue::Number {
                        value: DecimalValue::parse(number).unwrap(),
                    },
                );
            }
        }
        let table = test_table(rows);
        let mut options = QuantityOptions::default();
        options.filter.custom_sort = Some("Amount".into());
        let indices = |options: &QuantityOptions| {
            table
                .select(options)
                .iter()
                .map(|row| row.register.index)
                .collect::<Vec<_>>()
        };
        assert_eq!(indices(&options), [3, 5, 1, 4, 0, 2]);
        options.descending = true;
        assert_eq!(indices(&options), [0, 1, 4, 5, 3, 2]);
        options.filter.custom_sort = Some("Absent".into());
        assert_eq!(indices(&options), [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn formulas_use_decimal_arithmetic_and_preserve_quantity_units() {
        let mut source = row(0, QuantityUnit::Metres, 12.0, None);
        source.fields.custom.insert(
            "Waste %".into(),
            TypedValue::Number {
                value: DecimalValue::parse("10").unwrap(),
            },
        );
        let value = formula("quantity * (1 + [Waste %] / 100)")
            .evaluate(&source)
            .unwrap();
        assert_eq!(value.value.as_deref(), Some("13.20"));
        assert_eq!(value.unit, Some(QuantityUnit::Metres));
        assert_eq!(value.error, None);
        for (expression, expected) in [
            ("0.1 + 0.2", "0.30"),
            ("-1.005", "-1.01"),
            ("1 / 3", "0.33"),
            ("quantity / quantity", "1.00"),
        ] {
            let value = formula(expression).evaluate(&source).unwrap();
            assert_eq!(value.value.as_deref(), Some(expected), "{expression}");
            assert_eq!(value.unit, None);
        }
        let mut fixed = formula("1 / 6");
        fixed.rounding_decimals = 6;
        assert_eq!(
            fixed.evaluate(&source).unwrap().value.as_deref(),
            Some("0.166667")
        );
    }

    #[test]
    fn custom_text_date_and_invalid_number_order_is_deterministic() {
        let mut rows: Vec<_> = (0..5)
            .map(|index| row(index, QuantityUnit::Each, 1.0, None))
            .collect();
        for (row, value) in rows
            .iter_mut()
            .zip([Some("B"), Some("A"), None, Some(" "), Some("A")])
        {
            if let Some(value) = value {
                row.register
                    .fields
                    .custom
                    .insert("Room".into(), value.into());
                row.fields.custom.insert(
                    "Room".into(),
                    TypedValue::Text {
                        value: value.into(),
                    },
                );
            }
        }
        let mut table = test_table(rows);
        let mut options = QuantityOptions::default();
        options.filter.custom_sort = Some("Room".into());
        for (descending, expected) in [(false, vec![1, 4, 0, 2, 3]), (true, vec![0, 1, 4, 2, 3])] {
            options.descending = descending;
            assert_eq!(
                table
                    .select(&options)
                    .iter()
                    .map(|row| row.register.index)
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(
                table
                    .register
                    .select(&options.filter, RegisterSort::Page, descending)
                    .iter()
                    .map(|row| row.index)
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(
                table
                    .register
                    .json(&options.filter, RegisterSort::Page, descending)["rows"][0]["index"],
                expected[0]
            );
        }
        for (row, value) in table.rows.iter_mut().zip([
            "2028-02-29",
            "2027-12-31",
            "2028-01-01",
            "2028-03-01",
            "2028-01-01",
        ]) {
            row.fields.custom.insert(
                "Date".into(),
                TypedValue::Date {
                    value: value.into(),
                },
            );
        }
        options.filter.custom_sort = Some("Date".into());
        options.descending = false;
        assert_eq!(
            table
                .select(&options)
                .iter()
                .map(|row| row.register.index)
                .collect::<Vec<_>>(),
            [1, 2, 4, 0, 3]
        );
        table.rows[0].fields.custom.insert(
            "Number".into(),
            TypedValue::Number {
                value: DecimalValue("invalid".into()),
            },
        );
        table.rows[1].fields.custom.insert(
            "Number".into(),
            TypedValue::Number {
                value: DecimalValue::parse("0").unwrap(),
            },
        );
        options.filter.custom_sort = Some("Number".into());
        for descending in [false, true] {
            options.descending = descending;
            assert_eq!(
                table.select(&options)[0].register.index,
                1,
                "invalid numbers must never become zero"
            );
        }
    }

    #[test]
    fn formula_errors_are_explicit_and_never_substitute_zero() {
        let mut source = row(0, QuantityUnit::Metres, 2.0, None);
        source
            .fields
            .custom
            .insert("Text".into(), TypedValue::Text { value: "3".into() });
        for (expression, error) in [
            ("[Absent]", FormulaError::MissingField),
            ("[Text]", FormulaError::NonNumericField),
            ("1/0", FormulaError::DivisionByZero),
            ("quantity + 1", FormulaError::UnitMismatch),
            ("quantity * quantity", FormulaError::UnitMismatch),
            ("1 / quantity", FormulaError::UnitMismatch),
            ("1000000000000 * 2", FormulaError::Overflow),
        ] {
            let value = formula(expression).evaluate(&source).unwrap();
            assert_eq!(value.error, Some(error), "{expression}");
            assert!(value.value.is_none());
        }
        source.quantity.as_mut().unwrap().unit = QuantityUnit::Centimetres;
        assert_eq!(
            formula("quantity").evaluate(&source).unwrap().error,
            Some(FormulaError::UnitMismatch)
        );
        source.quantity = None;
        assert_eq!(
            formula("quantity").evaluate(&source).unwrap().error,
            Some(FormulaError::MissingQuantity)
        );
    }

    #[test]
    fn formula_grammar_and_configuration_reject_code_excessive_depth_and_duplicate_names() {
        for expression in [
            "=1+1",
            "sum(1)",
            "open('/file')",
            "quantity.name",
            "1;2",
            "1e3",
            "1.0000001",
            "1 +",
            "[]",
            "[x",
            "1 2",
            "(1",
        ] {
            assert!(formula(expression).validate().is_err(), "{expression}");
        }
        assert!(formula(&format!("{}1{}", "(".repeat(40), ")".repeat(40)))
            .validate()
            .is_err());
        assert!(formula(&["1"; 70].join("+")).validate().is_err());
        assert!(formula(&"1".repeat(513)).validate().is_err());
        let mut no_unit = formula("quantity");
        no_unit.quantity_unit = None;
        assert!(no_unit.validate().is_err());
        let mut options = QuantityOptions {
            formulas: vec![formula("1"), formula("2")],
            ..Default::default()
        };
        assert!(options.validate_formulas().is_err());
        options.formulas = (0..9)
            .map(|index| {
                let mut f = formula("1");
                f.name = index.to_string();
                f
            })
            .collect();
        assert!(options.validate_formulas().is_err());
        assert!(QuantityOptions::formulas_from_json(br#"[{"name":"Bad","expression":"1","quantity_unit":null,"rounding_decimals":2,"script":"x"}]"#).is_err());
        let config = vec![formula("quantity * 2")];
        assert_eq!(
            QuantityOptions::formulas_from_json(&serde_json::to_vec(&config).unwrap()).unwrap(),
            config
        );
    }

    #[test]
    fn formula_exports_keep_definitions_source_rows_errors_and_spreadsheet_escaping() {
        let table = test_table(vec![
            row(0, QuantityUnit::Metres, 2.0, None),
            row(1, QuantityUnit::Each, 1.0, None),
        ]);
        let mut column = formula("quantity * 1.1");
        column.name = "=Result".into();
        let options = QuantityOptions {
            formulas: vec![column],
            ..Default::default()
        };
        let json = table.json(&options).unwrap();
        assert_eq!(json["source_sha256"], "revision-a");
        assert_eq!(json["formula_evaluation_complete"], false);
        assert_eq!(json["formulas"][0]["expression"], "quantity * 1.1");
        assert_eq!(
            json["formula_rows"][0]["values"]["=Result"]["value"],
            "2.20"
        );
        assert_eq!(
            json["formula_rows"][1]["values"]["=Result"]["error"],
            "unit_mismatch"
        );
        assert!(json["formula_rows"][1]["values"]["=Result"]["value"].is_null());
        assert_eq!(json["rows"][0]["quantity"]["value"], 2.0);
        let csv = table.csv(&options).unwrap();
        assert!(csv.contains("formula:=Result:value"));
        assert!(csv.contains("unit_mismatch"));
        assert!(csv.contains("2.20"));
        let before = table.formula_values(&options).unwrap();
        let changed = test_table(vec![row(0, QuantityUnit::Metres, 3.0, None)]);
        assert_ne!(
            before[&(1, 0)],
            changed.formula_values(&options).unwrap()[&(1, 0)]
        );
        let mut denied = table;
        denied.can_export = false;
        assert!(matches!(
            denied.json(&options),
            Err(EngineError::SecurityRestricted)
        ));
        assert!(matches!(
            denied.csv(&options),
            Err(EngineError::SecurityRestricted)
        ));
    }

    #[test]
    fn quantity_table_summary_separates_units_currencies_and_missing_prices() {
        let mut usd = price("3", QuantityUnit::Each);
        usd.currency = "USD".into();
        let mut rows = vec![
            row(
                0,
                QuantityUnit::Each,
                1.0,
                Some(price("0", QuantityUnit::Each)),
            ),
            row(1, QuantityUnit::Each, 1.0, None),
            row(2, QuantityUnit::Each, 1.0, Some(usd)),
            row(3, QuantityUnit::Metres, 1.0, None),
            row(4, QuantityUnit::Centimetres, 100.0, None),
            row(5, QuantityUnit::Degrees, 90.0, None),
        ];
        rows[0].fields.custom.insert(
            "Room".into(),
            TypedValue::Text {
                value: "=external()".into(),
            },
        );
        let table = QuantityTable {
            register: AnnotationRegister {
                source_sha256: "test-source".into(),
                page_count: 1,
                rows: rows.iter().map(|r| r.register.clone()).collect(),
                source_snapshot: Arc::new(Vec::new()),
            },
            rows,
            can_export: true,
            metadata_bytes: 0,
            metadata_sizes: HashMap::new(),
        };
        let options = QuantityOptions::default();
        let summary = table.summary(&options).unwrap();
        assert_eq!(summary.groups.len(), 3);
        assert_eq!(summary.unquantified_rows, 0);
        let count = &summary.groups[0];
        assert_eq!(count.rows, 3);
        assert_eq!(count.missing_prices, 1);
        assert_eq!(count.costs.len(), 2);
        assert!(count.costs.iter().all(|c| !c.complete));
        let csv = table.csv(&options).unwrap();
        assert!(csv.contains("'\u{3d}external()"));
        assert!(csv.contains("quantity_subtotal"));
        assert!(csv.contains("cost_subtotal"));
        assert!(csv.contains("metadata"));
        let mut filtered = options.clone();
        filtered.filter.query = "external".into();
        assert_eq!(table.select(&filtered).len(), 1);
        assert_eq!(table.json(&options).unwrap(), table.json(&options).unwrap());
        let mut denied = table;
        denied.can_export = false;
        assert!(matches!(
            denied.json(&options),
            Err(EngineError::SecurityRestricted)
        ));
        assert!(denied.summary(&options).is_ok());
    }
}
