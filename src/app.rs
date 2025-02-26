use eframe::{egui, CreationContext};
use egui::{Context, ScrollArea, TextEdit};
use log::{error, info};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use crate::api::{AnthropicClient, Message, Role};
use crate::config::Config;
use crate::ui;



/// application state 
pub struct ClauChatApp {
    /// user input being typed
    input: String,

    /// conversation history
    messages: Vec<Message>,

    /// is the input in the process of sending
    is_sending: bool,

    /// config i.e api key 
    config: Config,

    /// tokio runtime
    runtime: Runtime,

    /// API client
    client: Option<AnthropicClient>,

    /// basic ui state
    ui_state: ui::UiState,

    /// error message if any
    error: Option<String>,
}

impl ClauChatApp {

    pub fn new(cc: &CreationContext) -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        let config = Config::load().unwrap_or_default(); 

        // TODO: strip apikey first 
        let client = if !config.api_key.is_empty(){
            Some(AnthropicClient::new(config.api_key.clone()))
        }else{
            None
        };

        let mut messages = Vec::new();
        messages.push(Message {
            role: Role::Assistant,
            content: "How can I help you?".to_string(),
        });

        Self {
            input: String::new(),
            messages,
            is_sending: false,
            config,
            runtime,
            client,
            ui_state: ui::UiState::default(),
            error: None,
        }
    }

    fn send_message(&mut self) {
        if self.input.trim().is_empty() || self.is_sending {
            return;
        }

        let client = match &self.client {
            Some(client) => client,
            None => {
                self.error = Some ("API key not configured. Please add it in settings.".to_string());
                return;
            }
        };

        let user_message = Message {
            role: Role::User,
            content: self.input.clone(),
        };
        self.messages.push(user_message);

        let input = std::mem::take(&mut self.input);
        self.is_sending = true;

        // clone for async process
        let client = client.clone();
        let messages = self.messages.clone();

        let result = Arc::new(Mutex::new(None));
        let messages = self.messages.clone();
    }

}
