#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod core;
mod screens;
mod unpacker;

pub use app::*;
pub use core::*;
pub use screens::*;
pub use unpacker::*;

use eframe::NativeOptions;

fn main() {
    let icon = image::load_from_memory(include_bytes!("../assets/Lumina.png")).unwrap();
    let icon = egui::IconData {
        width: icon.width(),
        height: icon.height(),
        rgba: icon.into_rgba8().into_raw(),
    };

    let viewport = egui::ViewportBuilder::default()
        .with_icon(icon)
        .with_inner_size([800., 600.]);

    let native_options = NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Lumina",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
    .unwrap()
}
