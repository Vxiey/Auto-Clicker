mod app;
mod theme;

use std::sync::Arc;

use eframe::egui;

pub use app::VxClickApp;

pub fn run_gui() -> Result<(), String> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([980.0, 640.0])
            .with_icon(Arc::new(load_icon()?)),
        ..Default::default()
    };

    eframe::run_native(
        "VxClick",
        native_options,
        Box::new(|cc| Ok(Box::new(VxClickApp::new(cc)))),
    )
    .map_err(|error| error.to_string())
}

fn load_icon() -> Result<egui::IconData, String> {
    let image = image::load_from_memory(include_bytes!("../../assets/icon.png"))
        .map_err(|error| format!("failed to decode app icon: {error}"))?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Ok(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}
