#![windows_subsystem = "windows"]

mod app;
mod competitive_companion;
mod executor;
mod judge;
mod models;
mod storage;
mod ui;

use anyhow::Result;
use eframe::egui;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with info level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create shared problem state
    let problem_store = Arc::new(Mutex::new(storage::ProblemStore::new()?));

    // Start Competitive Companion server
    let server_store = problem_store.clone();
    tokio::spawn(async move {
        if let Err(e) = competitive_companion::start_server(server_store).await {
            tracing::error!("Competitive Companion server error: {}", e);
        }
    });

    // Start GUI
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 800.0])
            .with_title("CPKit - Competitive Programming Testing Tool"),
        ..Default::default()
    };

    eframe::run_native(
        "CPKit",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::CPKitApp::new(cc, problem_store)))),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;

    Ok(())
}
