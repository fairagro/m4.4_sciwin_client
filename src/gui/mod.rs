use eframe::{
    egui::{self, CentralPanel, SidePanel},
    NativeOptions,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn main() -> eframe::Result {
    let options = NativeOptions { ..Default::default() };

    eframe::run_native("SciWIn Client", options, Box::new(|_| Ok(Box::<App>::default())))
}

struct App {
    workdir: PathBuf,
    current_file: Option<PathBuf>,
}

impl App {
    fn open_file(&mut self, filename: impl AsRef<Path>) {
        self.current_file = Some(filename.as_ref().to_path_buf());
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            workdir: env::current_dir().unwrap(),
            current_file: Default::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        catppuccin_egui::set_theme(ctx, catppuccin_egui::LATTE);
        SidePanel::left("file_sidebar").resizable(true).default_width(200.0).show(ctx, |ui| {
            ui.heading("Files");
            ui.separator();

            for entry in WalkDir::new(&self.workdir).into_iter().filter_map(Result::ok) {
                if entry.file_type().is_file() {
                    let file_name = entry.file_name().to_string_lossy();
                    if let Some(cwl_doc) = file_name.strip_suffix(".cwl") {
                        if ui.button(cwl_doc).clicked() {
                            self.open_file(entry.path());
                        }
                    }
                }
            }
        });

        // Main editor area
        CentralPanel::default().show(ctx, |ui| {
            if let Some(current_file) = &self.current_file {
                ui.heading(current_file.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown file"));
                ui.separator();
                ui.label(format!("Editing: {current_file:?}"));
            } else {
                ui.label("No file selected.");
            }
        });
    }
}
