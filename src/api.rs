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


fn token_count_heuristic(content: &str) -> usize {
    let char_count = content.chars().count();
    (char_count as f64 / 4.0).ceil() as usize
}

pub fn get_tokens_heur_price(content: &str, toktype: TokenType, model_price: &ModelPricing) -> f64 {
    let token_count = token_count_heuristic(content);
    match toktype {
        TokenType::InputToken => {
            model_price.input_cost_per_million * (token_count as f64 / 1000000.0)
        }
        TokenType::OutputToken => {
            model_price.output_cost_per_million * (token_count as f64 / 1000000.0)
        }
    }
}


impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30)) // timeout request in 30 sec
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            client: Arc::new(client),
            // TODO: pass it as argument to ::new() instead
            model: "claude-3-7-sonnet-20250219".to_string(),
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
        debug!(
            "API key status: {}",
            if success {
                "good key"
            } else {
                "bad key"
            }
        );
        Ok(success)
    }

    pub async fn send_message(&self, messages: Vec<Message>) -> Result<String> {
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

        debug!("Request was made to Anthropic API: {:?}", request);

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
        // debug!("Extracted response: {}", full_content);

        Ok(full_content)
    }

    pub async fn count_token(&self, message: &str) -> Result<u32> {

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10)) // enough to count ugh ?
            .build()
            .expect("Failed to create HTTP client");

        const API_URL: &str = "https://api.anthropic.com/v1/messages/count_tokens";
        const MAX_TOKENS: u32 = 4096;

        let request = AnthropicRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: Role::User,
                content: String::from(message),
            }],
            max_tokens: MAX_TOKENS,
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

    // TODO: use
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
