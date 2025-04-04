use crate::utils::data_url::optimize_data_url;
use flow_like_model_provider::history::{Content, History, ResponseFormat, Role};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalModelHistoryMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalModelHistory {
    model: String,
    messages: Vec<LocalModelHistoryMessage>,
    stream: Option<bool>,
    max_completion_tokens: Option<u32>,
    top_p: Option<f32>,
    temperature: Option<f32>,
    seed: Option<u32>,
    presence_penalty: Option<f32>,
    user: Option<String>,
    stop: Option<Vec<String>>,
    response_format: Option<ResponseFormat>,
}

impl LocalModelHistory {
    pub async fn from_history(history: &History) -> Self {
        LocalModelHistory {
            model: history.model.clone(),
            messages: parse_messages(history).await,
            stream: history.stream,
            max_completion_tokens: history.max_completion_tokens,
            top_p: history.top_p,
            temperature: history.temperature,
            seed: history.seed,
            presence_penalty: history.presence_penalty,
            user: history.user.clone(),
            stop: history.stop.clone(),
            response_format: history.response_format.clone(),
        }
    }
}

async fn parse_messages(history: &History) -> Vec<LocalModelHistoryMessage> {
    let mut messages = Vec::with_capacity(history.messages.len());
    for message in &history.messages {
        let role = match message.role {
            Role::Assistant => "assistant",
            Role::User => "user",
            Role::System => "system",
        };

        let mut message_string = String::new();
        let mut img_counter = 0;

        for content in &message.content {
            match content {
                Content::Text { text, .. } => {
                    message_string.push_str(text);
                    message_string.push(' ');
                }
                Content::Image { data, .. } => {
                    let optimized_url = optimize_data_url(data).await;

                    if let Ok(data_url) = optimized_url {
                        img_counter += 1;
                        message_string.push_str(&format!("![img: {}]({})", img_counter, data_url));
                    }
                }
            }
        }

        messages.push(LocalModelHistoryMessage {
            role: role.to_string(),
            content: message_string,
        });
    }
    messages
}
