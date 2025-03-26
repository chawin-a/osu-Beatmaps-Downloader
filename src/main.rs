#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
// #![windows_subsystem = "windows"]
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eyre::{eyre, Result};
use rosu_v2::prelude::*;
use serde_yaml;
use std::sync::Arc;

mod downloader;
mod settings;

fn check_config_file() -> bool {
    let config_path = std::path::Path::new("config.yaml");
    config_path.exists()
}

fn read_config_from_yaml(file_path: &str) -> Result<settings::Config> {
    let file = std::fs::File::open(file_path)?;
    let config: settings::Config = serde_yaml::from_reader(file)?;
    Ok(config)
}

fn main() -> Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    if !check_config_file() {
        let options = eframe::NativeOptions {
            run_and_return: true,
            viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 320.0]),
            ..Default::default()
        };

        log::info!("Starting settings window…");
        eframe::run_native(
            "First Window",
            options,
            Box::new(|_cc| Ok(Box::new(settings::ConfigApp::new()))),
        )
        .map_err(|e| eyre!("error occurs: {:?}", e))?;
    }

    let runtime = Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap(),
    );

    let config = read_config_from_yaml("config.yaml").unwrap();

    let osu = runtime
        .block_on(Osu::new(config.client_id, config.client_secret))
        .unwrap();

    let options = eframe::NativeOptions {
        run_and_return: true,
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "osu! Beatmap Downloader",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(downloader::BeatmapDownloaderApp::new(
                runtime,
                osu,
                config.songs_path,
                config.number_of_fetch,
                config.server,
                config.selected_server,
                config.number_of_simultaneous_downloads,
            ))
        }),
    )
    .map_err(|e| eyre!("error occurs: {:?}", e))
}
