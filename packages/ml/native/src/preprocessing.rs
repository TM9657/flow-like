use flow_like_ml_core::{Result, TensorData, content_digest, require};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Imputation {
    #[default]
    Median,
    Mean,
    Constant {
        value: f64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TabularOptions {
    pub numeric_columns: Vec<String>,
    pub categorical_columns: Vec<String>,
    #[serde(default)]
    pub imputation: Imputation,
    #[serde(default = "default_standardize")]
    pub standardize: bool,
    #[serde(default = "default_categories")]
    pub maximum_categories_per_column: usize,
    #[serde(default = "default_features")]
    pub maximum_output_features: usize,
}
fn default_standardize() -> bool {
    true
}
fn default_categories() -> usize {
    256
}
fn default_features() -> usize {
    4096
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FittedColumn {
    Numeric {
        name: String,
        imputation: f64,
        offset: f64,
        scale: f64,
    },
    /// Missing and unseen values have separate slots before the sorted vocabulary.
    Categorical {
        name: String,
        vocabulary: Vec<String>,
    },
}

/// Version 1 keyed categories by raw JSON text, so `3` and `3.0` were different categories.
const LEGACY_FORMAT_VERSION: u32 = 1;
const FORMAT_VERSION: u32 = 2;
const MAX_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct FittedTabularPreprocessor {
    pub format_version: u32,
    pub columns: Vec<FittedColumn>,
    pub output_features: Vec<String>,
    pub training_rows: usize,
    pub training_digest: String,
}

fn cell<'a>(row: &'a Value, name: &str) -> Result<&'a Value> {
    row.as_object()
        .and_then(|row| row.get(name))
        .ok_or_else(|| flow_like_ml_core::Error::Invalid(format!("Row is missing column '{name}'")))
}
fn numeric(value: &Value, name: &str) -> Result<Option<f64>> {
    if value.is_null() {
        return Ok(None);
    }
    let value = value.as_f64().ok_or_else(|| {
        flow_like_ml_core::Error::Invalid(format!("Column '{name}' must contain numbers or null"))
    })?;
    require(
        value.is_finite(),
        format!("Column '{name}' contains a non-finite number"),
    )?;
    Ok(Some(value))
}
/// Canonical keys write integral numbers up to 2^53 as integers, so an integer and a float
/// column carrying the same code agree.
fn category(value: &Value, name: &str, canonical: bool) -> Result<Option<String>> {
    if value.is_null() {
        return Ok(None);
    }
    require(
        value.is_string() || value.is_boolean() || value.is_number(),
        format!("Categorical column '{name}' must contain scalar values"),
    )?;
    let key = match value.as_f64() {
        Some(number)
            if canonical
                && !value.is_i64()
                && !value.is_u64()
                && number.fract() == 0.0
                && number.abs() <= MAX_EXACT_INTEGER =>
        {
            (number as i64).to_string()
        }
        _ => serde_json::to_string(value)?,
    };
    require(key.len() <= 4096, "Categorical value exceeds 4096 bytes")?;
    Ok(Some(key))
}

fn average(values: &[f64]) -> f64 {
    let count = values.len() as f64;
    let quotient = values.iter().sum::<f64>() / count;
    if quotient.is_finite() {
        quotient
    } else {
        values.iter().map(|value| value / count).sum()
    }
}

/// Offset and scale of a column. A constant column, or one whose spread is within the rounding
/// error of the two-pass variance (n * eps * |mean|), keeps scale 1 so later deviations stay finite
/// and proportionate instead of being divided by rounding noise.
fn standardization(values: &[f64]) -> (f64, f64) {
    let (minimum, maximum) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(minimum, maximum), value| {
            (minimum.min(*value), maximum.max(*value))
        });
    if minimum == maximum {
        return (minimum, 1.0);
    }
    let count = values.len() as f64;
    let mean = average(values);
    let deviation = values
        .iter()
        .map(|value| (value - mean).powi(2) / count)
        .sum::<f64>()
        .sqrt();
    if deviation <= count * f64::EPSILON * mean.abs() {
        (mean, 1.0)
    } else {
        (mean, deviation)
    }
}

