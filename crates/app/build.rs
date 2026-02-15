use std::env;
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use image::GenericImageView;

#[cfg(target_os = "windows")]
const ICON_FILES: [(&str, u16); 7] = [
    ("icon_16px.png", 16),
    ("icon_24px.png", 24),
    ("icon_32px.png", 32),
    ("icon_48px.png", 48),
    ("icon_64px.png", 64),
    ("icon_128px.png", 128),
    ("icon_256px.png", 256),
];

#[cfg(target_os = "windows")]
fn main() {
    for (file, _) in ICON_FILES {
        println!("cargo:rerun-if-changed={file}");
    }

    build_windows_icon().expect("failed to generate or embed keyoverlay.ico");
}

#[cfg(not(target_os = "windows"))]
fn main() {}

#[cfg(target_os = "windows")]
fn build_windows_icon() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let ico_path = out_dir.join("keyoverlay.ico");

    let mut png_entries: Vec<(u16, Vec<u8>)> = Vec::with_capacity(ICON_FILES.len());

    for (file, expected_size) in ICON_FILES {
        let bytes = fs::read(file)
            .unwrap_or_else(|e| panic!("failed to read required icon file '{file}': {e}"));

        let image = image::load_from_memory(&bytes)
            .unwrap_or_else(|e| panic!("failed to decode PNG '{file}': {e}"));
        let (width, height) = image.dimensions();

        if width != height || width != expected_size as u32 {
            panic!(
                "icon file '{file}' must be square and {expected_size}x{expected_size}, found {width}x{height}"
            );
        }

        png_entries.push((expected_size, bytes));
    }

    let mut ico = Vec::new();
    // ICONDIR header
    ico.extend_from_slice(&0u16.to_le_bytes()); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // type = icon
    ico.extend_from_slice(&(png_entries.len() as u16).to_le_bytes()); // image count

    // ICONDIRENTRY table starts after ICONDIR + all entries
    let mut offset = (6 + 16 * png_entries.len()) as u32;
    for (size, png_data) in &png_entries {
        let dim_byte: u8 = if *size == 256 { 0 } else { *size as u8 };
        ico.push(dim_byte); // width (0 means 256)
        ico.push(dim_byte); // height (0 means 256)
        ico.push(0); // color palette count
        ico.push(0); // reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // color planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico.extend_from_slice(&(png_data.len() as u32).to_le_bytes()); // image size
        ico.extend_from_slice(&offset.to_le_bytes()); // image offset
        offset += png_data.len() as u32;
    }

    // Image data blobs
    for (_, png_data) in &png_entries {
        ico.extend_from_slice(png_data);
    }

    fs::write(&ico_path, ico)?;

    let mut res = winres::WindowsResource::new();
    res.set_icon(ico_path.to_string_lossy().as_ref());
    res.compile()?;

    Ok(())
}
