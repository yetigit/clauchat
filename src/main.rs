use eframe::{egui, NativeOptions};
use log::info;
use egui::ViewportBuilder;

mod app;
mod api;
mod config;
mod ui;

// use app::ClauChatApp;

fn main() -> Result<(), eframe::Error>{
    env_logger::init();
    info!("Starting ClauChat app");

    dotenv::dotenv().ok();
    Ok(())
}
