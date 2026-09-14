use std::fs;
use std::path::PathBuf;

fn ensure_windows_icon() {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by Cargo"),
    );
    let png_path = manifest_dir.join("icons").join("icon.png");
    let ico_path = manifest_dir.join("icons").join("icon.ico");

    println!("cargo:rerun-if-changed={}", png_path.display());

    if ico_path.exists() {
        return;
    }

    // The locked VxClick app icon is a 64x64 RGBA PNG. Windows ICO files can
    // contain PNG-compressed frames, so wrap the exact PNG instead of
    // re-encoding it and changing the artwork.
    let png = fs::read(&png_path).expect("failed to read icons/icon.png");
    let image_offset = 6_u32 + 16_u32;
    let image_size = u32::try_from(png.len()).expect("icon PNG is too large");

    let mut ico = Vec::with_capacity(image_offset as usize + png.len());
    ico.extend_from_slice(&0_u16.to_le_bytes()); // reserved
    ico.extend_from_slice(&1_u16.to_le_bytes()); // image type: icon
    ico.extend_from_slice(&1_u16.to_le_bytes()); // one image

    ico.push(64); // width
    ico.push(64); // height
    ico.push(0); // palette colors
    ico.push(0); // reserved
    ico.extend_from_slice(&1_u16.to_le_bytes()); // color planes
    ico.extend_from_slice(&32_u16.to_le_bytes()); // bits per pixel
    ico.extend_from_slice(&image_size.to_le_bytes());
    ico.extend_from_slice(&image_offset.to_le_bytes());
    ico.extend_from_slice(&png);

    fs::write(&ico_path, ico).expect("failed to generate icons/icon.ico");
}

fn main() {
    #[cfg(target_os = "windows")]
    ensure_windows_icon();

    tauri_build::build()
}
