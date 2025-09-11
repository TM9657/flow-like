use super::response_chunk::{Delta, DeltaFunctionCall, ResponseChunk};
use flow_like_types::{
    JsonSchema,
    json::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct FunctionCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
    pub id: String,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    pub function: ResponseFunction,
}

//impl Default for FunctionCall {
//    fn default() -> Self {
//        FunctionCall {
//            index: None,
//            id: "".to_string(),
//            tool_type: None,
//            function: ResponseFunction {
//                name: None,
//                arguments: None,
//            },
//        }
//    }
//}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct ResponseFunction {
    //#[serde(skip_serializing_if = "Option::is_none")]
    pub name: String,
    //#[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct LogProbs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<TokenLogProbs>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<Vec<TokenLogProbs>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct TokenLogProbs {
    pub token: String,
    pub logprob: f64,
    pub bytes: Option<Vec<u8>>,
    pub top_logprobs: Option<Vec<TopLogProbs>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct TopLogProbs {
    pub token: String,
    pub logprob: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Choice {
    pub index: i32,
    pub finish_reason: String,
    pub message: ResponseMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<LogProbs>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Audio {
    pub data: String,
    pub expires_at: Option<u64>,
    pub id: String,
    pub transcript: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct ResponseMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<Audio>,

    //#[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Vec<FunctionCall>,
}

impl Default for ResponseMessage {
    fn default() -> Self {
        ResponseMessage {
            content: None,
            refusal: None,
            annotations: None,
            audio: None,
            tool_calls: vec![],
            role: "".to_string(),
        }
    }
}

impl ResponseMessage {
    pub fn apply_delta(&mut self, delta: Delta) {
        if let Some(content) = delta.content {
            self.content = Some(self.content.as_deref().unwrap_or("").to_string() + &content);
        }

        if let Some(refusal) = delta.refusal {
            self.refusal = Some(self.refusal.as_deref().unwrap_or("").to_string() + &refusal);
        }

        if let Some(role) = delta.role
            && role != self.role
        {
            self.role = self.role.to_string() + &role;
        }

        if let Some(tool_calls) = delta.tool_calls {
            for dcall in tool_calls.into_iter() {
                self.apply_delta_tool_call(dcall);
            }
        }
    }

    fn apply_delta_tool_call(&mut self, dcall: DeltaFunctionCall) {
        // Determine index (default to next position if missing)
        let idx = dcall.index;

        // Try to find existing entry by index when provided
        if let Some(i) = idx {
            if let Some(existing) = self.tool_calls.iter_mut().find(|c| c.index == Some(i)) {
                if let Some(id) = dcall.id { existing.id = id; }
                if let Some(t) = dcall.tool_type {
                    existing.tool_type = Some(existing.tool_type.as_deref().unwrap_or("").to_string() + &t);
                }
                if let Some(name) = dcall.function.name { existing.function.name += &name; }
                if let Some(args) = dcall.function.arguments { existing.function.arguments += &args; }
                return;
            }
        }

        // Create new entry, using empty strings for missing fields
        let index = idx;
        let id = dcall.id.unwrap_or_default();
        let tool_type = dcall.tool_type;
        let name = dcall.function.name.unwrap_or_default();
        let arguments = dcall.function.arguments.unwrap_or_default();
        self.tool_calls.push(FunctionCall {
            index,
            id,
            tool_type,
            function: ResponseFunction { name, arguments },
        });
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Default)]
pub struct Usage {
    pub completion_tokens: u32,
    pub prompt_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokenDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokenDetails>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct PromptTokenDetails {
    cached_tokens: u32,
    audio_tokens: u32,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct CompletionTokenDetails {
    accepted_prediction_tokens: u32,
    audio_tokens: u32,
    reasoning_tokens: u32,
    rejected_prediction_tokens: u32,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Annotation {
    r#type: String,
    url_citation: Option<UrlCitation>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct UrlCitation {
    end_index: u32,
    start_index: u32,
    title: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Default)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    pub usage: Usage,
}

impl Response {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last_message(&self) -> Option<&ResponseMessage> {
        self.choices.last().map(|c| &c.message)
    }

    pub fn push_chunk(&mut self, chunk: ResponseChunk) {
        // Update optional fields if present in the chunk
        if let Some(created) = chunk.created {
            self.created = Some(created);
        }

        if let Some(model) = chunk.model {
            self.model = Some(model);
        }

        if let Some(service_tier) = chunk.service_tier {
            self.service_tier = Some(service_tier);
        }

        if let Some(system_fingerprint) = chunk.system_fingerprint {
            self.system_fingerprint = Some(system_fingerprint);
        }

        if let Some(usage) = chunk.usage {
            self.usage.completion_tokens += usage.completion_tokens;
            self.usage.prompt_tokens += usage.prompt_tokens;
            self.usage.total_tokens += usage.total_tokens;
        }

        for choice in chunk.choices {
            // Check if a choice with the same index already exists
            if let Some(existing_choice) = self.choices.iter_mut().find(|c| c.index == choice.index)
            {
                // Update existing choice fields if present
                if let Some(delta) = choice.delta {
                    existing_choice.message.apply_delta(delta);
                }
                if let Some(logprobs) = choice.logprobs {
                    existing_choice.logprobs = Some(logprobs);
                }
                if let Some(finish_reason) = choice.finish_reason {
                    existing_choice.finish_reason = finish_reason;
                }

                return;
            }

            // Create a new choice if it doesn't exist
            let mut message = ResponseMessage::default();
            if let Some(delta) = choice.delta {
                message.apply_delta(delta);
            }

            self.choices.push(Choice {
                finish_reason: choice.finish_reason.unwrap_or_default(),
                index: choice.index,
                logprobs: choice.logprobs,
                message,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_types::json;

    #[test]
    fn deserialize_annotations_with_content() {
        let json_str = r#"{
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": "Here's the latest news I found: ...",
                    "annotations": [
                        {
                            "type": "url_citation",
                            "url_citation": {
                                "url": "https://www.example.com/web-search-result",
                                "title": "Title of the web search result",
                                "content": "Content of the web search result",
                                "start_index": 100,
                                "end_index": 200
                            }
                        }
                    ],
                    "tool_calls": []
                }
            }],
            "usage": {"completion_tokens":0, "prompt_tokens":0, "total_tokens":0}
        }"#;

        let resp: Response = json::from_str(json_str).expect("valid response json");
        let anns = resp
            .choices
            .first()
            .and_then(|c| c.message.annotations.as_ref())
            .expect("annotations present");
        assert_eq!(anns.len(), 1);

        // Ensure it deserializes rather than panics; structure fields are private by design.
        // We just check presence by re-serializing.
        let out = json::to_string(&resp).unwrap();
        assert!(out.contains("url_citation"));
        assert!(out.contains("content"));
    }
}
