//! Strict dimensional entry shared by the desktop and CLI.

use crate::{measurement::MeasurementUnit, EngineError};

/// Parse a positive distance in `unit`. A decimal point or comma uses that unit; grouping
/// separators and mixed decimal conventions are rejected. Feet/inches
/// notation (`5' 6 1/2"`, `5 ft 6 in`, `6 1/2 in`) is converted explicitly; mixed signs,
/// missing units, zero denominators, NaN and ambiguous trailing input are rejected.
pub fn parse_measurement_distance(text: &str, unit: MeasurementUnit) -> Result<f64, EngineError> {
    let invalid =
        || EngineError::Other("enter a positive distance or feet/inches such as 5' 6 1/2\"".into());
    if text.len() > 128 || !text.is_ascii() || (text.contains('.') && text.contains(',')) {
        return Err(invalid());
    }
    let text = text.trim().to_ascii_lowercase();
    fn scalar(text: &str) -> Option<f64> {
        let pieces: Vec<_> = text.split_whitespace().collect();
        if pieces.is_empty() || pieces.len() > 2 {
            return None;
        }
        let positive = |s: &str| -> Option<f64> {
            if s.is_empty()
                || s.bytes()
                    .any(|b| !b.is_ascii_digit() && b != b'.' && b != b',')
            {
                return None;
            }
            // A comma is always a decimal separator, never a thousands separator. Parsing
            // the normalized scalar also rejects repeated separators within one number.
            s.replace(',', ".")
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && *v >= 0.0)
        };
        let fraction = |s: &str| -> Option<f64> {
            let (n, d) = s.split_once('/')?;
            let (n, d) = (positive(n)?, positive(d)?);
            (d > 0.0 && n < d && n.fract() == 0.0 && d.fract() == 0.0).then_some(n / d)
        };
        match pieces.as_slice() {
            [one] => positive(one).or_else(|| fraction(one)),
            [whole, part] => {
                let whole = positive(whole)?;
                (whole.fract() == 0.0).then_some(whole + fraction(part)?)
            }
            _ => None,
        }
    }
    let normalized = text
        .replace("feet", "'")
        .replace("foot", "'")
        .replace("ft", "'")
        .replace("inches", "\"")
        .replace("inch", "\"")
        .replace("in", "\"");
    let value = if normalized.contains('\'') || normalized.contains('"') {
        let (feet, inches) = if let Some((feet, remainder)) = normalized.split_once('\'') {
            let feet = scalar(feet).ok_or_else(invalid)?;
            let remainder = remainder.trim();
            let inches = if remainder.is_empty() {
                0.0
            } else {
                scalar(remainder.strip_suffix('"').ok_or_else(invalid)?).ok_or_else(invalid)?
            };
            if inches >= 12.0 {
                return Err(invalid());
            }
            (feet, inches)
        } else {
            (
                0.0,
                scalar(normalized.trim().strip_suffix('"').ok_or_else(invalid)?)
                    .ok_or_else(invalid)?,
            )
        };
        let total_inches = feet * 12.0 + inches;
        match unit {
            MeasurementUnit::Feet => total_inches / 12.0,
            MeasurementUnit::Inches => total_inches,
            _ => total_inches * 0.0254 / unit.metres(),
        }
    } else {
        scalar(&normalized).ok_or_else(invalid)?
    };
    if !value.is_finite() || value <= 0.0 || value > 1e12 {
        return Err(invalid());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimal_comma_preserves_distance_and_explicit_unit_conversion() {
        assert_eq!(
            parse_measurement_distance("2,5", MeasurementUnit::Metres).unwrap(),
            2.5
        );
        assert_eq!(
            parse_measurement_distance("1,234", MeasurementUnit::Metres).unwrap(),
            1.234
        );
        assert_eq!(
            parse_measurement_distance("5' 6,5\"", MeasurementUnit::Inches).unwrap(),
            66.5
        );
        assert_eq!(
            parse_measurement_distance("5,5 ft 6,5 in", MeasurementUnit::Inches).unwrap(),
            72.5
        );
        assert_eq!(
            parse_measurement_distance("2,0 1/2", MeasurementUnit::Metres).unwrap(),
            2.5
        );
        assert!(
            (parse_measurement_distance("6,5 in", MeasurementUnit::Metres).unwrap() - 0.1651).abs()
                < 1e-10
        );
        for bad in [
            "2,,5",
            "1,234,567",
            "1.234,5",
            "1,234.5",
            "5.5 ft 6,5 in",
            "2,5 1/2",
            "1,5/2",
            "1/0,0",
            "3' 12,0\"",
            "0,0",
        ] {
            assert!(
                parse_measurement_distance(bad, MeasurementUnit::Metres).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn feet_inches_fractions_are_explicit_and_convert_units() {
        for value in ["5' 6 1/2\"", "5 ft 6 1/2 in", "5 feet 6 1/2 inches"] {
            assert!(
                (parse_measurement_distance(value, MeasurementUnit::Inches).unwrap() - 66.5).abs()
                    < 1e-10
            );
        }
        assert_eq!(
            parse_measurement_distance("1/2", MeasurementUnit::Metres).unwrap(),
            0.5
        );
        assert!(
            (parse_measurement_distance("12 in", MeasurementUnit::Metres).unwrap() - 0.3048).abs()
                < 1e-10
        );
        for bad in [
            "",
            "0",
            "-2",
            "+2",
            "NaN",
            "1e3",
            "3' 12\"",
            "3' 4",
            "3/0",
            "1/1",
            "1/2/3",
            "4 in trailing",
            "2 ft 1 ft",
        ] {
            assert!(
                parse_measurement_distance(bad, MeasurementUnit::Feet).is_err(),
                "{bad}"
            );
        }
    }
}
