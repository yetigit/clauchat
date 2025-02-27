use eframe::egui::{self, Button, Align, Color32, Layout, RichText, ScrollArea, TextEdit, Ui};

use crate::api::{Message, Role};
use crate::config::{Config, Theme};

// UI states
#[derive(Default)]
pub struct UiState {
    pub settings_open: bool,
    pub api_key_buffer: String,
}

pub fn render_header(
    ui: &mut Ui,
    ui_state: &mut UiState,
    config: &mut Config,
    on_api_key_change: impl FnOnce(String),
) {
    ui.horizontal(|ui| {
        ui.heading("ClauChat");
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
        egui::Frame::none()
            .fill(ui.style().visuals.extreme_bg_color)
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

                    if ui
                        .selectable_label(matches!(current_theme, Theme::System), "System")
                        .clicked()
                    {
                        config.theme = Theme::System;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Font Size:");
                    ui.add(egui::Slider::new(&mut config.font_size, 12.0..=24.0).step_by(1.0));
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
    let (color, prefix) = match message.role {
        Role::User => (Color32::RED, "You"),
        Role::Assistant => (Color32::LIGHT_BLUE, "Claude"),
        Role::System => (Color32::LIGHT_GREEN, "System"),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{}: ", prefix)).color(color).strong());
    });

    ui.label(RichText::new(&message.content).color(color));
    ui.add_space(8.0);
}

pub fn render_chat_area(ui: &mut Ui, messages: &[Message]) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
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
    is_sending: bool,
    on_send: impl FnOnce(),
) {
    ui.separator();

    ui.horizontal(|ui| {
        let text_edit = TextEdit::multiline(input)
            .hint_text("Type a message...")
            .desired_rows(3)
            .lock_focus(true);
            
        let text_edit_response = ui.add(text_edit);
        
        // Handle Enter key to send (but allow Shift+Enter for new lines)
        let pressed_enter = text_edit_response.lost_focus() && 
                           ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
        
        // Send button
        let button = ui.add_enabled(
            !input.trim().is_empty() && !is_sending,
            Button::new(if is_sending { "Sending..." } else { "Send" })
        );
        
        // Call the callback if either the enter key was pressed or the button was clicked
        if (pressed_enter || button.clicked()) && !input.trim().is_empty() && !is_sending {
            on_send();
        }
    });
}
