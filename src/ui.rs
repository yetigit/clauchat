use eframe::egui::{self, epaint::Marginf, Button, Align, Color32, Layout, RichText, ScrollArea, TextEdit, Ui};
use log::{debug, error, info};

use crate::api::{Message, Role};
use crate::config::{Config, Theme};
use crate::chat_render::ChatRenderer;

// UI states
#[derive(Clone)]
pub struct UiState {
    pub settings_open: bool,
    pub api_key_buffer: String,
    pub input_cost_display: Option<f64>,
    pub total_cost: f64,
}

impl Default for UiState {
    fn default() -> Self {
        Self{
            settings_open: false,
            api_key_buffer: String::new(),
            input_cost_display: None,
            total_cost: 0.0,
        }
    }

}

pub fn render_header(
    ui: &mut Ui,
    ui_state: &mut UiState,
    config: &mut Config,
    on_api_key_change: impl FnOnce(String),
) {
    ui.horizontal(|ui| {
        // ui.heading("ClauChat");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button("Settings").clicked() {
                ui_state.settings_open = !ui_state.settings_open;
                if ui_state.settings_open && ui_state.api_key_buffer.is_empty() {
                    ui_state.api_key_buffer = config.api_key.clone();
                }
            }
        });
    });

    ui.separator();

    if ui_state.settings_open {
        egui::Frame::new()
            // .fill(ui.style().visuals.extreme_bg_color)
            .show(ui, |ui| {
                ui.heading("Settings");

                ui.horizontal(|ui| {
                    ui.label("API Key:");
                    let api_key_response = ui.add(
                        TextEdit::singleline(&mut ui_state.api_key_buffer)
                            .password(true)
                            .hint_text("API key"),
                    );

                    if api_key_response.changed() {
                        let new_key = ui_state.api_key_buffer.clone();
                        on_api_key_change(new_key);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    let current_theme = config.theme.clone();
                    if ui
                        .selectable_label(matches!(current_theme, Theme::Light), "Light")
                        .clicked()
                    {
                        config.theme = Theme::Light;
                    }

                    if ui
                        .selectable_label(matches!(current_theme, Theme::Dark), "Dark")
                        .clicked()
                    {
                        config.theme = Theme::Dark;
                    }

                });

                ui.horizontal(|ui| {
                    let old_font_size = config.font_size;
                    ui.label("Font Size:");
                    let slider_response =
                        ui.add(egui::Slider::new(&mut config.font_size, 12.0..=24.0).step_by(1.0));
                    // save on slider change
                    if slider_response.drag_stopped() {
                        config
                            .save()
                            .unwrap_or_else(|e| {
                                error!("Could not save config: {}", e);
                                config.font_size = old_font_size;
                            });
                    }else if !slider_response.dragged() && slider_response.changed(){
                        config
                            .save()
                            .unwrap_or_else(|e| {
                                error!("Could not save config: {}", e);
                                config.font_size = old_font_size;
                            });

                    }
                });

                ui.separator();
            });
    }
}

pub fn render_error(ui: &mut Ui, error: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Error: ").color(Color32::RED).strong());
        ui.label(error);
    });
    ui.separator();
}

pub fn render_message(ui: &mut Ui, message: &Message) {

        // .color(Color32::from_rgba_premultiplied(255, 191, 0, 180))
    let (color, prefix) = match message.role {
        Role::User => (Color32::WHITE, "You"),
        Role::Assistant => (Color32::from_rgba_premultiplied(255, 191, 145, 255), "Claude"),
        Role::System => (Color32::LIGHT_GREEN, "System"),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{}: ", prefix)).color(color).strong());
    });

    ChatRenderer::render_message_content(ui, &message.content);
    // ui.label(RichText::new(&message.content).color(color));
    ui.add_space(8.0);
}

pub fn render_chat_area(ui: &mut Ui, messages: &[Message]) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .max_height(ui.available_height() * 0.7)
        .show(ui, |ui| {
            for message in messages {
                render_message(ui, message);
            }
        });
}
//
pub fn render_input_area(
    ui: &mut Ui,
    input: &mut String,
    ui_state: &UiState,
    is_sending: bool,
    on_send: impl FnOnce(),
    on_input_change: impl FnOnce(),
) {
    ui.separator();

    let available_width = ui.available_width();
    let available_height = ui.available_height();
    ui.allocate_ui_with_layout(
        egui::vec2(available_width, available_height),
        Layout::left_to_right(Align::LEFT),
        |ui| {
            let text_edit = TextEdit::multiline(input)
                .hint_text("Ask anything...")
                .desired_width(available_width - 70.0)
                .min_size(egui::vec2(available_width - 70.0, available_height))
                .lock_focus(true)
                .margin(Marginf {
                    left: 0.0,
                    right: 70.0,
                    top: 0.0,
                    bottom: 0.0,
                });

            let text_edit_response = ui.add(text_edit);
            if text_edit_response.changed() {
                on_input_change();
            }

            // TODO: tooltip for these
            if let Some(_input_cost) = ui_state.input_cost_display {
                let overlay_pos = ui.min_rect().max - egui::vec2(6.0, 8.0);
                let builder = egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    overlay_pos - egui::vec2(70.0, 40.0),
                    egui::vec2(72.0, 40.0),
                ));

                ui.allocate_new_ui(builder, |ui| {
                    let overlay_text = RichText::new(format!("${:.6}", _input_cost))
                        .color(Color32::from_rgba_premultiplied(250, 250, 210, 255))
                        .size(14.0);
                    ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                        // debug!("make price overlay");
                        ui.label(overlay_text);
                    });
                });
            }

            let overlay_pos = ui.min_rect().max - egui::vec2(6.0, 2.0);
            let builder = egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                overlay_pos - egui::vec2(70.0, 70.0),
                egui::vec2(72.0, 40.0),
            ));

            ui.allocate_new_ui(builder, |ui| {
                let overlay_text = RichText::new(format!("${:.6}", ui_state.total_cost))
                    .color(Color32::from_rgba_premultiplied(255, 191, 145, 255))
                    .size(14.0);
                ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                    // debug!("make price overlay");
                    ui.label(overlay_text);
                });
            });

            // Handle Enter key to send (but allow Shift+Enter for new lines)
            let pressed_enter = text_edit_response.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.shift);

            // Call the callback if either the enter key was pressed or the button was clicked
            if pressed_enter && !input.trim().is_empty() && !is_sending {
                on_send();
            }

            ui.add_space(14.0);
        },
    );
}
