use crate::{
    response::{Choice, FunctionCall, Response, ResponseFunction, ResponseMessage, Usage},
    response_chunk::{Delta, ResponseChunk, ResponseChunkChoice},
};
use aws_sdk_bedrockruntime::operation::converse::ConverseOutput;
use aws_sdk_bedrockruntime::types::ConverseStreamOutput as ConverseStreamOutputType;

pub async fn convert_bedrock_response(
    converse_output: ConverseOutput,
) -> flow_like_types::Result<Response> {
    let mut response = Response::default();
    if let Some(usage) = converse_output.usage {
        response.usage.completion_tokens = usage.output_tokens as u32;
        response.usage.total_tokens = usage.total_tokens as u32;
        response.usage.prompt_tokens = usage.input_tokens as u32;
    }

    let mut content = String::new();

    if let Some(output) = converse_output.output
        && let Ok(message) = output.as_message()
    {
        let mut response_message = ResponseMessage::default();
        response_message.role = message.role.as_str().to_string();
        // let mut tool_calls = vec![];

        for chunk in message.content.iter() {
            if let Ok(text) = chunk.as_text() {
                content.push_str(text);
            }

            if let Ok(tool_call) = chunk.as_tool_result() {
                // TODO
            }
        }

        response.choices.push(Choice {
            index: 0,
            finish_reason: converse_output.stop_reason.as_str().to_string(),
            logprobs: None,
            message: response_message,
        });

        return Ok(response);
    }

    response.choices.push(Choice {
        index: 0,
        finish_reason: converse_output.stop_reason.as_str().to_string(),
        logprobs: None,
        message: ResponseMessage {
            ..Default::default()
        },
    });

    Ok(response)
}

pub fn convert_bedrock_stream_output(
    converse_output: ConverseStreamOutputType,
) -> flow_like_types::Result<ResponseChunk> {
    let mut response_chunk = ResponseChunk::default();
    match converse_output {
        ConverseStreamOutputType::ContentBlockDelta(val) => {
            response_chunk.id = "0".to_string();
            if let Some(delta) = val.delta() {
                if let Ok(text) = delta.as_text() {
                    response_chunk.choices.push(ResponseChunkChoice {
                        index: val.content_block_index,
                        delta: Some(Delta {
                            content: Some(text.to_string()),
                            role: None,
                            refusal: None,
                            tool_calls: None,
                        }),
                        finish_reason: None,
                        logprobs: None,
                    });
                }

                if let Ok(tool) = delta.as_tool_use() {
                    response_chunk.choices.push(ResponseChunkChoice {
                        index: val.content_block_index,
                        delta: Some(Delta {
                            content: None,
                            role: None,
                            refusal: None,
                            tool_calls: Some(vec![FunctionCall {
                                id: "".to_string(),
                                index: None,
                                function: ResponseFunction {
                                    name: "".to_string(),
                                    arguments: tool.input.clone(),
                                },
                                tool_type: None,
                            }]),
                        }),
                        finish_reason: None,
                        logprobs: None,
                    });
                }
            }
        }
        ConverseStreamOutputType::ContentBlockStart(start) => {
            response_chunk.id = "0".to_string();
            if let Some(start) = start.start
                && let Ok(tool) = start.as_tool_use()
            {
                response_chunk.choices.push(ResponseChunkChoice {
                    logprobs: None,
                    index: 0,
                    finish_reason: None,
                    delta: Some(Delta {
                        content: None,
                        role: None,
                        refusal: None,
                        tool_calls: Some(vec![FunctionCall {
                            id: tool.tool_use_id.clone(),
                            index: None,
                            function: ResponseFunction {
                                name: tool.name.clone(),
                                arguments: "".to_string(),
                            },
                            tool_type: None,
                        }]),
                    }),
                })
            }
        }
        ConverseStreamOutputType::ContentBlockStop(stop) => {
            response_chunk.choices.push(ResponseChunkChoice {
                logprobs: None,
                index: stop.content_block_index,
                finish_reason: None,
                delta: Some(Delta {
                    content: None,
                    role: None,
                    refusal: None,
                    tool_calls: None,
                }),
            });
        }
        ConverseStreamOutputType::MessageStart(message) => {
            response_chunk.choices.push(ResponseChunkChoice {
                logprobs: None,
                index: 0,
                finish_reason: None,
                delta: Some(Delta {
                    content: None,
                    role: Some(message.role.as_str().to_string()),
                    refusal: None,
                    tool_calls: None,
                }),
            });
        }
        ConverseStreamOutputType::MessageStop(stop) => {
            response_chunk.choices.push(ResponseChunkChoice {
                logprobs: None,
                index: 0,
                finish_reason: Some(stop.stop_reason.as_str().to_string()),
                delta: Some(Delta {
                    content: None,
                    role: None,
                    refusal: None,
                    tool_calls: None,
                }),
            });
        }
        ConverseStreamOutputType::Metadata(metadata) => {
            if let Some(usage) = metadata.usage {
                response_chunk.usage = Some(Usage {
                    completion_tokens: usage.output_tokens as u32,
                    prompt_tokens: usage.input_tokens as u32,
                    total_tokens: usage.total_tokens as u32,
                    ..Default::default()
                })
            }
        }
        _ => {
            println!("Unknown response type");
        }
    }
    Ok(response_chunk)
}
