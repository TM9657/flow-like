pub mod branch;
pub mod find_llm;
pub mod history;
pub mod invoke;
pub mod invoke_simple;
pub mod invoke_with_tools;
pub mod llm_extractor;
pub mod llm_extractor_history;
pub mod preferences;
pub mod provider;
pub mod response;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut nodes: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(find_llm::FindLLMNode::default()),
        Arc::new(invoke::InvokeLLM::default()),
        Arc::new(invoke_simple::InvokeLLMSimpleNode::default()),
        Arc::new(preferences::make::MakePreferencesNode::default()),
        Arc::new(preferences::hint::SetModelHintNode::default()),
        Arc::new(preferences::weight::SetWeightNode::default()),
        Arc::new(history::make::MakeHistoryNode::default()),
        Arc::new(history::extract_attachments::ExtractAttachments::default()),
        Arc::new(history::from_messages::FromMessagesNode::default()),
        Arc::new(history::message::make::MakeHistoryMessageNode::default()),
        Arc::new(history::message::push_content::PushContentNode::default()),
        Arc::new(history::message::extract_content::ExtractContentNode::default()),
        Arc::new(history::push_message::PushHistoryMessageNode::default()),
        Arc::new(history::pop_message::PopHistoryMessageNode::default()),
        Arc::new(history::clear::ClearHistoryNode::default()),
        Arc::new(history::get_system::GetSystemPromptNode::default()),
        Arc::new(history::set_system::SetSystemPromptMessageNode::default()),
        Arc::new(history::set_stream::SetHistoryStreamNode::default()),
        Arc::new(history::set_max_completion_tokens::SetHistoryMaxTokensNode::default()),
        Arc::new(history::set_top_p::SetHistoryTopPNode::default()),
        Arc::new(history::set_temperature::SetHistoryTemperatureNode::default()),
        Arc::new(history::set_seed::SetHistorySeedNode::default()),
        Arc::new(history::set_presence_penalty::SetHistoryPresencePenaltyNode::default()),
        Arc::new(history::set_frequency_penalty::SetHistoryFrequencyPenaltyNode::default()),
        Arc::new(history::set_user::SetHistoryUserNode::default()),
        Arc::new(history::set_stop::SetHistoryStopWordsNode::default()),
        Arc::new(history::set_response_format::SetHistoryResponseFormatNode::default()),
        Arc::new(history::set_n::SetHistoryNNode::default()),
        Arc::new(history::from_string::HistoryFromStringNode::default()),
        Arc::new(branch::LLMBranchNode::default()),
        Arc::new(llm_extractor::LLMExtractNode::default()),
        Arc::new(llm_extractor_history::LLMExtractHistoryNode::default()),
        Arc::new(invoke_with_tools::InvokeLLMWithToolsNode::default()),
        Arc::new(provider::build_anthropic::BuildAnthropicNode::default()),
        Arc::new(provider::build_cohere::BuildCohereNode::default()),
        Arc::new(provider::build_deepseek::BuildDeepseekNode::default()),
        Arc::new(provider::build_galadriel::BuildGaladrielNode::default()),
        Arc::new(provider::build_gemini::BuildGeminiNode::default()),
        Arc::new(provider::build_groq::BuildGroqNode::default()),
        Arc::new(provider::build_huggingface::BuildHuggingfaceNode::default()),
        Arc::new(provider::build_hyperbolic::BuildHyperbolicNode::default()),
        Arc::new(provider::build_mira::BuildMiraNode::default()),
        Arc::new(provider::build_mistral::BuildMistralNode::default()),
        Arc::new(provider::build_moonshot::BuildMoonshotNode::default()),
        Arc::new(provider::build_ollama::BuildOllamaNode::default()),
        Arc::new(provider::build_openai::BuildOpenAiNode::default()),
        Arc::new(provider::build_openrouter::BuildOpenRouterNode::default()),
        Arc::new(provider::build_perplexity::BuildPerplexityNode::default()),
        Arc::new(provider::build_together::BuildTogetherNode::default()),
        Arc::new(provider::build_voyageai::BuildVoyageAINode::default()),
        Arc::new(provider::build_xai::BuildXAINode::default()),
    ];

    // Add response nodes
    let response_nodes = response::register_functions().await;
    nodes.extend(response_nodes);

    nodes
}
