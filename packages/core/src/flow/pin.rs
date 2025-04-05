use super::variable::VariableType;
use flow_like_types::{Value, json::to_string_pretty, sync::Mutex};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum PinType {
    Input,
    Output,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct PinOptions {
    pub valid_values: Option<Vec<String>>,
    pub range: Option<(f64, f64)>,
    pub step: Option<f64>,
    pub enforce_schema: Option<bool>,
    pub enforce_generic_value_type: Option<bool>,
}

impl Default for PinOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl PinOptions {
    pub fn new() -> Self {
        PinOptions {
            valid_values: None,
            range: None,
            step: None,
            enforce_schema: None,
            enforce_generic_value_type: None,
        }
    }

    pub fn set_valid_values(&mut self, valid_values: Vec<String>) -> &mut Self {
        self.valid_values = Some(valid_values);
        self
    }

    pub fn set_range(&mut self, range: (f64, f64)) -> &mut Self {
        self.range = Some(range);
        self
    }

    pub fn set_step(&mut self, step: f64) -> &mut Self {
        self.step = Some(step);
        self
    }

    pub fn set_enforce_schema(&mut self, enforce_schema: bool) -> &mut Self {
        self.enforce_schema = Some(enforce_schema);
        self
    }

    pub fn set_enforce_generic_value_type(
        &mut self,
        enforce_generic_value_type: bool,
    ) -> &mut Self {
        self.enforce_generic_value_type = Some(enforce_generic_value_type);
        self
    }

    pub fn build(&self) -> Self {
        self.clone()
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Pin {
    pub id: String,
    pub name: String,
    pub friendly_name: String,
    pub description: String,
    pub pin_type: PinType,
    pub data_type: VariableType,
    pub schema: Option<String>,
    pub valid_values: Option<Vec<String>>,
    pub value_type: ValueType,
    pub depends_on: HashSet<String>,
    pub connected_to: HashSet<String>,
    pub default_value: Option<Vec<u8>>,
    pub index: u16,
    pub options: Option<PinOptions>,

    // This will be set on execution, for execution it will be "Null"
    #[serde(skip)]
    pub value: Option<Arc<Mutex<Value>>>,
}

impl Pin {
    pub fn set_default_value(&mut self, default_value: Option<Value>) -> &mut Self {
        self.default_value = default_value.map(|v| flow_like_types::json::to_vec(&v).unwrap());
        self
    }

    pub fn set_value_type(&mut self, value_type: ValueType) -> &mut Self {
        self.value_type = value_type;
        self
    }

    pub fn set_data_type(&mut self, data_type: VariableType) -> &mut Self {
        self.data_type = data_type;
        self
    }

    pub fn set_schema<T: Serialize + JsonSchema>(&mut self) -> &mut Self {
        let schema = schema_for!(T);
        let schema_str = to_string_pretty(&schema).ok();
        self.schema = schema_str;
        self
    }

    pub fn reset_schema(&mut self) -> &mut Self {
        self.schema = None;
        self
    }

    pub fn set_options(&mut self, options: PinOptions) -> &mut Self {
        self.options = Some(options);
        self
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum ValueType {
    Array,
    Normal,
    HashMap,
    HashSet,
}

impl Pin {}

#[cfg(test)]
mod tests {

    use flow_like_types::sync::Mutex;
    use flow_like_types::{FromProto, ToProto};
    use flow_like_types::{Message, Value, tokio};
    use std::{collections::HashSet, sync::Arc};

    #[tokio::test]
    async fn serialize_pin() {
        let pin = super::Pin {
            id: "123".to_string(),
            name: "name".to_string(),
            friendly_name: "friendly_name".to_string(),
            description: "description".to_string(),
            pin_type: super::PinType::Input,
            data_type: super::VariableType::Execution,
            schema: None,
            valid_values: None,
            value_type: super::ValueType::Normal,
            depends_on: HashSet::new(),
            connected_to: HashSet::new(),
            default_value: None,
            index: 0,
            options: None,
            value: Some(Arc::new(Mutex::new(Value::Null))),
        };
        // let pin = super::SerializablePin::from(pin);

        let mut buf = Vec::new();
        pin.to_proto().encode(&mut buf).unwrap();
        let deser = super::Pin::from_proto(flow_like_types::proto::Pin::decode(&buf[..]).unwrap());

        assert_eq!(pin.id, deser.id);
    }
}
