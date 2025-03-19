use anyhow::Result;
use log::{info, debug};
use serde::{Deserialize, Serialize};
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
}

#[derive(Debug, Serialize)]
struct AntTokCountRequest {
    model: String,
    messages: Vec<Message>,
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

#[derive(Debug, Deserialize)]
struct AntTokCountResponse {
    input_tokens: u32,
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
