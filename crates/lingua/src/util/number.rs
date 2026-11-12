use crate::serde_json::Number;
use serde::{de::Error, Deserialize, Deserializer};

pub(crate) fn deserialize_optional_integer<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Number>::deserialize(deserializer)?
        .map(|number| {
            integer_from_number(&number)
                .ok_or_else(|| D::Error::custom("expected an integer within the i64 range"))
        })
        .transpose()
}

fn integer_from_number(number: &Number) -> Option<i64> {
    if let Some(value) = number.as_i64() {
        return Some(value);
    }
    let literal = number.to_string();
    let (mantissa, exponent) = match literal.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i64>().ok()?),
        None => (literal.as_str(), 0),
    };
    let fraction_len = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    let digits = mantissa.trim_start_matches('-').replace('.', "");
    let digits = digits.trim_start_matches('0');
    let significant = digits.trim_end_matches('0');
    if significant.is_empty() {
        return Some(0);
    }
    let zeros = exponent
        .checked_sub(i64::try_from(fraction_len).ok()?)?
        .checked_add(i64::try_from(digits.len() - significant.len()).ok()?)?;
    let zeros = usize::try_from(zeros).ok()?;
    if significant.len().checked_add(zeros)? > 19 {
        return None;
    }
    let negative_value = significant
        .bytes()
        .chain(std::iter::repeat_n(b'0', zeros))
        .try_fold(0_i64, |value, digit| {
            value.checked_mul(10)?.checked_sub(i64::from(digit - b'0'))
        })?;
    if mantissa.starts_with('-') {
        Some(negative_value)
    } else {
        negative_value.checked_neg()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json;

    #[derive(Debug, Deserialize)]
    struct Count {
        #[serde(default, deserialize_with = "deserialize_optional_integer")]
        value: Option<i64>,
    }

    #[test]
    fn integral_numbers_are_parsed_without_rounding() {
        for (literal, expected) in [
            ("null", None),
            ("0.0", Some(0)),
            ("-0.0", Some(0)),
            ("46696.0", Some(46696)),
            ("46696.000", Some(46696)),
            ("4.6696e4", Some(46696)),
            ("100.0e-2", Some(1)),
            ("-12.0", Some(-12)),
            ("9007199254740993.0", Some(9007199254740993)),
            ("9223372036854775807.0", Some(i64::MAX)),
            ("-9223372036854775808.0", Some(i64::MIN)),
        ] {
            let count: Count = serde_json::from_str(&format!("{{\"value\":{literal}}}")).unwrap();
            assert_eq!(count.value, expected, "{literal}");
        }
    }

    #[test]
    fn fractional_and_out_of_range_numbers_are_rejected() {
        for literal in [
            "1.5",
            "1.0000000000000000001",
            "1e-1",
            "9223372036854775808",
            "9223372036854775808.0",
            "-9223372036854775809.0",
            "1e100",
            "\"46696\"",
        ] {
            assert!(
                serde_json::from_str::<Count>(&format!("{{\"value\":{literal}}}")).is_err(),
                "{literal}"
            );
        }
    }
}
