use flow_like_types::{Value, create_id, sync::Mutex};
use highway::{HighwayHash, HighwayHasher};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::pin::ValueType;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Variable {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub default_value: Option<Vec<u8>>,
    pub data_type: VariableType,
    pub value_type: ValueType,
    pub exposed: bool,
    pub secret: bool,
    pub editable: bool,
    pub hash: Option<u64>,

    #[serde(skip)]
    pub value: Arc<Mutex<Value>>,
}

impl PartialEq for Variable {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.category == other.category
            && self.description == other.description
            && self.default_value == other.default_value
            && self.data_type == other.data_type
            && self.value_type == other.value_type
            && self.exposed == other.exposed
            && self.secret == other.secret
            && self.editable == other.editable
        // Intentionally excluding self.value comparison
    }
}

impl Eq for Variable {}

impl Variable {
    pub fn new(name: &str, data_type: VariableType, value_type: ValueType) -> Self {
        Self {
            id: create_id(),
            name: name.to_string(),
            category: None,
            description: None,
            default_value: None,
            data_type,
            value_type,
            exposed: false,
            secret: false,
            editable: true,
            value: Arc::new(Mutex::new(Value::Null)),
            hash: None,
        }
    }

    pub fn duplicate(&self) -> Self {
        Self {
            id: create_id(),
            name: self.name.clone(),
            category: self.category.clone(),
            description: self.description.clone(),
            default_value: self.default_value.clone(),
            data_type: self.data_type.clone(),
            value_type: self.value_type.clone(),
            exposed: self.exposed,
            secret: self.secret,
            editable: self.editable,
            value: Arc::new(Mutex::new(Value::Null)),
            hash: None,
        }
    }

    pub fn set_editable(&mut self, editable: bool) -> &mut Self {
        self.editable = editable;
        self
    }

    pub fn set_exposed(&mut self, exposed: bool) -> &mut Self {
        self.exposed = exposed;
        self
    }

    pub fn set_secret(&mut self, secret: bool) -> &mut Self {
        self.secret = secret;
        self
    }

    pub fn set_category(&mut self, category: String) -> &mut Self {
        self.category = Some(category);
        self
    }

    pub fn set_description(&mut self, description: String) -> &mut Self {
        self.description = Some(description);
        self
    }

    pub fn set_default_value(&mut self, default_value: Value) -> &mut Self {
        self.default_value = Some(flow_like_types::json::to_vec(&default_value).unwrap());
        self
    }

    pub fn get_value(&self) -> Arc<Mutex<Value>> {
        self.value.clone()
    }

    pub fn hash(&mut self) {
        let mut hasher = HighwayHasher::new(highway::Key([
            0x0123456789abcdfe,
            0xfedcba9876543200,
            0x0011223344556677,
            0x8899aabbccddeeff,
        ]));

        hasher.append(self.id.as_bytes());
        hasher.append(self.name.as_bytes());

        if let Some(category) = &self.category {
            hasher.append(category.as_bytes());
        }

        if let Some(description) = &self.description {
            hasher.append(description.as_bytes());
        }

        // We don´t leak secret values in the hash
        if !self.secret
            && let Some(default_value) = &self.default_value
        {
            hasher.append(default_value);
        }

        hasher.append(format!("{:?}", self.data_type).as_bytes());
        hasher.append(format!("{:?}", self.value_type).as_bytes());
        hasher.append(&[self.exposed as u8]);
        hasher.append(&[self.secret as u8]);
        hasher.append(&[self.editable as u8]);

        self.hash = Some(hasher.finalize64());
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub enum VariableType {
    Execution,
    String,
    Integer,
    Float,
    Boolean,
    Date,
    PathBuf,
    Generic,
    Struct,
    Byte,
}

#[cfg(test)]
mod tests {
    use flow_like_types::{FromProto, ToProto};
    use flow_like_types::{Message, tokio};

    #[tokio::test]
    async fn serialize_variable() {
        let variable = super::Variable::new(
            "name",
            super::VariableType::Execution,
            super::ValueType::Normal,
        );

        let mut buf = Vec::new();
        variable.to_proto().encode(&mut buf).unwrap();
        let deser = super::Variable::from_proto(
            flow_like_types::proto::Variable::decode(&buf[..]).unwrap(),
        );

        assert_eq!(variable.id, deser.id);
    }
}
