use futures_util::StreamExt;
use eframe::{egui, CreationContext};
use egui::Context;
use log::{debug, info, error };
use std::sync::{Arc, Mutex, mpsc};
use mpsc::Receiver;
use mpsc::Sender;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::runtime::Runtime;
use egui::Visuals;
use std::collections::HashMap;
use tiktoken_rs::cl100k_base; /// Use ChatGPT tokenizer

use crate::api::{AnthropicClient, AppMessageDelta, Message, Role, TokenType, ResponseUsage, ExtractedResponse};
use crate::config::{ Config, Theme};
use crate::ui;
use crate::price::{fetch_model_pricing, ModelPricing};

const STREAM_ERROR_TOKEN: &str = "Err\u{274}r:";

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
    response_receiver: Option<Receiver<Result<ExtractedResponse, String>>>,
    stream_receiver: Option<tokio_mpsc::Receiver<AppMessageDelta>>,
    input_sender: Option<Sender<String>>,
    input_receiver: Option<Receiver<String>>,

    /// error message if any
    error: Option<String>,

    /// model being used
    model: String,

    /// token pricing info
    pricing_data: Option<HashMap<String, ModelPricing>>,

    /// input cost estimate display
    input_cost: Arc<Mutex<Option<Result<f64, String>>>>,


}


impl ClauChatApp {
    pub fn new(cc: &CreationContext) -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        let ctx = &cc.egui_ctx;
        ctx.set_visuals(Visuals::dark());

        let config = Config::load().unwrap_or_default();

        const MODEL: &str = "claude-3-7-sonnet-20250219";
        let price_data = runtime.block_on(async {
            fetch_model_pricing(Some(MODEL)).await
        }).unwrap();

