//! What a table column means and who may read it, stored as Lance field metadata so it
//! travels with the table through versions, branches, clones and exports.

use std::collections::{BTreeSet, HashMap};

pub use flow_like_types_contracts::data_classification::{
    CLASSIFICATION_METADATA_KEY, DataClassification, DataSensitivity, PII_METADATA_KEY,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DISPLAY_NAME_KEY: &str = "flow-like:display-name";
pub const DESCRIPTION_KEY: &str = "flow-like:description";
pub const READ_ATTRIBUTES_KEY: &str = "flow-like:read-attributes";

pub const COLUMN_METADATA_KEYS: [&str; 5] = [
    DISPLAY_NAME_KEY,
    DESCRIPTION_KEY,
    CLASSIFICATION_METADATA_KEY,
    PII_METADATA_KEY,
    READ_ATTRIBUTES_KEY,
];

pub const MAX_DISPLAY_NAME_CHARS: usize = 128;
pub const MAX_DESCRIPTION_CHARS: usize = 4096;
pub const MAX_READ_ATTRIBUTES: usize = 64;
pub const MAX_ATTRIBUTE_CHARS: usize = 128;

/// What a column means and who may read it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ColumnMetadata {
    /// Human-readable name shown in place of the column name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// What the column holds and how to use it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Classification level and personal-data flag.
    #[serde(flatten)]
    pub sensitivity: DataSensitivity,
    /// Role attributes allowed to read the column. Empty: everyone who may read the table.
    /// Otherwise only admins and roles holding at least one of these attributes see it.
    pub read_attributes: Vec<String>,
}

impl ColumnMetadata {
    /// Reads the metadata from a column's field metadata. Unknown classifications are
    /// ignored; read attributes accept a JSON string array or comma-separated text.
    pub fn from_field_metadata(metadata: &HashMap<String, String>) -> Self {
        let text = |key: &str| {
            metadata
                .get(key)
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };
        Self {
            display_name: text(DISPLAY_NAME_KEY),
            description: text(DESCRIPTION_KEY),
            sensitivity: DataSensitivity::from_metadata(metadata),
            read_attributes: read_attributes(metadata),
        }
    }

    /// Trims every value, drops blank ones and duplicate attributes, and checks the limits.
    pub fn normalized(self, column: &str) -> anyhow::Result<Self> {
        let display_name = bounded_text(
            self.display_name,
            MAX_DISPLAY_NAME_CHARS,
            "display name",
            column,
        )?;
        let description = bounded_text(
            self.description,
            MAX_DESCRIPTION_CHARS,
            "description",
            column,
        )?;
        let read_attributes = normalize_attributes(self.read_attributes);
        anyhow::ensure!(
            read_attributes.len() <= MAX_READ_ATTRIBUTES,
            "Column '{column}' lists {} read attributes; at most {MAX_READ_ATTRIBUTES} are allowed",
            read_attributes.len()
        );
        if let Some(attribute) = read_attributes
            .iter()
            .find(|attribute| attribute.chars().count() > MAX_ATTRIBUTE_CHARS)
        {
            anyhow::bail!(
                "Read attribute '{attribute}' of column '{column}' is longer than {MAX_ATTRIBUTE_CHARS} characters"
            );
        }
        Ok(Self {
            display_name,
            description,
            sensitivity: self.sensitivity,
            read_attributes,
        })
    }

