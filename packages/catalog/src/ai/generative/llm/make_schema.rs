/// # Make OpenAI Function Node
/// Function call definitions or JSON Schemas are tedious to write by hand so this is an utility node to help you out.
/// Node execution can fail if the LLM produces an output that cannot be parsed as JSON schema.
/// If node execution succeeds, however, the output is *guaranteed* to be a valid OpenAI-like Function Call Definition with valid JSON schema in the "parameters" section.
use crate::ai::generative::llm::invoke_with_tools::extract_tagged;
use flow_like::{
    bit::Bit,
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::history::{
    History, HistoryFunction, HistoryMessage, Role, Tool, ToolType,
};
use flow_like_types::{anyhow, async_trait, json};

const SYSTEM_PROMPT: &str = r#"
# Instructions
You are a function call json schema generator.

Given the user's input, infer a matching function call json schema.

At every input, reply with valid function call json schema.

Do not ask follow-up questions.

Wrap function call json schema in <schema></schema> xml tags.

# Required Response Format
<schema>
{
  "name": "<name of the function>",
  "description": "<description of the function>",
  "parameters": {
    "type": "object",
    "properties": {
        "<property name>": {
            "type": "<property type>",
            "description": "<property description>"
        },
        ...
    },
    "required": [<list of required property args>]
  }
}
</schema>

# Example
user:
call a weather api, with location arg for city or country and units temperature

output:
<schema>
{
  "name": "get_weather",
  "description": "Retrieves current weather for the given location.",
  "parameters": {
    "type": "object",
    "properties": {
      "location": {
        "type": "string",
        "description": "City and country e.g. Bogotá, Colombia"
      },
      "units": {
        "type": "string",
        "enum": [
          "celsius",
          "fahrenheit"
        ],
        "description": "Units the temperature will be returned in."
      }
    },
    "required": [
      "location",
      "units"
    ]
  }
}
</schema>

# Important Instructions
- You **MUST** wrap your json data in xml tags <schema></schema>
"#;

#[derive(Default)]
pub struct LLMMakeSchema {}

impl LLMMakeSchema {
    pub fn new() -> Self {
        LLMMakeSchema {}
    }
}

#[async_trait]
impl NodeLogic for LLMMakeSchema {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "llm_make_schema",
            "Make Tool Schema",
            "Generate Tool Definitions for Tool Calls",
            "AI/Generative",
        );
        node.add_icon("/flow/icons/bot-invoke.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin("model", "Model", "Model", VariableType::Struct)
            .set_schema::<Bit>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "prompt",
            "Prompt",
            "Input message with hints how to generate the schema.",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Execution Output",
            "Execution Output",
            VariableType::Execution,
        );

        node.add_output_pin(
            "tool",
            "Tool",
            "Generated Tool Definition",
            VariableType::Struct,
        )
        .set_schema::<Tool>();

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        // fetch inputs
        context.deactivate_exec_pin("exec_out").await?;
        let model = context.evaluate_pin::<Bit>("model").await?;
        let prompt: String = context.evaluate_pin("prompt").await?;

        // construct system prompt + history
        let mut model_name = model.id.clone();
        if let Some(meta) = model.meta.get("en") {
            model_name = meta.name.clone();
        }
        let mut history = History::new(model_name.clone(), vec![]);
        history.set_system_prompt(SYSTEM_PROMPT.to_string());
        history.push_message(HistoryMessage::from_string(Role::User, &prompt));

        // generate response
        let response = {
            let model_factory = context.app_state.lock().await.model_factory.clone();
            let model = model_factory
                .lock()
                .await
                .build(&model, context.app_state.clone())
                .await?;
            model.invoke(&history, None).await?
        }; // drop model

        // parse response
        let mut response_string = "".to_string();
        if let Some(response) = response.last_message() {
            response_string = response.content.clone().unwrap_or("".to_string());
        }
        context.log_message(&response_string, LogLevel::Debug);

        // parse + validate function defintion
        let mut schema_str = extract_tagged(&response_string, "schema")?;
        let schema_str = if !schema_str.is_empty() {
            // account for reasoning models which might produce schema tags multiple times
            // we are assuming that the last occurance of schema is the actual one
            schema_str.pop().unwrap()
        } else {
            return Err(anyhow!(format!(
                "Invalid number of schemas: Expected 1, got {}",
                schema_str.len()
            )));
        };

        // attempt to load as HistoryFunction
        let function: HistoryFunction = match json::from_str(&schema_str) {
            Ok(value) => value,
            Err(e) => {
                return Err(anyhow!(
                    "Failed to parse as tool function from llm response: {e:?}"
                ));
            }
        };
        // wrap as Tool object
        let tool = Tool {
            tool_type: ToolType::Function,
            function,
        };

        // set outputs
        context.set_pin_value("tool", json::json!(tool)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
