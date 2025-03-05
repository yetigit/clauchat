use eframe::{egui, CreationContext};
use egui::Context;
use log::{debug, info, error };
use std::sync::{Arc, Mutex, mpsc};
use tokio::runtime::Runtime;
use tokio::time::{interval, sleep, Duration};
use egui::Visuals;
use std::collections::HashMap;

use crate::api::{get_tokens_heur_price, AnthropicClient, Message, Role, TokenType};
use crate::config::{ Config, Theme};
use crate::ui;
use crate::price::{fetch_model_pricing, ModelPricing};

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

    /// channel for api response thread transit 
    response_receiver: Option<mpsc::Receiver<Result<String, String>>>,

    /// error message if any
    error: Option<String>,

    /// model being used
    model: String,

    /// token pricing info
    pricing_data: Option<HashMap<String, ModelPricing>>,

    input_cost: Arc<Mutex<Option<Result<f64, String>>>>,

}

impl ClauChatApp {
    pub fn new(cc: &CreationContext) -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        let ctx = &cc.egui_ctx;
        ctx.set_visuals(Visuals::dark());

        let config = Config::load().unwrap_or_default();

        // TODO: pass it to anthropic client 
        const MODEL: &str = "claude-3-7-sonnet-20250219";
        let price_data = runtime.block_on(async {
            fetch_model_pricing(Some(MODEL)).await
        }).unwrap();

        let client = if !config.api_key.is_empty() {
            Some(AnthropicClient::new(config.api_key.clone()))
        } else {
            None
        };

        let messages = vec![Message {
            role: Role::Assistant,
            content: "How can I help you?".to_string(),
        }];

        let input_cost: Arc<Mutex<Option<Result<f64, String>>>> = Arc::new(Mutex::new(None));
        Self {
            input: String::new(),
            messages,
            is_sending: false,
            config,
            runtime,
            client,
            ui_state: ui::UiState::default(),
            response_receiver: None,
            error: None,
            model: MODEL.to_string(),
            pricing_data: price_data,
            input_cost,
        }
    }

    fn handle_api_response(&mut self, response: Result<String, String>) {
        match response {
            Ok(content) => {
                let assistant_message = Message {
                    role: Role::Assistant,
                    content,
                };
                self.messages.push(assistant_message);
            }
            Err(err) => {
                error!("Failed to get valid response: {}", err);
                self.error = Some(format!("Failed to get valid response: {}", err));
            }
        }
        self.is_sending = false;
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

        let api_key_clone = self.config.api_key.clone();
        let good_key = self.runtime.block_on(async move {
            AnthropicClient::is_api_key_valid(api_key_clone)
                .await
                .unwrap_or_else(|e| {
                    error!("API key validation request failed: {}", e);
                    false
                })
        });

        if !good_key {
            error!("Bad API key, request process canceled");
            self.client = None;
            self.error = Some("Bad API key, request process canceled".to_string());
            return;
        }else {
            info!("Good API key");
        }

        let user_message = Message {
            role: Role::User,
            content: self.input.clone(),
        };
        self.messages.push(user_message);
        self.error = None;

        std::mem::take(&mut self.input);
        self.is_sending = true;

        // clone for async
        let client = client.clone();
        let messages = self.messages.clone();

        let (tx, rx) = mpsc::channel();
        self.response_receiver = Some(rx);
        self.runtime.spawn(async move {
            match client.send_message(messages).await {
                Ok(response) => {
                    debug!("Got some response");
                    let _ = tx.send(Ok(response));
                }
                Err(err) => {
                    let _ = tx.send(Err(err.to_string()));
                }
            }
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

        let client = match &self.client {
            Some(client) => client,
            None => {
                error!("Corrupt client");
                self.error = Some("Corrupt client".to_string());
                return;
            }
        };

        let client = client.clone();
        let model_price = {
            let pricing_data = match &self.pricing_data {
                Some(pricing_data) => Some(pricing_data.clone()),
                None => None,
            };
            // TODO: dont unwrap like that
            pricing_data.unwrap().get(&self.model).unwrap().clone()
        };
        let input_clone = self.input.clone();
        let input_cost_clone = self.input_cost.clone();

        self.runtime.spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;

                match client
                    .get_tokens_price(&input_clone, TokenType::InputToken, &model_price)
                    .await
                {
                    Ok(price) => {
                        let mut input_cost = input_cost_clone.lock().unwrap();
                        *input_cost = Some(Ok(price));
                    }
                    Err(e) => {
                        let mut input_cost = input_cost_clone.lock().unwrap();
                        *input_cost = Some(Err(e.to_string()));
                    }
                }
            }
        });

        if let Some(receiver) = &self.response_receiver {
            if let Ok(response) = receiver.try_recv() {
                info!("Handling response");
                self.handle_api_response(response);
                self.response_receiver = None;
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

                let x = self.input_cost.clone();
                let y = x.lock().unwrap();
                let z = (*y).clone();
                let input_cost_avail: Option<String> = match z {
                    Some(response) => match response {
                        Ok(input_cost) => Some(format!("${:.6}", input_cost)),
                        Err(e) => {
                            error!("Error: {}", e);
                            None
                        }
                    },
                    None => None,
                };
                //
                ui::render_input_area(ui, &mut self.input, 
                    input_cost_avail, self.is_sending, || {
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
