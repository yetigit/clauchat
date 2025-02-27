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
                    // TODO: save when the sliding is done
                    if slider_response.changed() {
                        config
                            .save()
                            .unwrap_or_else(|_| config.font_size = old_font_size);
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
    let (color, prefix) = match message.role {
        Role::User => (Color32::WHITE, "You"),
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
    is_sending: bool,
    on_send: impl FnOnce(),
) {
    ui.separator();

    let available_height = ui.available_height();
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), available_height),
        Layout::left_to_right(Align::Center),
        |ui| {
            let text_edit = TextEdit::multiline(input)
                .hint_text("Type a message...")
                .desired_width(ui.available_width() - 80.0)
                .min_size(egui::vec2(0.0, available_height))
                .lock_focus(true);

            let text_edit_response = ui.add(text_edit);

            // Handle Enter key to send (but allow Shift+Enter for new lines)
            let pressed_enter = text_edit_response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);

            // Send button
            let button = ui.add_enabled(
                !input.trim().is_empty() && !is_sending,
                Button::new(RichText::new(if is_sending { "Sending..." } else { "Send" }).strong())
                    .min_size(egui::vec2(80.0, 50.0)),
            );

            // Call the callback if either the enter key was pressed or the button was clicked
            if (pressed_enter || button.clicked()) && !input.trim().is_empty() && !is_sending {
                on_send();
            }

            ui.add_space(14.0);
        },
    );
}
