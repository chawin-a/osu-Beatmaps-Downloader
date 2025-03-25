// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![windows_subsystem = "windows"]
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use crossbeam::channel;
use eframe::egui;
use egui::{Grid, Hyperlink};
use eyre::{eyre, Result};
use reqwest::header::CONTENT_DISPOSITION;
use rosu_v2::prelude::*;
use serde::Deserialize;
use serde_yaml;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use strfmt::strfmt;
use tokio::runtime::Runtime;

#[derive(Deserialize, Debug)]
struct Config {
    client_id: u64,
    client_secret: String,
    songs_path: String,
    number_of_fetch: u32,
    selected_server: String,
    number_of_simultaneous_downloads: u64,
    server: HashMap<String, String>,
}

fn read_config_from_yaml(file_path: &str) -> Result<Config> {
    let file = std::fs::File::open(file_path)?;
    let config: Config = serde_yaml::from_reader(file)?;
    Ok(config)
}

async fn download_file(
    url: &str,
    file_path: &String,
    default_file_name: String,
    progress: Arc<RwLock<f32>>,
) -> Result<()> {
    // Send a GET request to the URL
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("Mozilla/5.0"),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    let mut response = client.get(url).headers(headers).send().await?;

    let mut file_name = default_file_name; // set default file name to song
    if let Some(content_disposition) = response.headers().get(CONTENT_DISPOSITION) {
        // Parse the header to extract the filename
        if let Ok(content_disposition_str) = content_disposition.to_str() {
            if let Some(name) = content_disposition_str.split("filename=").nth(1) {
                let name = name.trim_matches('"');
                file_name = name.to_owned();
            }
        }
    }
    // Open a file to write the content to
    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded = 0u64;

    let file_path = Path::new(&file_path);
    let dest_path = file_path.join(file_name);
    // Write the content to the file in chunks
    let mut dest_file = std::fs::File::create(dest_path).unwrap();

    while let Some(chunk) = response.chunk().await? {
        downloaded += chunk.len() as u64;

        // Update the progress
        let current = if downloaded < total_size {
            downloaded as f32 / total_size as f32
        } else {
            1.0
        };
        let mut progress = progress.write().unwrap();
        *progress = current;

        // Write the chunk to the file
        dest_file.write_all(&chunk).unwrap();
    }
    Ok(())
}

