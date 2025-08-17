/// # Parse With Schema Node
/// Like utils > types > From String Node but with additional schema validation
/// Input strings must not only by JSON-serializable but also follow the provided schema
/// Schema definitions can either be JSON schemas or OpenAI function definitions.
/// Produces detailed error messages in case of violation.
/// Additionally, this module bundles JSON- and schema-related utility functions.
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{Error, Value, anyhow, async_trait, json, jsonschema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAIFunction {
    pub r#type: String,
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub strict: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAIToolCall {
    pub name: String,
    pub args: Value,
}

/// Is this JSON Value an OpenAI Function Definition?
/// https://platform.openai.com/docs/guides/function-calling?api-mode=responses#defining-functions
pub fn is_openai(data: &Value) -> bool {
    let map = match data.as_object() {
        Some(m) => m,
        None => return false,
    };

    // check "type" is "function"
    if map.get("type") != Some(&Value::String("function".into())) {
        return false;
    }

    // check required fields
    for key in ["name", "description", "parameters", "strict"] {
        if map.get(key).is_none_or(Value::is_null) {
            return false;
        }
    }
    true
}

/// Validates JSON against OpenAI Function Definition Schema
/// https://platform.openai.com/docs/guides/function-calling?api-mode=responses#defining-functions
/// Returns parsed JSON Value if valid
pub fn validate_openai_function(function: &OpenAIFunction) -> Result<&OpenAIFunction, Error> {
    // "type" should always be function
    if function.r#type != "function" {
        return Err(anyhow!("As of now, 'type' should always be 'function'"));
    }

    // check whether 'parameters' is valid JSON schema
    let parameters = &function.parameters;
    if !jsonschema::meta::is_valid(parameters) {
        return Err(anyhow!(
            "'parameters' value is not a valid JSON schema definition"
        ));
    }

    // todo: add strict option to ensure all properties in parameters have descriptions
    Ok(function)
}

pub fn validate_openai_functions(
    functions: &Vec<OpenAIFunction>,
) -> Result<&Vec<OpenAIFunction>, Error> {
    for function in functions {
        validate_openai_function(function)?;
    }
    Ok(functions)
}

pub fn validate_openai_functions_str(functions: &str) -> Result<Vec<OpenAIFunction>, Error> {
    let functions: Vec<OpenAIFunction> = match json::from_str(functions) {
        Ok(functions) => functions,
        Err(e) => {
            return Err(anyhow!(format!(
                "Failed to load list of functions from input string: {}",
                e
            )));
        }
    };
    match validate_openai_functions(&functions) {
        Ok(_) => Ok(functions),
        Err(e) => Err(e),
    }
}

pub fn validate_openai_function_str(function: &str) -> Result<OpenAIFunction, Error> {
    // Is definition input pin value valid JSON?
    let function: OpenAIFunction = match json::from_str(function) {
        Ok(function) => function,
        Err(e) => {
            return Err(anyhow!(format!(
                "Failed to load definition/schema from input string: {}",
                e
            )));
        }
    };
    match validate_openai_function(&function) {
        Ok(_) => Ok(function),
        Err(e) => Err(e),
    }
}

/// Converts OpenAI Function Defintion to JSON Schema
/// Returns data as-is if not OpenAI
pub fn into_json_schema(data: Value) -> Result<Value, Error> {
    if is_openai(&data) {
        if let Some(obj) = data.as_object()
            && let Some(parameters) = obj.get("parameters")
            && let Some(mut parameters_obj) = parameters.as_object().cloned()
        {
            parameters_obj.remove("additionalProperties");
            let name = match obj.get("name").cloned() {
                Some(name) => name,
                None => Value::Null,
            };
            parameters_obj.insert("title".to_string(), name);
            return Ok(Value::Object(parameters_obj));
        }
        Err(anyhow!("Failed to convert OpenAI function to JSON Schema"))
    } else {
        Ok(data)
    }
}

/// Creates a JSON Schema Validator for a JSON Schema or OpenAI Function Definition
fn get_schema_validator(definition_str: &str) -> Result<jsonschema::Validator, Error> {
    // Is definition input pin value valid JSON?
    let definition: Value = match json::from_str(definition_str) {
        Ok(definition) => definition,
        Err(e) => {
            return Err(anyhow!(format!(
                "Failed to load definition/schema from input string: {}",
                e
            )));
        }
    };

    // Convert defintion into JSON Schema spec
    let schema = into_json_schema(definition)?;

    // Create Schema Validator
    let validator = match jsonschema::validator_for(&schema) {
        Ok(validator) => validator,
        Err(e) => return Err(anyhow!(format!("Failed to load schema validator: {}", e))),
    };
    Ok(validator)
}

/// Validates JSON data against JSON/OpenAI Schema and returns JSON data as Value
/// Returns a JSON Value that is compliant with the given schema
pub fn validate_json_data(schema: &str, data: &str) -> Result<Value, Error> {
    // Get schema validator
    let validator = get_schema_validator(schema)?;

    // Is data input pin value valid JSON?
    let data: Value = match json::from_str(data) {
        Ok(data) => data,
        Err(e) => return Err(anyhow!(format!("Failed to parse JSON data: {}", e))),
    };

    // Validate input data againts JSON schema
    let errors = validator.iter_errors(&data);
    let error_msg = errors
        .map(|e| format!("Error: {}, Location: {}", e, e.instance_path))
        .collect::<Vec<_>>()
        .join("\n");

    if error_msg.is_empty() {
        Ok(data)
    } else {
        Err(anyhow!(format!("Schema validation failed: {}", error_msg)))
    }
}

/// Validates a Tool Call string against a list of OpenAi-Functions
pub fn validate_openai_tool_call_str(
    functions: &Vec<OpenAIFunction>,
    tool_call: &str,
) -> Result<OpenAIToolCall, Error> {
    // Deserialize tool call
    let tool_call: OpenAIToolCall = match json::from_str(tool_call) {
        Ok(tool_call) => tool_call,
        Err(e) => return Err(anyhow!(format!("Failed to parse tool call: {}", e))),
    };

    for function in functions {
        if tool_call.name == function.name {
            let validator = match jsonschema::validator_for(&function.parameters) {
                Ok(validator) => validator,
                Err(e) => return Err(anyhow!(format!("Failed to load schema validator: {}", e))),
            };

            // Validate tool call agains function schema
            let errors = validator.iter_errors(&tool_call.args);
            let error_msg = errors
                .map(|e| format!("Error: {}, Location: {}", e, e.instance_path))
                .collect::<Vec<_>>()
                .join("\n");

            if error_msg.is_empty() {
                return Ok(tool_call);
            } else {
                return Err(anyhow!(format!("Invalid tool call args: {}", error_msg)));
            }
        }
    }
    Err(anyhow!(format!(
        "No matching function found for tool call: {}",
        tool_call.name
    )))
}

#[derive(Default)]
pub struct ParseWithSchema {}

impl ParseWithSchema {
    pub fn new() -> Self {
        ParseWithSchema {}
    }
}

#[async_trait]
impl NodeLogic for ParseWithSchema {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "parse_with_schema",
            "Parse JSON with Schema",
            "Parse JSON input Data With JSON/OpenAI Schema and Return Value",
            "Utils/JSON",
        );

        node.add_icon("/flow/icons/repair.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "schema",
            "Schema",
            "JSON Schema or OpenAI Function Definition",
            VariableType::String,
        );

        node.add_input_pin(
            "data",
            "Data",
            "JSON Input Data to be parsed",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Execution continues if parsing succeeds",
            VariableType::Execution,
        );

        node.add_output_pin(
            "parsed",
            "Parsed",
            "Parsed and Validated JSON",
            VariableType::Struct,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let definition_str: String = context.evaluate_pin("schema").await?;
        let data_str: String = context.evaluate_pin("data").await?;

        let validated = validate_json_data(&definition_str, &data_str)?;

        context.set_pin_value("parsed", validated).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
