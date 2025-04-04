use super::conversions::{FromProto, ToProto};
use crate::flow::{
    pin::{Pin, PinOptions, PinType, ValueType},
    variable::{Variable, VariableType},
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

impl ToProto<super::types::PinOptions> for PinOptions {
    fn to_proto(&self) -> super::types::PinOptions {
        super::types::PinOptions {
            valid_values: self.valid_values.clone().unwrap_or_default(),
            range_min: self.range.map_or(0.0, |r| r.0),
            range_max: self.range.map_or(0.0, |r| r.1),
            step: self.step.unwrap_or(0.0),
            enforce_schema: self.enforce_schema.unwrap_or(false),
            enforce_generic_value_type: self.enforce_generic_value_type.unwrap_or(false),
        }
    }
}

impl FromProto<super::types::PinOptions> for PinOptions {
    fn from_proto(proto: super::types::PinOptions) -> Self {
        PinOptions {
            valid_values: if proto.valid_values.is_empty() {
                None
            } else {
                Some(proto.valid_values)
            },
            range: if proto.range_min == 0.0 && proto.range_max == 0.0 {
                None
            } else {
                Some((proto.range_min, proto.range_max))
            },
            step: if proto.step == 0.0 {
                None
            } else {
                Some(proto.step)
            },
            enforce_schema: if proto.enforce_schema {
                Some(true)
            } else {
                None
            },
            enforce_generic_value_type: if proto.enforce_generic_value_type {
                Some(true)
            } else {
                None
            },
        }
    }
}

impl ToProto<super::types::Pin> for Pin {
    fn to_proto(&self) -> super::types::Pin {
        super::types::Pin {
            id: self.id.clone(),
            name: self.name.clone(),
            friendly_name: self.friendly_name.clone(),
            description: self.description.clone(),
            pin_type: self.pin_type.to_proto(),
            data_type: self.data_type.to_proto(),
            schema: self.schema.clone().unwrap_or_default(),
            valid_values: self.valid_values.clone().unwrap_or_default(),
            value_type: self.value_type.to_proto(),
            depends_on: self.depends_on.iter().cloned().collect(),
            connected_to: self.connected_to.iter().cloned().collect(),
            default_value: self.default_value.clone().unwrap_or_default(),
            index: self.index as u32,
            options: self.options.as_ref().map(|o| o.to_proto()),
        }
    }
}

impl FromProto<super::types::Pin> for Pin {
    fn from_proto(proto: super::types::Pin) -> Self {
        Pin {
            id: proto.id,
            name: proto.name,
            friendly_name: proto.friendly_name,
            description: proto.description,
            pin_type: PinType::from_proto(proto.pin_type),
            data_type: VariableType::from_proto(proto.data_type),
            schema: if proto.schema.is_empty() {
                None
            } else {
                Some(proto.schema)
            },
            valid_values: if proto.valid_values.is_empty() {
                None
            } else {
                Some(proto.valid_values)
            },
            value_type: ValueType::from_proto(proto.value_type),
            depends_on: proto.depends_on.into_iter().collect(),
            connected_to: proto.connected_to.into_iter().collect(),
            default_value: if proto.default_value.is_empty() {
                None
            } else {
                Some(proto.default_value)
            },
            index: proto.index as u16,
            options: proto.options.map(PinOptions::from_proto),
            value: None,
        }
    }
}
