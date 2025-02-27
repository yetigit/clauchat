use anyhow::Result;
use log::{info, debug, error};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
}

/// Content block in the anth API response
#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

/// Message in the anth API response
#[derive(Debug, Deserialize)]
struct ResponseMessage {
    role: String,
    content: Vec<ContentBlock>,
}

/// Response structure from the anthropic API
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<ContentBlock>,
}

/// Http client for requests to anth
#[derive(Clone)]
pub struct AnthropicClient {
    api_key: String,
    client: Arc<reqwest::Client>,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            client: Arc::new(client),
        }
    }

    pub async fn send_message(&self, messages: Vec<Message>) -> Result<String> {
        const API_URL: &str = "https://api.anthropic.com/v1/messages";
        const MODEL: &str = "claude-3-7-sonnet-20250219";
        const MAX_TOKENS: u32 = 4096;

        let request = AnthropicRequest {
            model: MODEL.to_string(),
            messages,
            max_tokens: MAX_TOKENS,
        };

        debug!("Sending request to Anthropic API: {:?}", request);

        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        info!("Request was made from input");

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

        Ok(full_content)
    }
}
