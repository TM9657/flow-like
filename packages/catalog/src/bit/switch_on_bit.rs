use flow_like::{
    bit::{Bit, BitTypes},
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct SwitchOnBitNode {}

impl SwitchOnBitNode {
    pub fn new() -> Self {
        SwitchOnBitNode {}
    }
}

#[async_trait]
impl NodeLogic for SwitchOnBitNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "switch_on_bit",
            "Switch on Bit",
            "Routes execution based on the type of the Bit",
            "Bit",
        );

        node.add_icon("/flow/icons/bit.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("bit", "Bit", "Input Bit", VariableType::Struct)
            .set_schema::<Bit>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "llm",
            "LLM",
            "Execution if Bit is LLM",
            VariableType::Execution,
        );
        node.add_output_pin(
            "vlm",
            "VLM",
            "Execution if Bit is VLM",
            VariableType::Execution,
        );
        node.add_output_pin(
            "embedding",
            "Embedding",
            "Execution if Bit is Embedding",
            VariableType::Execution,
        );
        node.add_output_pin(
            "image_embedding",
            "Image Embedding",
            "Execution if Bit is ImageEmbedding",
            VariableType::Execution,
        );
        node.add_output_pin(
            "file",
            "File",
            "Execution if Bit is File",
            VariableType::Execution,
        );
        node.add_output_pin(
            "media",
            "Media",
            "Execution if Bit is Media",
            VariableType::Execution,
        );
        node.add_output_pin(
            "template",
            "Template",
            "Execution if Bit is Template",
            VariableType::Execution,
        );
        node.add_output_pin(
            "tokenizer",
            "Tokenizer",
            "Execution if Bit is Tokenizer",
            VariableType::Execution,
        );
        node.add_output_pin(
            "tokenizer_config",
            "Tokenizer Config",
            "Execution if Bit is TokenizerConfig",
            VariableType::Execution,
        );
        node.add_output_pin(
            "special_tokens_map",
            "Special Tokens Map",
            "Execution if Bit is SpecialTokensMap",
            VariableType::Execution,
        );
        node.add_output_pin(
            "config",
            "Config",
            "Execution if Bit is Config",
            VariableType::Execution,
        );
        node.add_output_pin(
            "course",
            "Course",
            "Execution if Bit is Course",
            VariableType::Execution,
        );
        node.add_output_pin(
            "preprocessor_config",
            "Preprocessor Config",
            "Execution if Bit is PreprocessorConfig",
            VariableType::Execution,
        );
        node.add_output_pin(
            "projection",
            "Projection",
            "Execution if Bit is Projection",
            VariableType::Execution,
        );
        node.add_output_pin(
            "project",
            "Project",
            "Execution if Bit is Project",
            VariableType::Execution,
        );
        node.add_output_pin(
            "board",
            "Board",
            "Execution if Bit is Board",
            VariableType::Execution,
        );
        node.add_output_pin(
            "other",
            "Other",
            "Execution if Bit is Other",
            VariableType::Execution,
        );

        node.add_output_pin("bit_out", "Bit", "Output Bit", VariableType::Struct)
            .set_schema::<Bit>();

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let bit: Bit = context.evaluate_pin("bit").await?;

        context.set_pin_value("bit_out", json!(bit)).await?;

        let output_pin = match bit.bit_type {
            BitTypes::Llm => "llm",
            BitTypes::Vlm => "vlm",
            BitTypes::Embedding => "embedding",
            BitTypes::ImageEmbedding => "image_embedding",
            BitTypes::File => "file",
            BitTypes::Media => "media",
            BitTypes::Template => "template",
            BitTypes::Tokenizer => "tokenizer",
            BitTypes::TokenizerConfig => "tokenizer_config",
            BitTypes::SpecialTokensMap => "special_tokens_map",
            BitTypes::Config => "config",
            BitTypes::Course => "course",
            BitTypes::PreprocessorConfig => "preprocessor_config",
            BitTypes::Projection => "projection",
            BitTypes::Project => "project",
            BitTypes::Board => "board",
            BitTypes::Other => "other",
        };

        context.activate_exec_pin(output_pin).await?;
        Ok(())
    }
}
