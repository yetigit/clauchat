use eframe::{egui, NativeOptions};
use log::info;
use egui::ViewportBuilder;

mod app;
mod api;
mod config;
mod ui;
mod price;

use app::ClauChatApp;

//TODO:
//-[x] estimate cost of input (display in real time if possible)
//-[] estimate cost using anth api
//-[] print total cost in realtime
//-[] animation for response waiting time
//-[] display code blocks
//-[] change colors of light theme
//-[] implement claude's system option, 
//-[] implement claude temperature setting
//-[] implement prompt caching
//-[] replies should be on opposite sides,
// e.g user post is offset to the left, claude answer to the right 
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
        // TODO: should take that from the model member variable of some struct
        "ClauChat - Claude 3.7 Sonnet",
        options,
        Box::new(|cc| {
            // Create the application state
            Ok(Box::new(ClauChatApp::new(cc)))
        }),
    )
}
