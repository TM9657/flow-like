use crate::flow::{
    pin::{Pin, PinOptions, PinType, ValueType},
    variable::VariableType,
};
use flow_like_types::{FromProto, ToProto};

impl ToProto<flow_like_types::proto::PinOptions> for PinOptions {
    fn to_proto(&self) -> flow_like_types::proto::PinOptions {
        flow_like_types::proto::PinOptions {
            valid_values: self.valid_values.clone().unwrap_or_default(),
            range_min: self.range.map(|(min, _)| min),
            range_max: self.range.map(|(_, max)| max),
            step: self.step,
            enforce_schema: self.enforce_schema,
            enforce_generic_value_type: self.enforce_generic_value_type,
            sensitive: self.sensitive,
            optional: self.optional,
        }
    }
}

impl FromProto<flow_like_types::proto::PinOptions> for PinOptions {
    fn from_proto(proto: flow_like_types::proto::PinOptions) -> Self {
        PinOptions {
            valid_values: if proto.valid_values.is_empty() {
                None
            } else {
                Some(proto.valid_values)
            },
            // Boards stored before these fields tracked presence omit a zero end, so one end
            // alone is still a range.
            range: match (proto.range_min, proto.range_max) {
                (None, None) => None,
                (min, max) => Some((min.unwrap_or_default(), max.unwrap_or_default())),
            },
            step: proto.step,
            enforce_schema: proto.enforce_schema,
            enforce_generic_value_type: proto.enforce_generic_value_type,
            sensitive: proto.sensitive,
            optional: proto.optional,
        }
    }
}

impl ToProto<flow_like_types::proto::Pin> for Pin {
    fn to_proto(&self) -> flow_like_types::proto::Pin {
        flow_like_types::proto::Pin {
            id: self.id.clone(),
            name: self.name.clone(),
            friendly_name: self.friendly_name.clone(),
            description: self.description.clone(),
            pin_type: self.pin_type.to_proto(),
            data_type: self.data_type.to_proto(),
            schema: self.schema.clone().unwrap_or_default(),
            value_type: self.value_type.to_proto(),
            depends_on: self.depends_on.iter().cloned().collect(),
            connected_to: self.connected_to.iter().cloned().collect(),
            default_value: self.default_value.clone().unwrap_or_default(),
            index: self.index as u32,
            options: self.options.as_ref().map(|o| o.to_proto()),
        }
    }
}

impl FromProto<flow_like_types::proto::Pin> for Pin {
    fn from_proto(proto: flow_like_types::proto::Pin) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_types::Message;

    fn through_the_wire(options: &PinOptions) -> PinOptions {
        let bytes = options.to_proto().encode_to_vec();
        let proto = flow_like_types::proto::PinOptions::decode(bytes.as_slice())
            .expect("decode stored pin options");
        PinOptions::from_proto(proto)
    }

    #[test]
    fn pin_options_round_trip_through_proto() {
        let options = PinOptions {
            sensitive: Some(true),
            valid_values: Some(vec!["GET".into(), "POST".into()]),
            range: Some((-1.0, 1.0)),
            step: Some(0.25),
            enforce_schema: Some(false),
            enforce_generic_value_type: Some(true),
            optional: Some(false),
        };
        assert_eq!(through_the_wire(&options), options);
    }

    #[test]
    fn pin_options_keep_unset_false_and_zero_apart() {
        for flag in [None, Some(false), Some(true)] {
            let options = PinOptions {
                enforce_schema: flag,
                enforce_generic_value_type: flag,
                ..PinOptions::new()
            };
            assert_eq!(through_the_wire(&options), options);
        }

        let zeros = PinOptions {
            range: Some((0.0, 0.0)),
            step: Some(0.0),
            ..PinOptions::new()
        };
        assert_eq!(through_the_wire(&zeros), zeros);
        assert_eq!(through_the_wire(&PinOptions::new()), PinOptions::new());
    }

    #[test]
    fn pin_options_stored_before_presence_tracking_read_unchanged() {
        // range (0, 10) and enforce_schema = true as the old writer encoded them: the zero range
        // end, the zero step and the false flag were left off the wire.
        let stored = [0x19, 0, 0, 0, 0, 0, 0, 0x24, 0x40, 0x28, 0x01];
        let proto = flow_like_types::proto::PinOptions::decode(stored.as_slice())
            .expect("decode pin options stored by the old writer");
        let options = PinOptions::from_proto(proto);
        assert_eq!(options.range, Some((0.0, 10.0)));
        assert_eq!(options.enforce_schema, Some(true));
        assert_eq!(options.step, None);
        assert_eq!(options.enforce_generic_value_type, None);
    }

    #[test]
    fn pin_keeps_an_explicitly_disabled_schema_check() {
        let mut pin = crate::flow::node::Node::new("cast", "Cast", "", "")
            .add_output_pin(
                "struct_out",
                "Struct",
                "",
                crate::flow::variable::VariableType::Struct,
            )
            .clone();
        pin.set_options(PinOptions::new().set_enforce_schema(false).build());

        let bytes = pin.to_proto().encode_to_vec();
        let stored = flow_like_types::proto::Pin::decode(bytes.as_slice()).expect("decode pin");
        let options = Pin::from_proto(stored).options.expect("options survive");
        assert_eq!(options.enforce_schema, Some(false));
    }
}
