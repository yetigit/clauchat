use eframe::{egui, NativeOptions};
use log::info;
use egui::ViewportBuilder;

mod app;
mod api;
mod config;
mod ui;

use app::ClauChatApp;

//TODO:
//-[] check if the api key is valid not if it is empty.
//-[] save the api key from the settings only if it differs from the current api_key

fn main() -> Result<(), eframe::Error> {
    dotenv::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format_timestamp_secs()
        .init();

    info!("Starting ClauChat app");

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([640.0, 480.0])
            .with_resizable(true)
            .with_decorations(true)
            .with_transparent(false)
            .with_title("ClauChat"),
        vsync: true,
        multisampling: 4,
        ..Default::default()
    };

    eframe::run_native(
        "ClauChat - Claude 3.7 Sonnet",
        options,
        Box::new(|cc| {
            // Create the application state
            Ok(Box::new(ClauChatApp::new(cc)))
        }),
    )
}
