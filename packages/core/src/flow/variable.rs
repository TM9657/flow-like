use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

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

    #[serde(skip)]
    pub value: Arc<Mutex<Value>>,
}

impl Variable {
    pub fn new(name: &str, data_type: VariableType, value_type: ValueType) -> Self {
        Self {
            id: cuid2::create_id(),
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
        }
    }

    pub fn duplicate(&self) -> Self {
        Self {
            id: cuid2::create_id(),
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
        self.default_value = Some(serde_json::to_vec(&default_value).unwrap());
        self
    }

    pub fn get_value(&self) -> Arc<Mutex<Value>> {
        self.value.clone()
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
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
    use crate::protobuf::conversions::{FromProto, ToProto};
    use prost::Message;

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
            crate::protobuf::types::Variable::decode(&buf[..]).unwrap(),
        );

        assert_eq!(variable.id, deser.id);
    }
}
