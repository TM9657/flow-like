pub mod find_llm;
pub mod invoke;
pub mod preferences;
pub mod history;
pub mod result;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(find_llm::FindLLMNode::default())),
        Arc::new(Mutex::new(invoke::InvokeLLM::default())),
        Arc::new(Mutex::new(preferences::make::MakePreferencesNode::default())),
        Arc::new(Mutex::new(preferences::hint::SetModelHintNode::default())),
        Arc::new(Mutex::new(preferences::weight::SetWeightNode::default())),
        Arc::new(Mutex::new(history::make::MakeHistoryNode::default())),
        Arc::new(Mutex::new(history::message::make::MakeHistoryMessageNode::default())),
        Arc::new(Mutex::new(history::message::push_content::PushContentNode::default())),
        Arc::new(Mutex::new(history::push_message::PushHistoryMessageNode::default())),
        Arc::new(Mutex::new(history::pop_message::PopHistoryMessageNode::default())),
        Arc::new(Mutex::new(history::clear::ClearHistoryNode::default())),
        Arc::new(Mutex::new(history::get_system::GetSystemPromptNode::default())),
        Arc::new(Mutex::new(history::set_system::SetSystemPromptMessageNode::default())),
        Arc::new(Mutex::new(history::set_stream::SetHistoryStreamNode::default())),
        Arc::new(Mutex::new(history::set_max_completion_tokens::SetHistoryMaxTokensNode::default())),
        Arc::new(Mutex::new(history::set_top_p::SetHistoryTopPNode::default())),
        Arc::new(Mutex::new(history::set_temperature::SetHistoryTemperatureNode::default())),
        Arc::new(Mutex::new(history::set_seed::SetHistorySeedNode::default())),
        Arc::new(Mutex::new(history::set_presence_penalty::SetHistoryPresencePenaltyNode::default())),
        Arc::new(Mutex::new(history::set_frequency_penalty::SetHistoryFrequencyPenaltyNode::default())),
        Arc::new(Mutex::new(history::set_user::SetHistoryUserNode::default())),
        Arc::new(Mutex::new(history::set_stop::SetHistoryStopWordsNode::default())),
        Arc::new(Mutex::new(history::set_response_format::SetHistoryResponseFormatNode::default())),
        Arc::new(Mutex::new(history::set_n::SetHistoryNNode::default())),

    ]
}
