use std::fs;

use eframe::{egui, NativeOptions};

use crate::commands::tool::{create_tool, CreateToolArgs};

pub fn main() -> eframe::Result {
    let options = NativeOptions { ..Default::default() };

    let mut command = String::new();
    let mut cwl = String::new();

    eframe::run_simple_native("SciWIn Client", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("This is a test!");
            ui.text_edit_singleline(&mut command);

            if ui.button("Fire!").clicked() {
                let opts = CreateToolArgs {
                    command: shlex::split(&command).unwrap(),
                    ..Default::default()
                };
                if create_tool(&opts).is_ok() {
                    let file = &opts.command[0];
                    let file = format!("workflows/{file}/{file}.cwl");
                    cwl = fs::read_to_string(file).unwrap_or("Failed to read!".to_string());
                }
            }

            ui.text_edit_multiline(&mut cwl);
        });
    })
}
