#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
// #![windows_subsystem = "windows"]
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eyre::{Result, WrapErr};
use rosu_v2::prelude::*;
use std::sync::Arc;
use thiserror::Error;

mod client;
mod downloader;
mod settings;
mod utils;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Failed to load application icon: {0}")]
    IconError(String),
    #[error("Failed to create search client: {0}")]
    ClientCreationError(String),
    #[error("Failed to initialize runtime: {0}")]
    RuntimeError(#[from] std::io::Error),
    #[error("Failed to read config: {0}")]
    ConfigError(#[from] eyre::Error),
    #[error("Failed to run application: {0}")]
    ApplicationError(String),
}

fn create_search_client(
    runtime: &Arc<tokio::runtime::Runtime>,
    config: &settings::Config,
) -> Result<Box<dyn client::SearchClient>, AppError> {
    match config.search_client.as_str() {
        "nerinyan" => Ok(Box::new(client::nerinyan::NerinyanClient::new())),
        "osu" => Ok(Box::new(client::osu::OsuClient::new())),
        "osu_api" => {
            let client = runtime
                .block_on(Osu::new(config.client_id, config.client_secret.clone()))
                .map_err(|e| AppError::ClientCreationError(e.to_string()))?;
            Ok(Box::new(client))
        }
        _ => Err(AppError::ClientCreationError("Unknown client type".to_string())),
    }
}

fn main() -> Result<()> {
    // Log to stderr (if you run with `RUST_LOG=debug`).
    env_logger::init();

    // Load application icon with proper error handling
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("img/raw_icon.png"))
        .map_err(|e| AppError::IconError(e.to_string()))?;

    // Show settings window if config doesn't exist
    if !settings::check_config_file() {
        let options = eframe::NativeOptions {
            run_and_return: true,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([320.0, 320.0])
                .with_icon(icon.clone()),
            ..Default::default()
        };

        log::info!("Starting settings window…");
        eframe::run_native(
            "First Window",
            options,
            Box::new(|_cc| Ok(Box::new(settings::ConfigApp::new()))),
        )
        .map_err(|e| AppError::ApplicationError(e.to_string()))?;
    }

    // Initialize runtime with proper error handling
    let runtime = Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(AppError::RuntimeError)?,
    );

    // Read config with proper error handling
    let config = settings::read_config_from_yaml("config.yaml")
        .wrap_err("Failed to read config file")?;

    // Create search client with proper error handling
    let search_client = create_search_client(&runtime, &config)?;

    // Configure main window
    let options = eframe::NativeOptions {
        run_and_return: true,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_icon(icon),
        ..Default::default()
    };

    // Run main application with proper error handling
    eframe::run_native(
        "osu! Beatmap Downloader",
        options,
        Box::new(|cc| {
            // Initialize image support
            egui_extras::install_image_loaders(&cc.egui_ctx);

            // Create and return the main application
            Ok(downloader::BeatmapDownloaderApp::new(
                runtime,
                search_client,
                config.songs_path,
                config.number_of_fetch,
                config.server,
                config.selected_server,
                config.number_of_simultaneous_downloads,
            ))
        }),
    )
    .map_err(|e| AppError::ApplicationError(e.to_string()).into())
}
