#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
// #![windows_subsystem = "windows"]
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eyre::{eyre, Result};
use rosu_v2::prelude::*;
use serde::Deserialize;
use serde_yaml;
use std::collections::HashSet;
use std::fs;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use tokio::runtime::Runtime;

#[derive(Deserialize, Debug)]
struct Config {
    client_id: u64,
    client_secret: String,
    songs_path: String,
}

fn read_config_from_yaml(file_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(file_path)?;
    let config: Config = serde_yaml::from_reader(file)?;
    Ok(config)
}

fn main() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let config = read_config_from_yaml("config.yaml").unwrap();

    let osu = runtime
        .block_on(Osu::new(config.client_id, config.client_secret))
        .unwrap();

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "osu! Beatmap Downloader",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(BeatmapDownloaderApp::new(runtime, osu, config.songs_path))
        }),
    )
    .map_err(|e| eyre!("error occurs: {:?}", e))
}

struct BeatmapDownloaderApp {
    number_of_fetch_songs: Arc<RwLock<u32>>,
    songs_path: String,
    local_songs: Arc<RwLock<HashSet<u32>>>,
    new_songs: HashSet<u32>,
    tx_control: Sender<bool>,
    rx_update: Receiver<HashSet<u32>>,
}

impl BeatmapDownloaderApp {
    fn new(runtime: Runtime, osu: Osu, songs_path: String) -> Box<Self> {
        let (tx_update, rx_update) = mpsc::channel::<HashSet<u32>>();
        let (tx_control, rx_control) = mpsc::channel::<bool>();
        let local_songs = Arc::new(RwLock::new(HashSet::<u32>::new()));
        let local_songs_clone = local_songs.clone();
        let number_of_fetch_songs = Arc::new(RwLock::<u32>::new(500));
        let number_of_fetch_songs_clone = number_of_fetch_songs.clone();
        // Spawn the background thread
        thread::spawn(move || {
            Self::background_process(
                runtime,
                osu,
                rx_control,
                tx_update,
                local_songs_clone,
                number_of_fetch_songs_clone,
            );
        });

        let mut app = Self {
            number_of_fetch_songs: number_of_fetch_songs,
            songs_path: songs_path,
            local_songs: local_songs,
            new_songs: HashSet::new(),
            tx_control,
            rx_update,
        };
        app.load_songs_from_local();

        Box::new(app)
    }

    // The background process logic
    fn background_process(
        runtime: Runtime,
        osu: Osu,
        rx: Receiver<bool>,
        tx: Sender<HashSet<u32>>,
        local_songs: Arc<RwLock<HashSet<u32>>>,
        number_of_fetch_songs: Arc<RwLock<u32>>,
    ) {
        loop {
            // Check for incoming commands
            if let Ok(_) = rx.try_recv() {
                // println!("receive");
                let mut new_songs = HashSet::new();
                let mut result = runtime
                    .block_on(
                        osu.beatmapset_search()
                            .mode(GameMode::Osu)
                            .status(Some(RankStatus::Ranked)),
                    )
                    .unwrap();
                let n: u32 = *number_of_fetch_songs.read().unwrap() / 50; // copy value
                println!("{}", n);
                for _ in 1..=n {
                    if !result.has_more() {
                        break;
                    }

                    for beatmap in result.mapsets.iter() {
                        if !local_songs.read().unwrap().contains(&beatmap.mapset_id) {
                            new_songs.insert(beatmap.mapset_id);
                        }
                    }

                    result = runtime.block_on(result.get_next(&osu)).unwrap().unwrap();
                    thread::sleep(Duration::from_millis(10));
                }
                let _ = tx.send(new_songs);
            }
            // Sleep to simulate work and avoid busy-waiting
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn load_songs_from_local(&mut self) {
        let entries = fs::read_dir(&self.songs_path).unwrap();

        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if let Some(folder_name) = path.file_name() {
                if let Some(song) = folder_name.to_str() {
                    // Check if the first element exists and print it
                    if let Some(song_id) = song.split_whitespace().next() {
                        // if song_ids.contains(song_id) {
                        //     println!("Duplicate song {}", song_id);
                        // }
                        self.local_songs
                            .write()
                            .unwrap()
                            .insert(song_id.to_string().parse().unwrap());
                    }
                }
            }
        }

        // Print all the unique song IDs
        // println!("Unique Song IDs: {:?}", song_ids.len());
    }

    fn list_new_songs(&mut self, ui: &mut egui::Ui) {
        if let Ok(new_songs) = self.rx_update.try_recv() {
            // println!("receive song {:?}", new_songs);
            self.new_songs = new_songs;
        }

        let available_width = ui.available_width();
        // Create a box with a scrollable list of items
        egui::Frame::NONE
            .inner_margin(egui::Margin::same(20)) // Optional padding inside the box
            .show(ui, |ui| {
                // Add a scroll area to allow scrolling through the items
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Create a vertical layout for the list items
                    ui.vertical(|ui| {
                        for song in self.new_songs.iter() {
                            ui.set_min_width(available_width);
                            ui.label(format!("{}", song));
                        }
                    });
                });
            });
    }

    fn find_new_songs(&self) {
        let _ = self.tx_control.send(true);
    }

    fn save_to_file(&self) {
        let mut file = fs::File::create("output").unwrap(); // Create (or overwrite) a file
        for value in self.new_songs.iter() {
            writeln!(file, "{}", value).unwrap(); // Write the value followed by a newline
        }
    }
}

impl eframe::App for BeatmapDownloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("osu! Beatmap Downloader");
            ui.horizontal(|ui| {
                let songs_path_label = ui.label("Songs path: ");
                ui.text_edit_singleline(&mut self.songs_path)
                    .labelled_by(songs_path_label.id);
            });

            let mut number_of_fetch_songs = *self.number_of_fetch_songs.read().unwrap();
            ui.add(
                egui::Slider::new(&mut number_of_fetch_songs, 50..=1500)
                    .text("Number of fetch songs"),
            );
            // Manually round the value to the nearest step of 50
            *self.number_of_fetch_songs.write().unwrap() = (number_of_fetch_songs / 50) * 50;
            if ui.button("Reload local songs").clicked() {
                self.load_songs_from_local();
            }
            ui.label(format!("Songs Path '{}'", self.songs_path));
            ui.label(format!(
                "Number of Local songs '{}'",
                self.local_songs.read().unwrap().len()
            ));

            if ui.button("Find new beatmaps").clicked() {
                self.find_new_songs()
            }
            if ui.button("Save to file").clicked() {
                self.save_to_file()
            }

            self.list_new_songs(ui);
        });
    }
}