    /// The field metadata changes that store exactly this metadata: `Some` sets a key,
    /// `None` removes it. Keys outside [`COLUMN_METADATA_KEYS`] are left alone.
    pub fn field_metadata_changes(&self) -> HashMap<String, Option<String>> {
        let attributes = (!self.read_attributes.is_empty()).then(|| {
            serde_json::to_string(&self.read_attributes)
                .expect("a list of strings always serializes")
        });
        let mut changes = HashMap::from([
            (DISPLAY_NAME_KEY.to_string(), self.display_name.clone()),
            (DESCRIPTION_KEY.to_string(), self.description.clone()),
            (READ_ATTRIBUTES_KEY.to_string(), attributes),
        ]);
        changes.extend(self.sensitivity.metadata_changes());
        changes
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    pub fn is_read_restricted(&self) -> bool {
        !self.read_attributes.is_empty()
    }
}

/// The role attributes a column's field metadata restricts reading to; empty when anyone
/// who may read the table may read the column.
pub fn read_attributes(metadata: &HashMap<String, String>) -> Vec<String> {
    metadata
        .get(READ_ATTRIBUTES_KEY)
        .map(|value| parse_attributes(value))
        .unwrap_or_default()
}

/// Whether `metadata` carries any key of [`COLUMN_METADATA_KEYS`].
pub fn has_column_metadata(metadata: &HashMap<String, String>) -> bool {
    COLUMN_METADATA_KEYS
        .iter()
        .any(|key| metadata.contains_key(*key))
}

/// `metadata` without the keys of [`COLUMN_METADATA_KEYS`].
pub fn without_column_metadata(metadata: &HashMap<String, String>) -> HashMap<String, String> {
    metadata
        .iter()
        .filter(|(key, _)| !COLUMN_METADATA_KEYS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// Who is reading, as far as column read restrictions are concerned.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ColumnReader {
    /// Admins, owners and local runs read every column.
    #[default]
    Unrestricted,
    /// A role that sees restricted columns only through one of these attributes.
    Attributes(BTreeSet<String>),
}

impl ColumnReader {
    pub fn with_attributes<I, S>(attributes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self::Attributes(
            attributes
                .into_iter()
                .map(|attribute| attribute.as_ref().trim().to_string())
                .filter(|attribute| !attribute.is_empty())
                .collect(),
        )
    }

    pub fn is_unrestricted(&self) -> bool {
        matches!(self, Self::Unrestricted)
    }

    pub fn can_read(&self, read_attributes: &[String]) -> bool {
        match self {
            Self::Unrestricted => true,
            Self::Attributes(held) => {
                read_attributes.is_empty()
                    || read_attributes
                        .iter()
                        .any(|attribute| held.contains(attribute))
            }
        }
    }

    pub fn can_read_field(&self, field_metadata: &HashMap<String, String>) -> bool {
        self.is_unrestricted() || self.can_read(&read_attributes(field_metadata))
    }
}

fn parse_attributes(raw: &str) -> Vec<String> {
    let raw = raw.trim();
    let listed = serde_json::from_str::<Vec<String>>(raw).unwrap_or_else(|_| {
        raw.trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(|attribute| attribute.trim().trim_matches('"').to_string())
            .collect()
    });
    normalize_attributes(listed)
}

fn normalize_attributes(attributes: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    attributes
        .into_iter()
        .map(|attribute| attribute.trim().to_string())
        .filter(|attribute| !attribute.is_empty() && seen.insert(attribute.clone()))
        .collect()
}

fn bounded_text(
    value: Option<String>,
    max_chars: usize,
    label: &str,
    column: &str,
) -> anyhow::Result<Option<String>> {
    let Some(value) = value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let chars = value.chars().count();
    anyhow::ensure!(
        chars <= max_chars,
        "The {label} of column '{column}' has {chars} characters; at most {max_chars} are allowed"
    );
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(metadata: &ColumnMetadata) -> HashMap<String, String> {
        metadata
            .field_metadata_changes()
            .into_iter()
            .filter_map(|(key, value)| value.map(|value| (key, value)))
            .collect()
    }

    fn salary() -> ColumnMetadata {
        ColumnMetadata {
            display_name: Some("Annual salary".into()),
            description: Some("Gross, in EUR".into()),
            sensitivity: DataSensitivity {
                classification: Some(DataClassification::Confidential),
                pii: true,
            },
            read_attributes: vec!["hr".into(), "finance, eu".into()],
        }
    }

    #[test]
    fn round_trips_through_field_metadata() {
        let metadata = salary();

        assert_eq!(
            ColumnMetadata::from_field_metadata(&stored(&metadata)),
            metadata
        );
    }

    #[test]
    fn serializes_flat() {
        let json = serde_json::to_value(salary()).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "display_name": "Annual salary",
                "description": "Gross, in EUR",
                "classification": "confidential",
                "pii": true,
                "read_attributes": ["hr", "finance, eu"],
            })
        );
        assert_eq!(
            serde_json::from_value::<ColumnMetadata>(serde_json::json!({})).unwrap(),
            ColumnMetadata::default()
        );
    }

    #[test]
    fn empty_metadata_removes_every_key() {
        let changes = ColumnMetadata::default().field_metadata_changes();

        assert_eq!(changes.len(), COLUMN_METADATA_KEYS.len());
        assert!(
            COLUMN_METADATA_KEYS
                .iter()
                .all(|key| changes.get(*key) == Some(&None))
        );
    }

    #[test]
    fn normalizes_blank_values_and_duplicate_attributes() {
        let metadata = ColumnMetadata {
            display_name: Some("  ".into()),
            description: Some(" Notes ".into()),
            read_attributes: vec![" hr ".into(), "hr".into(), "".into(), "legal".into()],
            ..ColumnMetadata::default()
        }
        .normalized("notes")
        .unwrap();

        assert_eq!(metadata.display_name, None);
        assert_eq!(metadata.description.as_deref(), Some("Notes"));
        assert_eq!(metadata.read_attributes, vec!["hr", "legal"]);
    }

    #[test]
    fn rejects_values_over_the_limits() {
        let error = ColumnMetadata {
            display_name: Some("x".repeat(MAX_DISPLAY_NAME_CHARS + 1)),
            ..ColumnMetadata::default()
        }
        .normalized("salary")
        .unwrap_err()
        .to_string();

        assert!(
            error.contains("salary") && error.contains("display name"),
            "{error}"
        );
    }

    #[test]
    fn reads_attributes_written_as_plain_text() {
        let metadata = HashMap::from([
            (READ_ATTRIBUTES_KEY.to_string(), "hr, legal ,".to_string()),
            ("other".to_string(), "kept".to_string()),
        ]);

        assert_eq!(read_attributes(&metadata), vec!["hr", "legal"]);
        assert!(has_column_metadata(&metadata));
        assert_eq!(
            without_column_metadata(&metadata),
            HashMap::from([("other".to_string(), "kept".to_string())])
        );
    }

    #[test]
    fn restricted_columns_need_a_listed_attribute() {
        let restricted = vec!["hr".to_string(), "finance".to_string()];

        assert!(ColumnReader::Unrestricted.can_read(&restricted));
        assert!(ColumnReader::with_attributes(["finance"]).can_read(&restricted));
        assert!(!ColumnReader::with_attributes(["sales"]).can_read(&restricted));
        assert!(!ColumnReader::with_attributes(Vec::<String>::new()).can_read(&restricted));
        assert!(ColumnReader::with_attributes(["sales"]).can_read(&[]));
    }
}
