use flow_like::flow::{
    board::Board,
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct TryTransformNode {}

impl TryTransformNode {
    pub fn new() -> Self {
        TryTransformNode {}
    }
}

#[async_trait]
impl NodeLogic for TryTransformNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "utils_types_try_transform",
            "Try Transform",
            "Tries to transform cast types.",
            "Utils/Types",
        );
        node.set_flowscript_name("types", "tryTransform");

        node.add_input_pin(
            "type_in",
            "Type In",
            "Type to transform",
            VariableType::Generic,
        );

        node.add_output_pin(
            "type_out",
            "Type Out",
            "If the type was successfully transformed, transformed type",
            VariableType::Generic,
        );

        node.add_output_pin(
            "success",
            "Success",
            "Determines of tje transformation was successful",
            VariableType::Boolean,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let input_value: Value = context.evaluate_pin("type_in").await?;
        let output_value = context.get_pin_by_name("type_out").await?;
        let out_type = output_value.data_type.clone();
        let mut out_value: Value = Value::Null;

        let success = match out_type {
            VariableType::String => value_to_string(&input_value, &mut out_value),
            VariableType::Float => value_to_float(&input_value, &mut out_value),
            VariableType::Integer => value_to_int(&input_value, &mut out_value),
            VariableType::Boolean => value_to_boolean(&input_value, &mut out_value),
            VariableType::Struct => value_to_struct(&input_value, &mut out_value),
            VariableType::Byte => value_to_byte(&input_value, &mut out_value),
            VariableType::Date => value_to_date(&input_value, &mut out_value),
            VariableType::PathBuf => value_to_pathbuf(&input_value, &mut out_value),
            VariableType::Geometry => {
                let kind = flow_like::flow::variable::geometry_kind_from_schema(
                    output_value.schema.as_deref(),
                )?;
                match flow_like_types::geometry::canonicalize_geometry(&input_value, kind) {
                    Ok(value) => {
                        out_value = value;
                        true
                    }
                    Err(_) => false,
                }
            }
            VariableType::Execution => false,
            VariableType::Generic => false,
        };

        if success || out_type != VariableType::Geometry {
            context.set_pin_value("type_out", out_value).await?;
        } else {
            output_value.reset().await;
        }
        context.set_pin_value("success", json!(success)).await?;

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: &Board) {
        match_output_type(node, board);
        let _ = node.match_type("type_in", board, None, None);
    }
}

// Generic consumers impose no target type. When ordinary matching selects a Generic
// peer, use concrete consumers if their contracts agree. Keep legacy conflict handling.
fn match_output_type(node: &mut Node, board: &Board) {
    let _ = node.match_type("type_out", board, None, None);
    let Some(output) = node.get_pin_by_name("type_out") else {
        return;
    };
    if output.data_type != VariableType::Generic {
        return;
    }

    let mut inferred = None;
    for peer in output
        .connected_to
        .iter()
        .filter_map(|id| board.get_pin_by_id(id))
        .filter(|pin| pin.data_type != VariableType::Generic)
    {
        let schema = peer
            .schema
            .as_deref()
            .map(|schema| board.refs.get(schema).map(String::as_str).unwrap_or(schema))
            .filter(|schema| !flow_like::flow::pin::is_open_object_schema(schema))
            .map(str::to_owned);
        let Some((data_type, value_type, inferred_schema)) = inferred.as_mut() else {
            inferred = Some((peer.data_type.clone(), peer.value_type.clone(), schema));
            continue;
        };
        if *data_type != peer.data_type || *value_type != peer.value_type {
            return;
        }
        match (&*inferred_schema, schema) {
            (Some(current), Some(next)) if *current != next => return,
            (None, Some(next)) => *inferred_schema = Some(next),
            _ => {}
        }
    }

    let Some((data_type, value_type, schema)) = inferred else {
        return;
    };
    if output
        .connected_to
        .iter()
        .filter_map(|id| board.get_pin_by_id(id))
        .any(|peer| {
            peer.data_type == VariableType::Generic
                && (peer.value_type != flow_like::flow::pin::ValueType::Normal
                    || peer
                        .options
                        .as_ref()
                        .and_then(|options| options.enforce_generic_value_type)
                        == Some(true))
                && peer.value_type != value_type
        })
    {
        return;
    }
    if let Some(output) = node.get_pin_mut_by_name("type_out") {
        output.data_type = data_type;
        output.value_type = value_type;
        output.schema = schema;
    }
}

