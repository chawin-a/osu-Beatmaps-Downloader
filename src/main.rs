#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
// #![windows_subsystem = "windows"]
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eyre::{eyre, Result};
use rosu_v2::prelude::*;
use std::sync::Arc;

mod client;
mod downloader;
mod settings;
mod utils;

fn create_search_client(
    runtime: &Arc<tokio::runtime::Runtime>,
    config: &settings::Config,
) -> Box<dyn client::SearchClient> {
    match config.search_client.as_str() {
        "nerinyan" => Box::new(client::nerinyan::NerinyanClient::new()),
        "osu" => Box::new(client::osu::OsuClient::new()),
        "osu_api" => Box::new(
            runtime
                .block_on(Osu::new(config.client_id, config.client_secret.clone()))
                .unwrap(),
        ),
        _ => panic!("Unknown client type"),
    }
}

fn main() -> Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("img/raw_icon.png"))
        .expect("The icon data must be valid");

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
        .map_err(|e| eyre!("error occurs: {:?}", e))?;
    }

    let runtime = Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap(),
    );

    let config = settings::read_config_from_yaml("config.yaml").unwrap();

    let search_client = create_search_client(&runtime, &config);

    let options = eframe::NativeOptions {
        run_and_return: true,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_icon(icon),
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
                search_client,
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
