//! Platform-wide sensitivity labels: a classification level and a personal-data flag that any
//! resource (table columns, files, variables, …) can carry and that roll up across sources.

use std::{collections::HashMap, fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Key/value metadata key holding a [`DataClassification`] name.
pub const CLASSIFICATION_METADATA_KEY: &str = "flow-like:classification";
/// Key/value metadata key holding `"true"` when the resource contains personal data.
pub const PII_METADATA_KEY: &str = "flow-like:pii";

/// How sensitive data is, from least to most sensitive.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum DataClassification {
    /// May be shared with anyone.
    Public,
    /// For members of the organization only.
    Internal,
    /// Limited to the people who need it; disclosure causes harm.
    Confidential,
    /// Most sensitive; disclosure causes severe harm.
    Restricted,
}

impl DataClassification {
    pub const ALL: [Self; 4] = [
        Self::Public,
        Self::Internal,
        Self::Confidential,
        Self::Restricted,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Confidential => "confidential",
            Self::Restricted => "restricted",
        }
    }

    /// The level named `value`, ignoring case and surrounding whitespace.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        Self::ALL
            .into_iter()
            .find(|level| level.as_str().eq_ignore_ascii_case(value))
    }

    /// Every level's name, least sensitive first; the valid values of a classification pin.
    pub fn names() -> Vec<String> {
        Self::ALL
            .into_iter()
            .map(|level| level.as_str().to_string())
            .collect()
    }
}

impl fmt::Display for DataClassification {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DataClassification {
    type Err = UnknownDataClassification;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or_else(|| UnknownDataClassification(value.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownDataClassification(pub String);

impl fmt::Display for UnknownDataClassification {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Unknown data classification '{}'; use public, internal, confidential or restricted",
            self.0
        )
    }
}

impl std::error::Error for UnknownDataClassification {}

/// A resource's sensitivity label. Empty means nobody has labelled it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct DataSensitivity {
    /// Sensitivity level; unset when nobody has classified the data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<DataClassification>,
    /// Whether the data identifies or relates to a person.
    pub pii: bool,
}

impl DataSensitivity {
    pub fn is_empty(&self) -> bool {
        self.classification.is_none() && !self.pii
    }

    /// The label of data derived from both: the stricter classification, and personal
    /// data when either holds it.
    pub fn combine(self, other: Self) -> Self {
        Self {
            classification: self.classification.max(other.classification),
            pii: self.pii || other.pii,
        }
    }

    pub fn combined(labels: impl IntoIterator<Item = Self>) -> Self {
        labels.into_iter().fold(Self::default(), Self::combine)
    }

    /// Reads the label from key/value metadata. An unknown classification reads as unset.
    pub fn from_metadata(metadata: &HashMap<String, String>) -> Self {
        Self {
            classification: metadata
                .get(CLASSIFICATION_METADATA_KEY)
                .and_then(|value| DataClassification::parse(value)),
            pii: metadata.get(PII_METADATA_KEY).is_some_and(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "true" | "1" | "yes"
                )
            }),
        }
    }

    /// The metadata changes that store exactly this label: `Some` sets a key, `None`
    /// removes it.
    pub fn metadata_changes(&self) -> [(String, Option<String>); 2] {
        [
            (
                CLASSIFICATION_METADATA_KEY.to_string(),
                self.classification.map(|level| level.as_str().to_string()),
            ),
            (
                PII_METADATA_KEY.to_string(),
                self.pii.then(|| "true".to_string()),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_order_from_least_to_most_sensitive() {
        assert!(DataClassification::Public < DataClassification::Internal);
        assert!(DataClassification::Internal < DataClassification::Confidential);
        assert!(DataClassification::Confidential < DataClassification::Restricted);
        assert_eq!(
            DataClassification::names(),
            ["public", "internal", "confidential", "restricted"]
        );
    }

    #[test]
    fn parses_names_case_insensitively() {
        assert_eq!(
            " Confidential ".parse::<DataClassification>(),
            Ok(DataClassification::Confidential)
        );
        let error = "secret".parse::<DataClassification>().unwrap_err();
        assert!(error.to_string().contains("'secret'"), "{error}");
    }

    #[test]
    fn serializes_as_lowercase_names() {
        let label = DataSensitivity {
            classification: Some(DataClassification::Restricted),
            pii: true,
        };
        let json = serde_json::to_value(label).unwrap();

        assert_eq!(
            json,
            serde_json::json!({ "classification": "restricted", "pii": true })
        );
        assert_eq!(
            serde_json::from_value::<DataSensitivity>(serde_json::json!({})).unwrap(),
            DataSensitivity::default()
        );
    }

    #[test]
    fn combining_keeps_the_stricter_level_and_any_personal_data() {
        let combined = DataSensitivity::combined([
            DataSensitivity {
                classification: Some(DataClassification::Internal),
                pii: false,
            },
            DataSensitivity {
                classification: None,
                pii: true,
            },
            DataSensitivity {
                classification: Some(DataClassification::Confidential),
                pii: false,
            },
        ]);

        assert_eq!(
            combined,
            DataSensitivity {
                classification: Some(DataClassification::Confidential),
                pii: true,
            }
        );
        assert!(DataSensitivity::combined([]).is_empty());
    }

    #[test]
    fn round_trips_through_metadata() {
        let label = DataSensitivity {
            classification: Some(DataClassification::Public),
            pii: true,
        };
        let stored: HashMap<String, String> = label
            .metadata_changes()
            .into_iter()
            .filter_map(|(key, value)| value.map(|value| (key, value)))
            .collect();

        assert_eq!(DataSensitivity::from_metadata(&stored), label);
        assert!(
            DataSensitivity::default()
                .metadata_changes()
                .iter()
                .all(|(_, value)| value.is_none())
        );
    }
}