fn value_to_string(input: &Value, target: &mut Value) -> bool {
    if input.is_string() {
        *target = input.clone();
        return true;
    }

    if let Ok(val) = flow_like_types::json::to_string_pretty(input) {
        *target = Value::String(val);
        return true;
    }

    false
}

fn value_to_float(input: &Value, target: &mut Value) -> bool {
    if input.is_number() {
        if let Some(val) = input.as_f64() {
            *target = Value::Number(flow_like_types::json::Number::from_f64(val).unwrap());
            return true;
        }

        if let Some(val) = input.as_i64() {
            *target = Value::Number(flow_like_types::json::Number::from_f64(val as f64).unwrap());
            return true;
        }
    }

    if input.is_string()
        && let Some(s) = input.as_str()
        && let Ok(val) = s.parse::<f64>()
        && let Some(num) = flow_like_types::json::Number::from_f64(val)
    {
        *target = Value::Number(num);
        return true;
    }

    if input.is_boolean() {
        let val = if input.as_bool().unwrap() { 1.0 } else { 0.0 };
        if let Some(num) = flow_like_types::json::Number::from_f64(val) {
            *target = Value::Number(num);
            return true;
        }
    }

    false
}

fn value_to_int(input: &Value, target: &mut Value) -> bool {
    if input.is_number() {
        if let Some(val) = input.as_i64() {
            *target = Value::Number(val.into());
            return true;
        }

        if let Some(val) = input.as_f64() {
            let int_val = val as i64;
            *target = Value::Number(int_val.into());
            return true;
        }
    }

    if input.is_string()
        && let Some(s) = input.as_str()
    {
        // Try parsing as i64 first
        if let Ok(val) = s.parse::<i64>() {
            *target = Value::Number(val.into());
            return true;
        }

        // Try parsing as f64 and then converting to i64
        if let Ok(val) = s.parse::<f64>() {
            *target = Value::Number((val as i64).into());
            return true;
        }
    }

    // Handle boolean conversion (true = 1, false = 0)
    if input.is_boolean() {
        let val = if input.as_bool().unwrap() { 1 } else { 0 };
        *target = Value::Number(val.into());
        return true;
    }

    false
}

fn value_to_boolean(input: &Value, target: &mut Value) -> bool {
    if input.is_boolean() {
        *target = input.clone();
        return true;
    }

    if input.is_number() {
        if let Some(val) = input.as_f64() {
            *target = Value::Bool(val != 0.0);
            return true;
        }

        if let Some(val) = input.as_i64() {
            *target = Value::Bool(val != 0);
            return true;
        }
    }

    if input.is_string()
        && let Some(s) = input.as_str()
    {
        let lower = s.to_lowercase();
        if ["true", "yes", "y", "1", "on"].contains(&lower.as_str()) {
            *target = Value::Bool(true);
            return true;
        }

        if ["false", "no", "n", "0", "off"].contains(&lower.as_str()) {
            *target = Value::Bool(false);
            return true;
        }

        if let Ok(val) = s.parse::<f64>() {
            *target = Value::Bool(val != 0.0);
            return true;
        }
    }

    if input.is_null() {
        *target = Value::Bool(false);
        return true;
    }

    if input.is_array() {
        *target = Value::Bool(!input.as_array().unwrap().is_empty());
        return true;
    }

    if input.is_object() {
        *target = Value::Bool(!input.as_object().unwrap().is_empty());
        return true;
    }

    false
}

