use super::{ForecastTableMapping, ForecastTimestampFormat};
use crate::inspection::auto_training_tables::{TemporalUnit, epoch_milliseconds};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use flow_like_types::{Result, Value, anyhow, bail};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EpochUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl EpochUnit {
    fn decimal_shift(self) -> i32 {
        match self {
            Self::Seconds => 3,
            Self::Milliseconds => 0,
            Self::Microseconds => -3,
            Self::Nanoseconds => -6,
        }
    }
}

// Keep decimal epoch conversion exact, including integer nanoseconds beyond f64 precision.
struct Decimal {
    coefficient: i128,
    exponent: i32,
}

impl Decimal {
    fn parse(text: &str) -> Result<Self> {
        if text.len() > 128 {
            bail!("Numeric timestamp exceeds the supported precision");
        }
        let (mantissa, exponent) = match text.split_once(['e', 'E']) {
            Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>()?),
            None => (text, 0),
        };
        let negative = mantissa.starts_with('-');
        let mantissa = mantissa.strip_prefix(['-', '+']).unwrap_or(mantissa);
        let mut coefficient = 0_i128;
        let mut decimals = None;
        let mut digits = 0;
        for byte in mantissa.bytes() {
            if byte == b'.' && decimals.is_none() {
                decimals = Some(0);
            } else if byte.is_ascii_digit() {
                coefficient = coefficient
                    .checked_mul(10)
                    .and_then(|value| value.checked_add((byte - b'0') as i128))
                    .ok_or_else(|| anyhow!("Numeric timestamp exceeds the supported precision"))?;
                digits += 1;
                if let Some(decimals) = &mut decimals {
                    *decimals += 1;
                }
            } else {
                bail!("Timestamp must be a finite decimal or scientific-notation number");
            }
        }
        if digits == 0 {
            bail!("Timestamp must be a finite number");
        }
        let exponent = exponent
            .checked_sub(decimals.unwrap_or(0))
            .filter(|value| value.unsigned_abs() <= 38)
            .ok_or_else(|| anyhow!("Numeric timestamp exceeds the supported precision"))?;
        Ok(Self {
            coefficient: if negative { -coefficient } else { coefficient },
            exponent,
        })
    }

    fn milliseconds(&self, unit: EpochUnit) -> Result<i64> {
        let shift = self.exponent + unit.decimal_shift();
        let value = if shift >= 0 {
            10_i128
                .checked_pow(shift as u32)
                .and_then(|scale| self.coefficient.checked_mul(scale))
                .ok_or_else(|| anyhow!("Timestamp overflows epoch milliseconds"))?
        } else {
            let scale = 10_i128
                .checked_pow((-shift) as u32)
                .ok_or_else(|| anyhow!("Timestamp exceeds whole-millisecond precision"))?;
            if self.coefficient % scale != 0 {
                bail!("Forecast timestamps must have whole-millisecond precision");
            }
            self.coefficient / scale
        };
        i64::try_from(value).map_err(|_| anyhow!("Timestamp overflows epoch milliseconds"))
    }

    fn inferred_unit(&self) -> Result<EpochUnit> {
        let magnitude = (self.coefficient as f64 * 10_f64.powi(self.exponent)).abs();
        // Contemporary Unix dates have clearly separated decimal magnitudes.
        // Short epochs near 1970 require an explicit unit instead of guessing.
        match magnitude {
            value if (1e8..1e11).contains(&value) => Ok(EpochUnit::Seconds),
            value if (1e11..1e14).contains(&value) => Ok(EpochUnit::Milliseconds),
            value if (1e14..1e17).contains(&value) => Ok(EpochUnit::Microseconds),
            value if (1e17..1e20).contains(&value) => Ok(EpochUnit::Nanoseconds),
            _ => bail!(
                "Unix timestamp unit is ambiguous; choose seconds, milliseconds, microseconds, or nanoseconds"
            ),
        }
    }
}

fn epoch_override(format: Option<ForecastTimestampFormat>) -> Option<EpochUnit> {
    match format {
        Some(ForecastTimestampFormat::UnixSeconds) => Some(EpochUnit::Seconds),
        Some(ForecastTimestampFormat::UnixMilliseconds) => Some(EpochUnit::Milliseconds),
        Some(ForecastTimestampFormat::UnixMicroseconds) => Some(EpochUnit::Microseconds),
        Some(ForecastTimestampFormat::UnixNanoseconds) => Some(EpochUnit::Nanoseconds),
        _ => None,
    }
}

fn date_milliseconds(date: DateTime<Utc>) -> Result<i64> {
    if date.timestamp_subsec_nanos() % 1_000_000 != 0 {
        bail!("Forecast timestamps must have whole-millisecond precision");
    }
    Ok(date.timestamp_millis())
}

