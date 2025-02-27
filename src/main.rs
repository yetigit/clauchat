use eframe::{egui, NativeOptions};
use log::info;
use egui::ViewportBuilder;

mod app;
mod api;
mod config;
mod ui;

use app::ClauChatApp;

fn main() -> Result<(), eframe::Error>{
    env_logger::init();
    info!("Starting ClauChat app");

    dotenv::dotenv().ok();

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([800.0, 600.0])
            .with_resizable(true)
            .with_decorations(true)
            .with_transparent(false)
        ,
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
