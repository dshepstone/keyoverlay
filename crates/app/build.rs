use std::env;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
fn main() {
    println!("cargo:rerun-if-changed=icon.png");

    build_windows_icon().expect("failed to build Windows icon resources");
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("cargo:rerun-if-changed=icon.png");
}

#[cfg(target_os = "windows")]
fn build_windows_icon() -> Result<(), Box<dyn std::error::Error>> {
    let src_icon = PathBuf::from("icon.png");
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let ico_path = out_dir.join("app.ico");

    let image = image::open(src_icon)?.into_rgba8();
    let sizes = [16u32, 32, 48, 64, 256];

    let mut pngs = Vec::with_capacity(sizes.len());
    for size in sizes {
        let resized =
            image::imageops::resize(&image, size, size, image::imageops::FilterType::Lanczos3);

        let mut png_bytes = Vec::new();
        image::DynamicImage::ImageRgba8(resized)
            .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)?;
        pngs.push((size, png_bytes));
    }

    let mut ico = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&(pngs.len() as u16).to_le_bytes());

    let mut offset = (6 + 16 * pngs.len()) as u32;
    for (size, png_data) in &pngs {
        let icon_dim = if *size == 256 { 0 } else { *size as u8 };
        ico.push(icon_dim);
        ico.push(icon_dim);
        ico.push(0);
        ico.push(0);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(png_data.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += png_data.len() as u32;
    }

    for (_, png_data) in &pngs {
        ico.extend_from_slice(png_data);
    }

    fs::write(&ico_path, ico)?;

    let mut res = winres::WindowsResource::new();
    res.set_icon(ico_path.to_string_lossy().as_ref());
    res.compile()?;

    Ok(())
}
