use eframe::egui;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn check_config_file() -> bool {
    let config_path = std::path::Path::new("config.yaml");
    config_path.exists()
}

pub fn read_config_from_yaml(file_path: &str) -> Result<Config> {
    let file = std::fs::File::open(file_path)?;
    let config: Config = serde_yaml::from_reader(file)?;
    Ok(config)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub client_id: u64,
    pub client_secret: String,
    pub search_client: String,
    pub songs_path: String,
    pub number_of_fetch: u32,
    pub selected_server: String,
    pub number_of_simultaneous_downloads: u64,
    pub server: HashMap<String, String>,
}

pub struct ConfigApp {
    config: Config,
}

impl ConfigApp {
    pub fn new() -> Self {
        let mut server = HashMap::new();
        server.insert(
            "beatconnect".to_owned(),
            "https://beatconnect.io/b/{beatmap_id}".to_owned(),
        );
        server.insert(
            "nerinyan".to_owned(),
            "https://api.nerinyan.moe/d/{beatmap_id}".to_owned(),
        );
        server.insert(
            "osu_direct".to_owned(),
            "https://osu.direct/api/d/{beatmap_id}".to_owned(),
        );
        server.insert(
            "catboy".to_owned(),
            "https://catboy.best/d/{beatmap_id}".to_owned(),
        );
        server.insert(
            "osu_ppy".to_owned(),
            "https://osu.ppy.sh/beatmapsets/{beatmap_id}/download".to_owned(),
        );
        Self {
            config: Config {
                client_id: 0,
                client_secret: "".to_owned(),
                songs_path: "".to_owned(),
                number_of_fetch: 250,
                selected_server: "nerinyan".to_owned(),
                number_of_simultaneous_downloads: 5,
                server: server,
                search_client: "nerinyan".to_owned(),
            },
        }
    }
}

impl eframe::App for ConfigApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Configuration");

            let options = vec!["nerinyan", "osu", "osu_api"];
            egui::ComboBox::from_label("Select a Search Option")
                .selected_text(self.config.search_client.clone())
                .show_ui(ui, |ui| {
                    for option in options {
                        ui.selectable_value(
                            &mut self.config.search_client,
                            option.to_string(),
                            option,
                        );
                    }
                });
            match self.config.search_client.as_str() {
                "nerinyan" => (),
                "osu" => (),
                "osu_api" => {
                    ui.horizontal(|ui| {
                        let client_id_label = ui.label("Client ID: ");
                        ui.add(egui::DragValue::new(&mut self.config.client_id))
                            .labelled_by(client_id_label.id);
                    });

                    ui.horizontal(|ui| {
                        let client_secret_label = ui.label("Client Secret: ");
                        ui.text_edit_singleline(&mut self.config.client_secret)
                            .labelled_by(client_secret_label.id);
                    });
                }
                _ => (),
            }

            ui.horizontal(|ui| {
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.config.songs_path = path.display().to_string();
                    }
                }
                ui.label(format!("Songs path: {}", &self.config.songs_path));
            });

            if ui.button("Save").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let config_path = "config.yaml";
        let config_file = std::fs::File::create(config_path).unwrap();
        serde_yaml::to_writer(config_file, &self.config).unwrap();
    }
}