fn parse_date_with_pattern(text: &str, date: &str) -> Option<DateTime<Utc>> {
    for separator in [" ", "T"] {
        for time in ["%H:%M:%S%.f", "%H:%M"] {
            let pattern = format!("{date}{separator}{time}");
            for zone in ["%:z", " %:z", "%z", " %z"] {
                if let Ok(value) = DateTime::parse_from_str(text, &format!("{pattern}{zone}")) {
                    return Some(value.with_timezone(&Utc));
                }
            }
            if let Ok(value) = NaiveDateTime::parse_from_str(text, &pattern) {
                return Some(value.and_utc());
            }
        }
    }
    NaiveDate::parse_from_str(text, date)
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|value| value.and_utc())
}

fn text_timestamp(text: &str, format: Option<ForecastTimestampFormat>) -> Result<i64> {
    if let Ok(date) =
        DateTime::parse_from_rfc3339(text).or_else(|_| DateTime::parse_from_rfc2822(text))
    {
        return date_milliseconds(date.with_timezone(&Utc));
    }
    for pattern in [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%d %b %Y",
        "%d %B %Y",
        "%b %d, %Y",
        "%B %d, %Y",
        "%d-%b-%Y",
    ] {
        if let Some(date) = parse_date_with_pattern(text, pattern) {
            return date_milliseconds(date);
        }
    }
    let mut day_first = None;
    let mut month_first = None;
    for separator in ["/", "-", "."] {
        day_first = day_first
            .or_else(|| parse_date_with_pattern(text, &format!("%d{separator}%m{separator}%Y")));
        month_first = month_first
            .or_else(|| parse_date_with_pattern(text, &format!("%m{separator}%d{separator}%Y")));
    }
    let selected = match format {
        Some(ForecastTimestampFormat::DayFirst) => day_first,
        Some(ForecastTimestampFormat::MonthFirst) => month_first,
        _ => match (day_first, month_first) {
            (Some(day), Some(month)) if day != month => {
                bail!("Date order is ambiguous; choose day_first or month_first")
            }
            (Some(date), _) | (_, Some(date)) => Some(date),
            _ => None,
        },
    };
    selected
        .ok_or_else(|| {
            anyhow!("Timestamp is not a supported date; use ISO 8601 or choose its date order")
        })
        .and_then(date_milliseconds)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Representation {
    Number,
    NumericText,
    DateText,
}

pub(super) fn parse_column(
    rows: &[Value],
    mapping: &ForecastTableMapping,
    units: &BTreeMap<String, TemporalUnit>,
) -> Result<Vec<i64>> {
    let mut representation = None;
    let mut inferred_unit = None;
    rows.iter().enumerate().map(|(index, row)| {
        let parse = || -> Result<i64> {
            let value = row.get(&mapping.timestamp).ok_or_else(|| anyhow!("Mapped timestamp column is missing"))?;
            if value.is_null() {
                bail!("Timestamp must not be null; filter or fill missing timestamps explicitly");
            }
            if let Some(unit) = units.get(&mapping.timestamp) {
                let value = value.as_i64().ok_or_else(|| anyhow!("Native timestamp/date must contain an integer storage value"))?;
                if matches!(unit, TemporalUnit::Microseconds) && value % 1_000 != 0
                    || matches!(unit, TemporalUnit::Nanoseconds) && value % 1_000_000 != 0
                {
                    bail!("Forecast timestamps must have whole-millisecond precision");
                }
                return epoch_milliseconds(&mapping.timestamp, value, units);
            }
            let (kind, text) = match value {
                Value::Number(number) => {
                    if number.is_f64() && number.as_f64().is_some_and(|value| value.abs() > 9_007_199_254_740_991.0) {
                        bail!("Floating-point epoch exceeds exact integer precision; use an integer or text column");
                    }
                    (Representation::Number, number.to_string())
                },
                Value::String(text) => {
                    let text = text.trim();
                    let kind = if text.parse::<f64>().is_ok() { Representation::NumericText } else { Representation::DateText };
                    (kind, text.to_owned())
                },
                _ => bail!("Timestamp must contain dates or numeric Unix epochs"),
            };
            if representation.is_some_and(|previous| previous != kind) {
                bail!("Timestamp column mixes numeric, numeric-text, or date-text values; normalize it to one representation");
            }
            representation = Some(kind);
            let explicit = epoch_override(mapping.timestamp_format);
            if kind == Representation::DateText {
                if explicit.is_some() { bail!("The selected Unix unit requires numeric timestamps"); }
                return text_timestamp(&text, mapping.timestamp_format);
            }
            if matches!(mapping.timestamp_format, Some(ForecastTimestampFormat::DayFirst | ForecastTimestampFormat::MonthFirst)) {
                bail!("The selected date order requires date text, not numeric epochs");
            }
            let decimal = Decimal::parse(&text)?;
            let unit = if let Some(unit) = explicit {
                unit
            } else if mapping.timestamp_format.is_none() {
                EpochUnit::Milliseconds
            } else {
                let unit = decimal.inferred_unit()?;
                if inferred_unit.is_some_and(|previous| previous != unit) {
                    bail!("Timestamp column mixes Unix units; normalize all rows to the same unit");
                }
                inferred_unit = Some(unit);
                unit
            };
            decimal.milliseconds(unit)
        };
        let mut parse = parse;
        parse().map_err(|error| anyhow!("Timestamp '{}' row {}: {error}", mapping.timestamp, index + 1))
    }).collect()
}
