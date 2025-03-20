use anyhow::Result;
use log::{info, debug};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::price::ModelPricing;

pub enum TokenType { 
    InputToken,
    OutputToken,
}

/// Roles for messages in the conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    #[serde(rename = "user")]
    User,

    #[serde(rename = "assistant")]
    Assistant,

    #[serde(rename = "system")]
    System,
}

/// Class for a Role's message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Anthropic API request structure
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}


/// --- Streaming --- ///

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart {
        message: StreamMessage,
    },

    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },

    ContentBlockDelta {
        index: usize,
        delta: Delta,
    },

    ContentBlockStop {
        index: usize,
    },

    MessageDelta {
        delta: MessageDelta,
        usage: Option<ResponseUsage>,
    },

    MessageStop,

    Ping,

    Error {
        error: StreamError,
    },
}

#[derive(Debug, Deserialize)]
pub struct StreamMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub usage: Option<ResponseUsage>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamError {
    pub message: String,
}

pub struct StreamingBuffer {
    pub content: String,
    pub is_complete: bool,
}


/// ---

/// Struct to get the number of tokens with the count_token endpoint 
#[deprecated]
#[derive(Debug, Serialize)]
struct AntTokCountRequest {
    model: String,
    messages: Vec<Message>,
}

#[deprecated]
#[derive(Debug, Deserialize)]
struct AntTokCountResponse {
    input_tokens: u32,
}

/// Content block in the anth API response
#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct ResponseUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}


/// Response structure from the anthropic API
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<ContentBlock>,
    usage: ResponseUsage,
}

#[derive(Debug)]
pub struct ExtractedResponse {
    pub content: String,
    pub usage: ResponseUsage,
}


/// Http client for requests to anth
#[derive(Clone)]
pub struct AnthropicClient {
    api_key: String,
    client: Arc<reqwest::Client>,
    model: String,
}


impl AnthropicClient {
    pub fn new(model: &str, api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30)) // timeout request in 30 sec
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            client: Arc::new(client),
            model: model.to_string()
        }
    }

    pub async fn is_api_key_valid(api_key: String) -> Result<bool> {
        const API_URL: &str = "https://api.anthropic.com/v1/models";

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5)) // shorter timeout for validation
            .build()
            .expect("Failed to create HTTP client");

        let response = client
            .get(API_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .send()
            .await?;

        let success = response.status().is_success();
        Ok(success)
    }

    // TODO: possible to use ref for Vec of messages ?
    pub async fn send_message(&self, messages: Vec<Message>) -> Result<ExtractedResponse> {
        const API_URL: &str = "https://api.anthropic.com/v1/messages";
        const MAX_TOKENS: u32 = 4096;

        let request = AnthropicRequest {
            model: self.model.clone(),
            messages,
            max_tokens: MAX_TOKENS,
            stream: None,
        };


        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        // let response_text = response.text().await?;
        // info!("Full response: {}", response_text);

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            let error_f = format!("API error ({}): {}", status, error_text);
            return Err(anyhow::anyhow!(error_f));
        }

        let anthropic_response: AnthropicResponse = response.json().await?;
        debug!("Received response: {:?}", anthropic_response);

        let mut full_content = String::new();
        for content_block in anthropic_response.content {
            if content_block.content_type == "text" {
                full_content.push_str(&content_block.text);
            }
        }

        Ok(
            ExtractedResponse { 
                content: full_content, 
                usage: anthropic_response.usage,
        })
    }

    pub async fn send_message_streaming(
        &self,
        messages: Vec<Message>,
    ) -> Result<impl futures_util::Stream<Item = Result<StreamingBuffer>>> {
        use futures_util::stream::{self, StreamExt};
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio_stream::wrappers::LinesStream;

        const API_URL: &str = "https://api.anthropic.com/v1/messages";
        const MAX_TOKENS: u32 = 4096;

        let request = AnthropicRequest {
            model: self.model.clone(),
            messages,
            max_tokens: MAX_TOKENS,
            stream: Some(true),
        };

        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            let error_f = format!("API error ({}): {}", status, error_text);
            return Err(anyhow::anyhow!(error_f));
        }

        let byte_stream = response.bytes_stream();
        let reader = BufReader::new(tokio_util::io::StreamReader::new(byte_stream.map(
            |result| result.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
        )));

        let lines_stream = LinesStream::new(reader.lines());

        let event_stream = lines_stream.filter_map(|line_result| async move {
            let line = match line_result {
                Ok(line) => line,
                Err(e) => return Some(Err(anyhow::anyhow!("Error reading stream line {}", e))),
            };

            if line.is_empty() {
                return None;
            }

            if line.starts_with("event: ") {
                // TODO: manage the event type
                let _ = line.strip_prefix("event: ").unwrap_or_default();
                None
            } else if line.starts_with("data: ") {
                let data = line.strip_prefix("data: ").unwrap_or_default();

                match serde_json::from_str::<StreamEvent>(data) {
                    Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                        if delta.delta_type == "text_delta" {
                            return Some(Ok(StreamingBuffer {
                                content: delta.text,
                                is_complete: false,
                            }));
                        } else {
                            return None;
                        }
                    }
                    Ok(StreamEvent::MessageStop) => {
                        return Some(Ok(StreamingBuffer {
                            content: String::new(),
                            is_complete: true,
                        }));
                    }
                    _ => {
                        return None;
                    } // TODO: what other events ?
                }
            } else {
                return None;
            }
        });

        Ok(event_stream)
    }

    #[deprecated]
    pub async fn count_token(&self, message: &str) -> Result<u32> {

        if message.trim().is_empty(){
            return Ok(0);
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10)) // enough to count ugh ?
            .build()
            .expect("Failed to create HTTP client");

        const API_URL: &str = "https://api.anthropic.com/v1/messages/count_tokens";

        let request = AntTokCountRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: Role::User,
                content: String::from(message),
            }],
        };

        let response = client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            let error_f = format!("API error ({}): {}", status, error_text);
            return Err(anyhow::anyhow!(error_f));
        }

        let anthropic_response: AntTokCountResponse = response.json().await?;
        debug!("Received response: {:?}", anthropic_response);

        Ok(anthropic_response.input_tokens)
    }

    #[deprecated]
    pub async fn get_tokens_price(
        &self,
        message: &str,
        toktype: TokenType,
        model_price: &ModelPricing,
    ) -> Result<f64> {

        let token_count = self.count_token(message).await?;
        match toktype {
            TokenType::InputToken => {
                Ok(model_price.input_cost_per_million * (token_count as f64 / 1000000.0))
            }
            TokenType::OutputToken => {
                Ok(model_price.output_cost_per_million * (token_count as f64 / 1000000.0))
            }
        }
    }

}
