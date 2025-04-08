use aws_sdk_bedrockruntime::{
    primitives::Blob,
    types::{ContentBlock, ImageBlock, ImageFormat, ImageSource, Message},
};
use flow_like_types::utils::data_url::{data_url_to_bytes, make_data_url};

use crate::history::History;

impl History {
    pub async fn to_messages(&self) -> Vec<Message> {
        let mut messages = Vec::new();
        for message in &self.messages {
            let role = match message.role {
                crate::history::Role::User => aws_sdk_bedrockruntime::types::ConversationRole::User,
                crate::history::Role::Assistant => {
                    aws_sdk_bedrockruntime::types::ConversationRole::Assistant
                }
                _ => continue,
            };

            let mut content: Vec<ContentBlock> = Vec::new();
            for history_content in message.content.iter() {
                match history_content {
                    crate::history::Content::Text { text, .. } => {
                        content.push(ContentBlock::Text(text.clone()));
                    }
                    crate::history::Content::Image { data, .. } => {
                        let url = match make_data_url(data).await {
                            Ok(url) => url,
                            Err(err) => {
                                println!("Error creating data URL: {}", err);
                                continue;
                            }
                        };

                        let bytes = match data_url_to_bytes(&url).await {
                            Ok(bytes) => bytes,
                            Err(err) => {
                                println!("Error converting data URL to bytes: {}", err);
                                continue;
                            }
                        };

                        let blob = Blob::new(bytes);

                        let img_block = match ImageBlock::builder()
                            .set_format(Some(ImageFormat::Jpeg))
                            .set_source(Some(ImageSource::Bytes(blob)))
                            .build()
                        {
                            Ok(block) => block,
                            Err(err) => {
                                println!("Error creating image block: {}", err);
                                continue;
                            }
                        };

                        content.push(ContentBlock::Image(img_block));
                    }
                }
            }

            let message = match Message::builder()
                .set_role(Some(role))
                .set_content(Some(content))
                .build()
            {
                Ok(message) => message,
                Err(err) => {
                    println!("Error creating message: {}", err);
                    continue;
                }
            };
            messages.push(message);
        }
        messages
    }
}
