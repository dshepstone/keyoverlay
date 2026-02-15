use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ICON_FILES: [(&str, u8); 7] = [
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
fn main() {
    for (file, _) in ICON_FILES {
        println!("cargo:rerun-if-changed={file}");
    }
}

#[cfg(target_os = "windows")]
fn build_windows_icon() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let ico_path = out_dir.join("keyoverlay.ico");

    let mut png_entries = Vec::with_capacity(ICON_FILES.len());
    for (file, size) in ICON_FILES {
        let bytes = fs::read(file).map_err(|e| {
            format!("missing required icon file '{file}' (expected in crates/app): {e}")
        })?;
        validate_png_dimensions(Path::new(file), &bytes, size, size)?;
        png_entries.push((size, bytes));
    }

    let mut ico = Vec::new();
    // ICONDIR
    ico.extend_from_slice(&0u16.to_le_bytes()); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // type = icon
    ico.extend_from_slice(&(png_entries.len() as u16).to_le_bytes()); // count

    let mut offset = (6 + 16 * png_entries.len()) as u32;
    for (size, png_data) in &png_entries {
        let dim = if *size == 256 { 0 } else { *size };
        ico.push(dim); // width
        ico.push(dim); // height
        ico.push(0); // color count
        ico.push(0); // reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // color planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bpp
        ico.extend_from_slice(&(png_data.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += png_data.len() as u32;
    }

    for (_, png_data) in &png_entries {
        ico.extend_from_slice(png_data);
    }

    fs::write(&ico_path, ico)?;

    let mut res = winres::WindowsResource::new();
    res.set_icon(ico_path.to_string_lossy().as_ref());
    res.compile()?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn validate_png_dimensions(
    path: &Path,
    bytes: &[u8],
    expected_w: u8,
    expected_h: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    // PNG signature (8) + IHDR chunk length+type (8) + IHDR data starts at byte 16.
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(format!("{} is not a valid PNG file", path.display()).into());
    }

    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);

    if width != expected_w as u32 || height != expected_h as u32 {
        return Err(format!(
            "{} has dimensions {}x{}, expected {}x{}",
            path.display(),
            width,
            height,
            expected_w,
            expected_h
        )
        .into());
    }

    Ok(())
}