impl FittedTabularPreprocessor {
    /// Only the supplied training indices contribute to statistics and vocabulary.
    pub fn fit(rows: &[Value], training: &[usize], options: &TabularOptions) -> Result<Self> {
        require(!training.is_empty(), "Preprocessing requires training rows")?;
        require(
            (1..=4096).contains(&options.maximum_categories_per_column)
                && (1..=65536).contains(&options.maximum_output_features),
            "Invalid preprocessing feature limits",
        )?;
        let mut indices = HashSet::new();
        require(
            training
                .iter()
                .all(|index| *index < rows.len() && indices.insert(*index)),
            "Training indices must be unique and valid",
        )?;
        let mut names = HashSet::new();
        require(
            options
                .numeric_columns
                .iter()
                .chain(&options.categorical_columns)
                .all(|name| {
                    !name.trim().is_empty() && name.len() <= 1024 && names.insert(name.as_str())
                })
                && !names.is_empty()
                && names.len() <= options.maximum_output_features,
            "Feature columns must be explicit, unique and bounded",
        )?;
        let mut columns = Vec::new();
        let mut output_features = Vec::new();
        for name in &options.numeric_columns {
            let values = training
                .iter()
                .map(|index| numeric(cell(&rows[*index], name)?, name))
                .collect::<Result<Vec<_>>>()?;
            let mut present = values.iter().flatten().copied().collect::<Vec<_>>();
            let imputation = match options.imputation {
                Imputation::Constant { value } => {
                    require(value.is_finite(), "Imputation constant must be finite")?;
                    value
                }
                Imputation::Mean => {
                    require(
                        !present.is_empty(),
                        format!(
                            "Column '{name}' is entirely missing in training; select constant imputation or remove it"
                        ),
                    )?;
                    average(&present)
                }
                Imputation::Median => {
                    require(
                        !present.is_empty(),
                        format!(
                            "Column '{name}' is entirely missing in training; select constant imputation or remove it"
                        ),
                    )?;
                    present.sort_by(f64::total_cmp);
                    let middle = present.len() / 2;
                    if present.len() % 2 == 0 {
                        present[middle - 1] / 2.0 + present[middle] / 2.0
                    } else {
                        present[middle]
                    }
                }
            };
            let filled = values
                .iter()
                .map(|value| value.unwrap_or(imputation))
                .collect::<Vec<_>>();
            let (offset, scale) = if options.standardize {
                standardization(&filled)
            } else {
                (0.0, 1.0)
            };
            require(
                imputation.is_finite() && offset.is_finite() && scale.is_finite() && scale > 0.0,
                "Numeric preprocessing statistics overflowed",
            )?;
            columns.push(FittedColumn::Numeric {
                name: name.clone(),
                imputation,
                offset,
                scale,
            });
            output_features.push(name.clone());
        }
        for name in &options.categorical_columns {
            let mut vocabulary = BTreeSet::new();
            for index in training {
                if let Some(value) = category(cell(&rows[*index], name)?, name, true)? {
                    vocabulary.insert(value);
                }
                require(
                    vocabulary.len() <= options.maximum_categories_per_column,
                    format!("Column '{name}' exceeds its category budget"),
                )?;
            }
            let vocabulary = vocabulary.into_iter().collect::<Vec<_>>();
            output_features.extend([format!("{name}:missing"), format!("{name}:unknown")]);
            output_features.extend(vocabulary.iter().map(|value| format!("{name}:{value}")));
            require(
                output_features.len() <= options.maximum_output_features,
                "Encoded features exceed the output feature budget",
            )?;
            columns.push(FittedColumn::Categorical {
                name: name.clone(),
                vocabulary,
            });
        }
        let training_values = training
            .iter()
            .map(|index| {
                options
                    .numeric_columns
                    .iter()
                    .chain(&options.categorical_columns)
                    .map(|name| cell(&rows[*index], name).cloned())
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let result = Self {
            format_version: FORMAT_VERSION,
            columns,
            output_features,
            training_rows: training.len(),
            training_digest: content_digest(&serde_json::to_vec(&training_values)?),
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<()> {
        require(
            matches!(self.format_version, LEGACY_FORMAT_VERSION | FORMAT_VERSION)
                && self.training_rows > 0
                && !self.columns.is_empty()
                && self.columns.len() <= 65536,
            "Invalid preprocessing manifest version or dimensions",
        )?;
        require(
            self.training_digest.len() == 64
                && self
                    .training_digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "Invalid preprocessing training digest",
        )?;
        let mut names = HashSet::new();
        let mut features = Vec::new();
        for column in &self.columns {
            let name = match column {
                FittedColumn::Numeric {
                    name,
                    imputation,
                    offset,
                    scale,
                } => {
                    require(
                        imputation.is_finite()
                            && offset.is_finite()
                            && scale.is_finite()
                            && *scale > 0.0,
                        "Invalid fitted numeric statistics",
                    )?;
                    features.push(name.clone());
                    name
                }
                FittedColumn::Categorical { name, vocabulary } => {
                    require(
                        vocabulary.len() <= 4096
                            && features.len() + vocabulary.len() + 2 <= 65536
                            && vocabulary.windows(2).all(|pair| pair[0] < pair[1])
                            && vocabulary.iter().all(|value| value.len() <= 4096),
                        "Categorical vocabulary must be sorted, unique and bounded",
                    )?;
                    features.extend([format!("{name}:missing"), format!("{name}:unknown")]);
                    features.extend(vocabulary.iter().map(|value| format!("{name}:{value}")));
                    name
                }
            };
            require(
                !name.trim().is_empty() && name.len() <= 1024 && names.insert(name),
                "Preprocessing contains duplicate or empty column names",
            )?;
        }
        require(
            features == self.output_features && features.len() <= 65536,
            "Preprocessing output schema differs from fitted columns",
        )
    }

    pub fn transform_row(&self, row: &Value) -> Result<TensorData> {
        self.validate()?;
        self.transform_validated_row(row)
    }

    fn transform_validated_row(&self, row: &Value) -> Result<TensorData> {
        let mut values = Vec::with_capacity(self.output_features.len());
        for column in &self.columns {
            match column {
                FittedColumn::Numeric {
                    name,
                    imputation,
                    offset,
                    scale,
                } => {
                    let value = ((numeric(cell(row, name)?, name)?.unwrap_or(*imputation) - offset)
                        / scale) as f32;
                    require(
                        value.is_finite(),
                        format!("Transformed column '{name}' exceeds float32 range"),
                    )?;
                    values.push(value);
                }
                FittedColumn::Categorical { name, vocabulary } => {
                    let value = category(
                        cell(row, name)?,
                        name,
                        self.format_version != LEGACY_FORMAT_VERSION,
                    )?;
                    let index = match value {
                        None => 0,
                        Some(value) => vocabulary
                            .binary_search(&value)
                            .map_or(1, |index| index + 2),
                    };
                    let start = values.len();
                    values.resize(start + vocabulary.len() + 2, 0.0);
                    values[start + index] = 1.0;
                }
            }
        }
        Ok(TensorData {
            shape: vec![values.len()],
            values,
        })
    }

    pub fn transform_rows(
        &self,
        rows: &[Value],
        maximum_elements: usize,
    ) -> Result<Vec<TensorData>> {
        self.validate()?;
        require(
            rows.len()
                .checked_mul(self.output_features.len())
                .is_some_and(|count| count <= maximum_elements),
            "Transformed dataset exceeds its element budget",
        )?;
        rows.iter()
            .map(|row| self.transform_validated_row(row))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn options() -> TabularOptions {
        TabularOptions {
            numeric_columns: vec!["value".into()],
            categorical_columns: vec!["category".into()],
            imputation: Imputation::Median,
            standardize: true,
            maximum_categories_per_column: 10,
            maximum_output_features: 20,
        }
    }

    #[test]
    fn held_out_values_never_change_scaling_or_vocabulary() {
        let rows = vec![
            json!({"value":0,"category":"A"}),
            json!({"value":2,"category":"A"}),
            json!({"value":1000,"category":"test-only"}),
        ];
        let fitted = FittedTabularPreprocessor::fit(&rows, &[0, 1], &options()).unwrap();
        assert_eq!(
            fitted.transform_row(&rows[0]).unwrap().values,
            vec![-1.0, 0.0, 0.0, 1.0]
        );
        assert_eq!(
            fitted.transform_row(&rows[2]).unwrap().values,
            vec![999.0, 0.0, 1.0, 0.0]
        );
        let mut changed = rows.clone();
        changed[2] = json!({"value":-100000,"category":"different"});
        assert_eq!(
            fitted,
            FittedTabularPreprocessor::fit(&changed, &[0, 1], &options()).unwrap()
        );
        let restored: FittedTabularPreprocessor =
            serde_json::from_slice(&serde_json::to_vec(&fitted).unwrap()).unwrap();
        assert_eq!(
            restored.transform_row(&rows[2]).unwrap(),
            fitted.transform_row(&rows[2]).unwrap()
        );
    }
    #[test]
    fn imputation_and_missing_unknown_slots_are_stable_and_bounded() {
        let rows = vec![
            json!({"value":1,"category":null}),
            json!({"value":3,"category":"A"}),
            json!({"value":null,"category":"A"}),
        ];
        let fitted = FittedTabularPreprocessor::fit(&rows, &[0, 1, 2], &options()).unwrap();
        assert_eq!(
            fitted
                .transform_row(&json!({"value":null,"category":null}))
                .unwrap()
                .values,
            vec![0.0, 1.0, 0.0, 0.0]
        );
        assert!(fitted.transform_rows(&rows, 11).is_err());
        let mut corrupt = fitted;
        corrupt.output_features.clear();
        assert!(corrupt.validate().is_err());
        assert!(FittedTabularPreprocessor::fit(&rows, &[0, 0], &options()).is_err());
        assert!(
            FittedTabularPreprocessor::fit(
                &[json!({"value":null,"category":"A"})],
                &[0],
                &options()
            )
            .is_err()
        );
    }

    fn single_column(numeric: bool, imputation: Imputation) -> TabularOptions {
        let column = vec![String::from(if numeric { "value" } else { "code" })];
        TabularOptions {
            numeric_columns: if numeric { column.clone() } else { vec![] },
            categorical_columns: if numeric { vec![] } else { column },
            imputation,
            standardize: true,
            maximum_categories_per_column: 10,
            maximum_output_features: 20,
        }
    }

    fn encoded(fitted: &FittedTabularPreprocessor, row: Value) -> Vec<f32> {
        fitted.transform_row(&row).unwrap().values
    }

    fn hot_slot(fitted: &FittedTabularPreprocessor, code: Value) -> usize {
        let values = encoded(fitted, json!({ "code": code }));
        values.iter().position(|value| *value == 1.0).unwrap()
    }

    #[test]
    fn constant_training_columns_keep_unit_scale() {
        let options = single_column(true, Imputation::Median);
        for constant in [1.0, 25.0, 0.1] {
            let rows: Vec<_> = (0..700).map(|_| json!({"value": constant})).collect();
            let training: Vec<_> = (0..rows.len()).collect();
            let fitted = FittedTabularPreprocessor::fit(&rows, &training, &options).unwrap();
            let expected = FittedColumn::Numeric {
                name: "value".into(),
                imputation: constant,
                offset: constant,
                scale: 1.0,
            };
            assert_eq!(fitted.columns, [expected]);
            assert_eq!(encoded(&fitted, json!({"value": constant + 1.0})), [1.0]);
        }
        let noisy: Vec<_> = (0..700)
            .map(|index| json!({"value": if index % 2 == 0 { 0.3 } else { 0.1 + 0.2 }}))
            .collect();
        let training: Vec<_> = (0..noisy.len()).collect();
        let options = single_column(true, Imputation::Mean);
        let fitted = FittedTabularPreprocessor::fit(&noisy, &training, &options).unwrap();
        let FittedColumn::Numeric { scale, .. } = fitted.columns[0] else {
            panic!("Expected a numeric column");
        };
        assert_eq!(scale, 1.0);
        assert!((encoded(&fitted, json!({"value": 1.3}))[0] - 1.0).abs() < 1e-6);
        let missing = [json!({"value": null}), json!({"value": null})];
        let options = single_column(true, Imputation::Constant { value: 5.0 });
        let fitted = FittedTabularPreprocessor::fit(&missing, &[0, 1], &options).unwrap();
        assert_eq!(encoded(&fitted, json!({"value": null})), [0.0]);
        assert_eq!(encoded(&fitted, json!({"value": 6.0})), [1.0]);
    }

    #[test]
    fn integer_and_integral_float_codes_share_a_category() {
        let options = single_column(false, Imputation::Median);
        let training = [0, 1, 2, 3];
        let floats = [
            json!({"code": 1.0}),
            json!({"code": 3.0}),
            json!({"code": 0.5}),
            json!({"code": "3"}),
        ];
        let fitted = FittedTabularPreprocessor::fit(&floats, &training, &options).unwrap();
        assert_eq!(fitted.format_version, FORMAT_VERSION);
        let FittedColumn::Categorical { vocabulary, .. } = &fitted.columns[0] else {
            panic!("Expected a categorical column");
        };
        assert_eq!(vocabulary, &["\"3\"", "0.5", "1", "3"]);
        assert_eq!(hot_slot(&fitted, json!(3)), 5);
        assert_eq!(hot_slot(&fitted, json!(3.0)), 5);
        assert_eq!(hot_slot(&fitted, json!("3")), 2);
        assert_eq!(hot_slot(&fitted, json!(0.5)), 3);
        assert_eq!(hot_slot(&fitted, json!(2)), 1);
        let integers = [
            json!({"code": 1}),
            json!({"code": 3}),
            json!({"code": 0.5}),
            json!({"code": "3"}),
        ];
        let refitted = FittedTabularPreprocessor::fit(&integers, &training, &options).unwrap();
        assert_eq!(refitted.columns, fitted.columns);
    }

    #[test]
    fn legacy_manifests_keep_their_training_time_category_keys() {
        let legacy = FittedTabularPreprocessor {
            format_version: LEGACY_FORMAT_VERSION,
            columns: vec![FittedColumn::Categorical {
                name: "code".into(),
                vocabulary: vec!["1.0".into(), "3.0".into()],
            }],
            output_features: ["code:missing", "code:unknown", "code:1.0", "code:3.0"]
                .map(String::from)
                .to_vec(),
            training_rows: 2,
            training_digest: "0".repeat(64),
        };
        let restored: FittedTabularPreprocessor =
            serde_json::from_value(serde_json::to_value(&legacy).unwrap()).unwrap();
        assert_eq!(hot_slot(&restored, json!(3.0)), 3);
        assert_eq!(hot_slot(&restored, json!(1.0)), 2);
        assert_eq!(hot_slot(&restored, json!(3)), 1);
        let unsupported = FittedTabularPreprocessor {
            format_version: FORMAT_VERSION + 1,
            ..legacy
        };
        assert!(unsupported.validate().is_err());
    }
}