fn main() -> Result<()> {
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

            Ok(BeatmapDownloaderApp::new(
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

struct BeatmapDownloaderApp {
    number_of_fetch_songs: Arc<RwLock<u32>>,
    songs_path: String,
    local_songs: Arc<RwLock<HashSet<u32>>>,
    new_songs: HashSet<u32>,
    tx_control: Sender<bool>,
    rx_update: Receiver<HashSet<u32>>,
    is_fetching: bool,
    is_download: bool,
    is_download_finish: Arc<RwLock<bool>>,
    selected_server: String,
    number_of_simultaneous_downloads: u64,
    server: HashMap<String, String>,
    runtime: Arc<Runtime>,
    percentage: Arc<RwLock<HashMap<u32, Arc<RwLock<f32>>>>>,
}

impl BeatmapDownloaderApp {
    fn new(
        runtime: Arc<Runtime>,
        osu: Osu,
        songs_path: String,
        number_of_fetch: u32,
        server: HashMap<String, String>,
        selected_server: String,
        number_of_simultaneous_downloads: u64,
    ) -> Box<Self> {
        let (tx_update, rx_update) = mpsc::channel::<HashSet<u32>>();
        let (tx_control, rx_control) = mpsc::channel::<bool>();
        let local_songs = Arc::new(RwLock::new(HashSet::<u32>::new()));
        let local_songs_clone = local_songs.clone();
        let number_of_fetch_songs = Arc::new(RwLock::<u32>::new(number_of_fetch));
        let number_of_fetch_songs_clone = number_of_fetch_songs.clone();
        let runtime_clone = runtime.clone();
        // Spawn the background thread
        thread::spawn(move || {
            Self::background_process(
                runtime_clone,
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
            is_fetching: false,
            is_download: false,
            is_download_finish: Arc::new(RwLock::new(true)),
            selected_server: selected_server,
            server: server,
            runtime: runtime,
            percentage: Arc::new(RwLock::new(HashMap::<u32, Arc<RwLock<f32>>>::new())),
            number_of_simultaneous_downloads,
        };
        app.load_songs_from_local();

        Box::new(app)
    }

    // The background process logic
    fn background_process(
        runtime: Arc<Runtime>,
        osu: Osu,
        rx: Receiver<bool>,
        tx: Sender<HashSet<u32>>,
        local_songs: Arc<RwLock<HashSet<u32>>>,
        number_of_fetch_songs: Arc<RwLock<u32>>,
    ) {
        loop {
            // Check for incoming commands
            if let Ok(_) = rx.try_recv() {
                let mut new_songs = HashSet::new();
                let mut result = runtime
                    .block_on(
                        osu.beatmapset_search()
                            .mode(GameMode::Osu)
                            .status(Some(RankStatus::Ranked)),
                    )
                    .unwrap();
                let n: u32 = *number_of_fetch_songs.read().unwrap() / 50; // copy value
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
                    thread::sleep(Duration::from_millis(1));
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
                        self.local_songs
                            .write()
                            .unwrap()
                            .insert(song_id.to_string().parse().unwrap());
                    }
                }
            }
        }
    }

    fn list_new_songs(&mut self, ui: &mut egui::Ui) {
        if let Ok(new_songs) = self.rx_update.try_recv() {
            // println!("receive song {:?}", new_songs);
            self.is_fetching = false;
            self.new_songs = new_songs;
            self.is_download = false;
            for song in self.new_songs.iter() {
                let mut p = self.percentage.write().unwrap();
                p.insert(*song, Arc::new(RwLock::new(0.0)));
            }
        }

        // Create a box with a scrollable list of items
        egui::Frame::NONE
            .inner_margin(egui::Margin::same(20)) // Optional padding inside the box
            .show(ui, |ui| {
                // Add a scroll area to allow scrolling through the items
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Create a vertical layout for the list items
                    Grid::new("Table")
                        .num_columns(3)
                        .min_col_width(100.0)
                        .max_col_width(1000.0)
                        .show(ui, |ui| {
                            for song in self.new_songs.iter() {
                                ui.set_min_width(6000.0);
                                ui.label(format!("{}", song));
                                let fmt = self.server.get(&self.selected_server).unwrap();
                                let mut selected_server = HashMap::<String, u32>::new();
                                selected_server.insert("beatmap_id".to_string(), *song);
                                let result = strfmt(fmt, &selected_server).unwrap();
                                ui.add(Hyperlink::new(result));
                                if self.is_download {
                                    let percentage_rw = self.percentage.read().unwrap();
                                    let percentage =
                                        percentage_rw.get(song).unwrap().read().unwrap();
                                    ui.add(
                                        egui::ProgressBar::new(*percentage)
                                            .show_percentage()
                                            .animate(true),
                                    );
                                }
                                ui.end_row();
                            }
                        });
                });
            });
    }

    fn find_new_songs(&mut self) {
        if !self.is_fetching {
            self.is_fetching = true;
            let _ = self.tx_control.send(true);
        }
    }

    fn download_v2(&mut self) {
        if !self.is_download {
            self.is_download = true;
            {
                //
                let mut is_download_finish = self.is_download_finish.write().unwrap();
                *is_download_finish = false;
            }
            let fmt = self.server.get(&self.selected_server).unwrap().to_owned();
            let (sender, receiver) = channel::bounded::<u32>(5);
            let mut handlers = vec![];
            for _ in 1..=self.number_of_simultaneous_downloads {
                // Create 5 consumer thread
                let runtime = self.runtime.clone();
                let receiver = receiver.clone();
                let fmt = fmt.clone();
                let percentage = self.percentage.clone();
                let songs_path = self.songs_path.clone();
                handlers.push(thread::spawn(move || {
                    // TODO: Download here
                    while let Ok(song) = receiver.recv() {
                        let mut params = HashMap::<String, u32>::new();
                        params.insert("beatmap_id".to_string(), song);
                        let url = strfmt(&fmt, &params).unwrap();

                        let progress_rw = percentage.read().unwrap();
                        let progress = progress_rw.get(&song).unwrap();
                        let _ = runtime.block_on(download_file(
                            &url,
                            &songs_path,
                            format!("{}.osz", song),
                            progress.clone(),
                        ));
                    }
                }));
            }
            let new_songs = self.new_songs.clone();
            let is_download_finish = self.is_download_finish.clone();
            // TODO: fix should wait until all threads are finish then update is_download_finish to "true"
            thread::spawn(move || {
                // Producer thread
                for song in new_songs.iter() {
                    sender.send(*song).unwrap();
                }
                drop(sender);
                for handler in handlers {
                    handler.join().unwrap(); // Unwrap the result to handle any potential panics
                }
                let mut cur = is_download_finish.write().unwrap();
                *cur = true;
            });
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

            ui.horizontal(|ui| {
                let simulteneous_downloads = ui.label("Simultaneous downloads: ");
                ui.add(egui::DragValue::new(
                    &mut self.number_of_simultaneous_downloads,
                ))
                .labelled_by(simulteneous_downloads.id);
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
            let status = if self.is_fetching { "loading" } else { "idle" };
            ui.label(format!("Status: {}", status));
            ui.label(format!("Found {} songs", self.new_songs.len()));
            let options = self.server.keys().cloned().collect::<Vec<String>>();
            egui::ComboBox::from_label("Select an Option")
                .selected_text(self.selected_server.clone())
                .show_ui(ui, |ui| {
                    for option in options {
                        ui.selectable_value(&mut self.selected_server, option.to_string(), option);
                    }
                });
            ui.label(format!("You selected: {}", self.selected_server));

            // Create a column layout with 2 columns
            ui.columns(10, |columns| {
                if columns[0].button("Find new beatmaps").clicked() {
                    self.is_download = false;
                    self.percentage =
                        Arc::new(RwLock::new(HashMap::<u32, Arc<RwLock<f32>>>::new()));
                    self.find_new_songs()
                }
                // First column
                if columns[1].button("Download").clicked() {
                    self.download_v2();
                }
                let result = if *self.is_download_finish.read().unwrap() {
                    "Finish"
                } else {
                    "Downloading..."
                };
                columns[2].label(result);
            });

            self.list_new_songs(ui);
        });
    }
}
