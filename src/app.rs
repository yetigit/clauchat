use eframe::{egui, CreationContext};
use egui::Context;
use log::{ info, error };
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use egui::Visuals;

use crate::api::{AnthropicClient, Message, Role};
use crate::config::{ Config, Theme};
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

        let ctx = &cc.egui_ctx;
        ctx.set_visuals(Visuals::dark());

        let config = Config::load().unwrap_or_default();

        let client = if !config.api_key.is_empty() {
            Some(AnthropicClient::new(config.api_key.clone()))
        } else {
            None
        };

        let messages = vec![Message {
            role: Role::Assistant,
            content: "How can I help you?".to_string(),
        }];

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
                error!("API key not configured. Please add it in settings.");
                self.error = Some("API key not configured. Please add it in settings.".to_string());
                return;
            }
        };

        let user_message = Message {
            role: Role::User,
            content: self.input.clone(),
        };
        self.messages.push(user_message);

        std::mem::take(&mut self.input);
        self.is_sending = true;

        // clone for threads
        let client = client.clone();
        let messages = self.messages.clone();
        let result: Arc<Mutex<Option<Result<String, String>>>> = Arc::new(Mutex::new(None));
        let result_clone = result.clone();

        self.runtime.spawn(async move {
            match client.send_message(messages).await {
                Ok(response) => {
                    let mut result = result_clone.lock().unwrap();
                    *result = Some(Ok(response));
                }
                Err(err) => {
                    let mut result = result_clone.lock().unwrap();
                    *result = Some(Err(err.to_string()));
                }
            }
        });

        let ctx = egui::Context::clone(&egui::Context::default());
        std::thread::spawn(move || loop {
            let response = {
                let mut result = result.lock().unwrap();
                result.take()
            };

            if response.is_some() {
                ctx.request_repaint();
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        });
    }


    fn save_config(&self) {
        if let Err(err) = self.config.save() {
            error!("Failed to save config: {}", err);
        }
    }

    fn update_api_key(&mut self, new_key: String) {
        self.config.api_key = new_key;
        if !self.config.api_key.is_empty() {
            self.client = Some(AnthropicClient::new(self.config.api_key.clone()));
            self.error = None;
        } else {
            self.client = None;
        }
        self.save_config();
    }

    fn apply_font_size(&self, ctx:&Context) {
        let mut style = (*ctx.style()).clone();
        style.text_styles.iter_mut().for_each(|(_text_style, font_id)|{
            font_id.size = self.config.font_size;
        });
        ctx.set_style(style) ;
    }
}

impl eframe::App for ClauChatApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame){
        match self.config.theme {
            Theme::Dark => {
                ctx.set_visuals(Visuals::dark());
            }
            Theme::Light => {
                ctx.set_visuals(Visuals::light());
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut update_api_key_action: Option<String> = None;

            // apply font size
            self.apply_font_size(ctx);
            ui::render_header(ui, &mut self.ui_state, &mut self.config, |new_key| {
                update_api_key_action = Some(new_key);
            });

            if let Some(new_key) = update_api_key_action {
                self.update_api_key(new_key);
            }

            if let Some(error) = &self.error {
                ui::render_error(ui, error);
            }

            //
            ui.vertical(|ui| {
                ui::render_chat_area(ui, &self.messages);

                let mut should_send_message = false;
                //
                ui::render_input_area(ui, &mut self.input, self.is_sending, || {
                    should_send_message = true;
                });
                if should_send_message {
                    self.send_message();
                }
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let config_path = Config::config_path().unwrap();
        self.save_config();
        info!("Configuration saved to {}", config_path.display());
    }
}