fn value_to_struct(input: &Value, target: &mut Value) -> bool {
    if input.is_object() {
        *target = input.clone();
        return true;
    }

    if input.is_string()
        && let Some(s) = input.as_str()
    {
        // Parse string as JSON
        if let Ok(parsed) = flow_like_types::json::from_str::<Value>(s)
            && parsed.is_object()
        {
            *target = parsed;
            return true;
        }
    }

    if input.is_array() {
        let array = input.as_array().unwrap();
        let mut obj = flow_like_types::json::Map::new();

        for (index, value) in array.iter().enumerate() {
            obj.insert(index.to_string(), value.clone());
        }

        *target = Value::Object(obj);
        return true;
    }

    if input.is_number() || input.is_boolean() || input.is_null() {
        let mut obj = flow_like_types::json::Map::new();
        obj.insert("value".to_string(), input.clone());
        *target = Value::Object(obj);
        return true;
    }

    false
}

fn value_to_byte(input: &Value, target: &mut Value) -> bool {
    if input.is_number() {
        if let Some(val) = input.as_i64()
            && (0..=255).contains(&val)
        {
            *target = Value::Number((val as u8).into());
            return true;
        }

        if let Some(val) = input.as_f64() {
            let byte_val = val.round() as i64;
            if (0..=255).contains(&byte_val) {
                *target = Value::Number((byte_val as u8).into());
                return true;
            }
        }
    }

    if input.is_string()
        && let Some(s) = input.as_str()
    {
        if let Ok(val) = s.parse::<u8>() {
            *target = Value::Number(val.into());
            return true;
        }

        if let Ok(val) = s.parse::<f64>() {
            let byte_val = val.round() as i64;
            if (0..=255).contains(&byte_val) {
                *target = Value::Number((byte_val as u8).into());
                return true;
            }
        }

        if s.chars().count() == 1 {
            let byte_val = s.chars().next().unwrap() as u32;
            if byte_val <= 255 {
                *target = Value::Number((byte_val as u8).into());
                return true;
            }
        }
    }

    if input.is_boolean() {
        let val = if input.as_bool().unwrap() { 1_u8 } else { 0_u8 };
        *target = Value::Number(val.into());
        return true;
    }

    false
}

fn value_to_date(input: &Value, target: &mut Value) -> bool {
    use chrono::{DateTime, Utc};

    // If input is already a string (RFC3339), validate and use it
    if input.is_string()
        && let Some(s) = input.as_str()
    {
        // Try parsing as RFC3339 to validate
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            let dt_utc: DateTime<Utc> = dt.with_timezone(&Utc);
            *target = json!(dt_utc);
            return true;
        }

        // Try other common formats and convert to DateTime<Utc>
        for format in &["%Y-%m-%d", "%d/%m/%Y", "%m/%d/%Y", "%Y-%m-%d %H:%M:%S"] {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, format) {
                let dt_utc = dt.and_utc();
                *target = json!(dt_utc);
                return true;
            } else if let Ok(d) = chrono::NaiveDate::parse_from_str(s, format) {
                let dt = d.and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                let dt_utc = dt.and_utc();
                *target = json!(dt_utc);
                return true;
            }
        }

        return false;
    }

    // Convert numbers to RFC3339 strings
    if input.is_number() {
        let dt_utc: Option<DateTime<Utc>>;

        if let Some(val) = input.as_i64() {
            // If value is greater than year 2000 in milliseconds, treat as milliseconds
            if val > 946684800 * 1000 {
                dt_utc = DateTime::from_timestamp_millis(val);
            } else {
                // Treat as seconds
                dt_utc = DateTime::from_timestamp(val, 0);
            }
        } else if let Some(val) = input.as_f64() {
            if val > 946684800.0 * 1000.0 {
                // Treat as milliseconds
                dt_utc = DateTime::from_timestamp_millis(val as i64);
            } else {
                // Treat as seconds with fractional part as nanos
                let secs = val.floor() as i64;
                let nanos = (val.fract() * 1_000_000_000.0) as u32;
                dt_utc = DateTime::from_timestamp(secs, nanos);
            }
        } else {
            return false;
        }

        if let Some(dt) = dt_utc {
            *target = json!(dt);
            return true;
        }

        return false;
    }

    // Handle old SystemTime format for backward compatibility
    if input.is_object() {
        let obj = input.as_object().unwrap();
        if obj.contains_key("secs_since_epoch")
            && obj.contains_key("nanos_since_epoch")
            && let (Some(secs), Some(nanos)) = (
                obj.get("secs_since_epoch").and_then(|v| v.as_i64()),
                obj.get("nanos_since_epoch").and_then(|v| v.as_u64()),
            )
            && let Some(dt) = DateTime::<Utc>::from_timestamp(secs, nanos as u32)
        {
            *target = json!(dt);
            return true;
        }
    }

    false
}

