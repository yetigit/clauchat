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

use app::ClauChatApp;

//TODO:
//-[x] estimate cost of input (display in real time if possible)
//-[x] print total cost in realtime
//-[] animation for response waiting time
//-[x] display code blocks
//-[] change colors of light theme
//-[] implement claude's system option, 
//-[] implement claude temperature setting
//-[] implement prompt caching
//-[] replies should be on opposite sides,
// e.g user post is offset to the right, claude answer to the left
//-[] save window rect in config
//-[] add scroll area for text input
//-[] ctrl middle mouse for font adjust
//have it in config/settings

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
