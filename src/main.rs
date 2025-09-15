use eframe::{egui, NativeOptions};
use log::info;
use egui::ViewportBuilder;

mod api;
mod config;
mod syntax_lit;
mod chat_render;
mod ui;
mod price;
mod app;

use crate::app::ClauChatApp;

//TODO:
//-[] change colors of light theme
//-[] save window rect in config
//-[] upload files
//-[] implement claude's system option, 
// ---
//-[] implement claude temperature setting
//-[] implement prompt caching

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
        "ClauChat",
        options,
        Box::new(|cc| {
            let mut clauchat_app = ClauChatApp::new(cc);
            clauchat_app.init()?;
            Ok(Box::new(clauchat_app))
        }),
    )
}