fn value_to_pathbuf(input: &Value, target: &mut Value) -> bool {
    if input.is_string() {
        *target = input.clone();
        return true;
    }

    if input.is_object()
        && let Some(obj) = input.as_object()
    {
        if let Some(path) = obj.get("path")
            && path.is_string()
        {
            *target = path.clone();
            return true;
        }

        if let (Some(dir), Some(file)) = (obj.get("directory"), obj.get("filename"))
            && let (Some(dir_str), Some(file_str)) = (dir.as_str(), file.as_str())
        {
            let path = format!("{}{}{}", dir_str, std::path::MAIN_SEPARATOR, file_str);
            *target = Value::String(path);
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like::flow::pin::{OPEN_OBJECT_SCHEMA, ValueType};

    fn add_consumer(
        board: &mut Board,
        converter: &mut Node,
        pin_id: &str,
        data_type: VariableType,
        value_type: ValueType,
        schema: Option<&str>,
    ) {
        let mut consumer = Node::new("consumer", "Consumer", "", "Tests");
        consumer.id = format!("consumer_{pin_id}");
        let old_id = consumer
            .add_input_pin("value", "Value", "", data_type)
            .id
            .clone();
        let mut pin = consumer.pins.remove(&old_id).unwrap();
        pin.id = pin_id.to_string();
        pin.value_type = value_type;
        pin.schema = schema.map(str::to_owned);
        let output = converter.get_pin_mut_by_name("type_out").unwrap();
        pin.depends_on.insert(output.id.clone());
        output.connected_to.insert(pin.id.clone());
        consumer.pins.insert(pin.id.clone(), pin);
        board.nodes.insert(consumer.id.clone(), consumer);
    }

    #[test]
    fn generic_consumer_cannot_mask_an_unambiguous_string_consumer() {
        // Run 4 sorted its Struct Set consumer before the typed helper argument.
        // Run 6 sorted the helper first. Both must infer the same output contract.
        for (generic_id, string_id) in [("a", "z"), ("z", "a")] {
            let mut board = Board::new_detached(None, Default::default());
            let mut converter = TryTransformNode::new().get_node();
            add_consumer(
                &mut board,
                &mut converter,
                generic_id,
                VariableType::Generic,
                ValueType::Normal,
                None,
            );
            add_consumer(
                &mut board,
                &mut converter,
                string_id,
                VariableType::String,
                ValueType::Normal,
                None,
            );
            let original_edges = converter
                .get_pin_by_name("type_out")
                .unwrap()
                .connected_to
                .clone();

            match_output_type(&mut converter, &board);

            let output = converter.get_pin_by_name("type_out").unwrap();
            assert_eq!(output.data_type, VariableType::String);
            assert_eq!(output.value_type, ValueType::Normal);
            assert_eq!(output.schema, None);
            assert_eq!(output.connected_to, original_edges);
        }
    }

    #[test]
    fn output_inference_treats_open_schemas_as_unspecified() {
        const CONCRETE: &str = r#"{"type":"object","properties":{"summary":{"type":"string"}}}"#;
        for (open_id, concrete_id) in [("b", "c"), ("c", "b")] {
            let mut board = Board::new_detached(None, Default::default());
            board.refs.insert("concrete-schema".into(), CONCRETE.into());
            let mut converter = TryTransformNode::new().get_node();
            add_consumer(
                &mut board,
                &mut converter,
                "a",
                VariableType::Generic,
                ValueType::Normal,
                None,
            );
            add_consumer(
                &mut board,
                &mut converter,
                open_id,
                VariableType::Struct,
                ValueType::Normal,
                Some(OPEN_OBJECT_SCHEMA),
            );
            add_consumer(
                &mut board,
                &mut converter,
                concrete_id,
                VariableType::Struct,
                ValueType::Normal,
                Some("concrete-schema"),
            );

            match_output_type(&mut converter, &board);

            let output = converter.get_pin_by_name("type_out").unwrap();
            assert_eq!(output.data_type, VariableType::Struct);
            assert_eq!(output.value_type, ValueType::Normal);
            assert_eq!(output.schema.as_deref(), Some(CONCRETE));
        }
    }

    #[test]
    fn output_inference_preserves_legacy_ambiguous_contract_behavior() {
        for generic_id in ["a", "z"] {
            for (
                left_type,
                left_container,
                left_schema,
                right_type,
                right_container,
                right_schema,
            ) in [
                (
                    VariableType::String,
                    ValueType::Normal,
                    None,
                    VariableType::Integer,
                    ValueType::Normal,
                    None,
                ),
                (
                    VariableType::String,
                    ValueType::Normal,
                    None,
                    VariableType::String,
                    ValueType::Array,
                    None,
                ),
                (
                    VariableType::Struct,
                    ValueType::Normal,
                    Some(r#"{"type":"object","required":["left"]}"#),
                    VariableType::Struct,
                    ValueType::Normal,
                    Some(r#"{"type":"object","required":["right"]}"#),
                ),
            ] {
                let mut board = Board::new_detached(None, Default::default());
                let mut converter = TryTransformNode::new().get_node();
                add_consumer(
                    &mut board,
                    &mut converter,
                    generic_id,
                    VariableType::Generic,
                    ValueType::Normal,
                    None,
                );
                add_consumer(
                    &mut board,
                    &mut converter,
                    "b",
                    left_type,
                    left_container,
                    left_schema,
                );
                add_consumer(
                    &mut board,
                    &mut converter,
                    "c",
                    right_type,
                    right_container,
                    right_schema,
                );
                let mut legacy = converter.clone();
                legacy.match_type("type_out", &board, None, None).unwrap();

                match_output_type(&mut converter, &board);

                assert_eq!(
                    flow_like_types::json::to_value(converter.get_pin_by_name("type_out").unwrap())
                        .unwrap(),
                    flow_like_types::json::to_value(legacy.get_pin_by_name("type_out").unwrap())
                        .unwrap(),
                    "conflicting concrete contracts keep the previous matching behavior"
                );
            }
        }
    }

    #[test]
    fn output_inference_preserves_generic_container_constraints() {
        for leading_unconstrained_peer in [false, true] {
            let mut board = Board::new_detached(None, Default::default());
            let mut converter = TryTransformNode::new().get_node();
            if leading_unconstrained_peer {
                add_consumer(
                    &mut board,
                    &mut converter,
                    "a",
                    VariableType::Generic,
                    ValueType::Normal,
                    None,
                );
            }
            add_consumer(
                &mut board,
                &mut converter,
                "b",
                VariableType::Generic,
                ValueType::Array,
                None,
            );
            add_consumer(
                &mut board,
                &mut converter,
                "z",
                VariableType::String,
                ValueType::Normal,
                None,
            );
            let mut legacy = converter.clone();
            legacy.match_type("type_out", &board, None, None).unwrap();

            match_output_type(&mut converter, &board);

            assert_eq!(
                flow_like_types::json::to_value(converter.get_pin_by_name("type_out").unwrap())
                    .unwrap(),
                flow_like_types::json::to_value(legacy.get_pin_by_name("type_out").unwrap())
                    .unwrap(),
                "the scalar consumer must not override a Generic/Array contract"
            );
        }
    }

    #[test]
    fn output_inference_without_a_concrete_consumer_stays_generic() {
        for has_generic_consumer in [false, true] {
            let mut board = Board::new_detached(None, Default::default());
            let mut converter = TryTransformNode::new().get_node();
            if has_generic_consumer {
                add_consumer(
                    &mut board,
                    &mut converter,
                    "a",
                    VariableType::Generic,
                    ValueType::Normal,
                    None,
                );
            }
            match_output_type(&mut converter, &board);
            assert_eq!(
                converter.get_pin_by_name("type_out").unwrap().data_type,
                VariableType::Generic
            );
        }
    }
}