        let client = if !config.api_key.is_empty() {
            Some(AnthropicClient::new(MODEL, config.api_key.clone()))
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
            stream_receiver: None,
            input_sender: None,
            input_receiver: None,
            error: None,
            model: MODEL.to_string(),
            pricing_data: price_data,
            input_cost,
        }
    }

    fn send_input_required(&mut self) -> Result<(), String> {
        // debug!("Sending input to thread");
        if let Err(e) = self.input_sender.as_ref().unwrap().send(self.input.clone()) {
            error!("Error sending input to processing thread: {}", e);
        }

        Ok(())
    }

    // fn send_input(&mut self) -> Result<(), String> {
    //     const SEND_INTERVAL: Duration = Duration::from_millis(300);
    //     let time_now = std::time::Instant::now();
    //     if time_now >= self.input_next_send_t {
    //         debug!("Sending input to thread");
    //         if let Err(e) = self.input_sender.as_ref().unwrap().send(self.input.clone()) {
    //             error!("Error sending input to processing thread: {}", e);
    //         }
    //         self.input_next_send_t = time_now + SEND_INTERVAL;
    //     }

    //     Ok(())
    // }

    pub fn init(&mut self) -> Result<(), String> {
        if self.input_sender.is_none() || self.input_receiver.is_none() {
            let (tx, rx) = mpsc::channel::<String>();
            self.input_sender = Some(tx);
            self.input_receiver = Some(rx);
        }

        let input_cost_clone = self.input_cost.clone();
        let model_price = self
            .pricing_data
            .as_ref()
            .and_then(|pricing_data| pricing_data.get(&self.model).cloned())
            .unwrap();

        let t_receiver = self
            .input_receiver
            .take()
            .expect("Input receiver already taken");

        std::thread::spawn(move || {
            loop {
                if let Ok(input) = t_receiver.recv() {
                    // debug!("Input: {}", input);
                    match ClauChatApp::get_tokens_heur_price(
                        &input,
                        TokenType::InputToken,
                        &model_price,
                    ) {
                        Ok(_input_cost) => {
                            let mut input_cost = input_cost_clone.lock().unwrap();
                            *input_cost = Some(Ok(_input_cost));
                        }
                        Err(e) => {
                            error!("Error: {}", e.to_string());
                        }
                    };

                    // std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }

        });

        Ok(())
    }

    fn usage_as_cost(&self, usage: &ResponseUsage) -> Result<f64, String> {
        let model_price = self.pricing_data.as_ref().unwrap().get(&self.model).unwrap();
        let total = model_price.input_cost_per_million * (usage.input_tokens as f64 / 1000000.0) +
        model_price.output_cost_per_million * (usage.output_tokens as f64 / 1000000.0);
        Ok(total)
    }

    fn handle_stream_response(&mut self, content_delta: AppMessageDelta) {
        if content_delta.content.starts_with(STREAM_ERROR_TOKEN) {
            error!("Failed to get valid response: {}", content_delta.content);
            self.error = Some(content_delta.content);

            if let Some(usage) = &content_delta.usage {
                debug!("There is some usage: {:?}", usage);
                self.ui_state.total_cost += self.usage_as_cost(usage).unwrap();
            }
        } else if let Some(last_message) = self.messages.last_mut() {
            if last_message.role == Role::Assistant {
                last_message.content = content_delta.content;

                if let Some(usage) = &content_delta.usage {
                    debug!("There is some usage: {:?}", usage);
                    self.ui_state.total_cost += self.usage_as_cost(usage).unwrap();
                }
                // ctx.request_repaint(); // Request immediate repaint to show update
            }
        }
        if content_delta.is_complete {
            self.is_sending = false;
        }
    }

    #[deprecated]
    fn handle_api_response(&mut self, response: Result<ExtractedResponse, String>) {
        match response {
            Ok(response) => {
                let assistant_message = Message {
                    role: Role::Assistant,
                    content: response.content,
                };
                self.ui_state.total_cost += self.usage_as_cost(&response.usage).unwrap();
                self.messages.push(assistant_message);
            }
            Err(err) => {
                error!("Failed to get valid response: {}", err);
                self.error = Some(format!("Failed to get valid response: {}", err));
            }
        }
        self.is_sending = false;
        // info!("Total cost: {}", self.ui_state.total_cost);
    }

    /// Counting tokens using ChatGPT tokenizer, 
    /// it matches enough when the Anthropic pricing is applied
    fn token_count_heuristic(content: &str) -> Result<usize, String> {
        match cl100k_base() {
            Ok (bpe)=> {
                Ok(bpe.encode_ordinary(content).len())
            }
            Err(e) => Err(e.to_string()) 
        }
    }

    fn get_tokens_heur_price(content: &str, toktype: TokenType, model_price :&ModelPricing) -> Result<f64, String> {

        let token_count = ClauChatApp::token_count_heuristic(content)?;
        debug!("Token count: {}", token_count);
        match toktype {
            TokenType::InputToken => {
                Ok(model_price.input_cost_per_million * (token_count as f64 / 1000000.0))
            }
            TokenType::OutputToken => {
                Ok(model_price.output_cost_per_million * (token_count as f64 / 1000000.0))
            }
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

        let api_key_clone = self.config.api_key.clone();
        // TODO: why do I check if it's a good key, won't the request fail with an appropriate
        // message ?
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

        let (tx, rx) = tokio_mpsc::channel::<AppMessageDelta>(100);
        self.stream_receiver = Some(rx);

        // message we are going to dump the string into
        self.messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
        });

        self.runtime.spawn(async move {
            let mut content_delta = AppMessageDelta::default();

            match client.send_message_streaming(messages).await {
                Ok(mut stream) => {
                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(buffer) => {
                                content_delta.content.push_str(&buffer.content);
                                content_delta.is_complete = buffer.is_complete;
                                content_delta.usage = buffer.usage;
                                let _ = tx.send(content_delta.clone()).await;
                                if content_delta.is_complete {
                                    break;
                                }
                            }
                            Err(e) => {
                                content_delta.content = format!("{} {}", STREAM_ERROR_TOKEN, e);
                                content_delta.is_complete = true;
                                let _ = tx.send(content_delta).await;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    content_delta.content = format!("{} {}", STREAM_ERROR_TOKEN, e);
                    content_delta.is_complete = true;
                    let _ = tx.send(content_delta).await;
                }
            }
        });


        // let (tx, rx) = mpsc::channel();
        // self.response_receiver = Some(rx);
        // self.runtime.spawn(async move {
        //     match client.send_message(messages).await {
        //         Ok(response) => {
        //             debug!("Got some response");
        //             let _ = tx.send(Ok(response));
        //         }
        //         Err(err) => {
        //             let _ = tx.send(Err(err.to_string()));
        //         }
        //     }
        // });


    }

    fn save_config(&self) {
        if let Err(err) = self.config.save() {
            error!("Failed to save config: {}", err);
        }
    }

    fn update_api_key(&mut self, new_key: String) {
        self.config.api_key = new_key;
        if !self.config.api_key.is_empty() {
            self.client = Some(AnthropicClient::new(&self.model, self.config.api_key.clone()));
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


        if let Some(Ok(input_cost)) = &*self.input_cost.lock().unwrap() {
            self.ui_state.input_cost_display = Some(*input_cost);
        }

        if let Some(receiver) = &mut self.stream_receiver {
            if let Ok(content_delta) = receiver.try_recv() {
                self.handle_stream_response(content_delta);
            }
        }

        // if let Some(receiver) = &self.response_receiver {
        //     if let Ok(response) = receiver.try_recv() {
        //         info!("Handling response");
        //         self.handle_api_response(response);
        //         self.response_receiver = None;
        //     }
        // }

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
                let mut should_send_input = false;

                ui::render_input_area(ui, &mut self.input, 
                    &self.ui_state, self.is_sending, || {
                    should_send_message = true;
                }, || {
                        should_send_input = true;
                    });
                if should_send_message {
                    self.send_message();
                }
                if should_send_input {
                    self.send_input_required().unwrap();
                }
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_config();
    }
}
