use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use image::{DynamicImage, ImageFormat};
use std::io::Write;

/// Check whether the current terminal supports the Kitty graphics protocol.
pub fn is_kitty() -> bool {
    std::env::var("TERM").ok().as_deref() == Some("xterm-kitty")
        || std::env::var("KITTY_WINDOW_ID").is_ok()
}

/// Display an image from raw bytes (JPEG/PNG/WebP) using Kitty's graphics protocol.
/// `cols` and `rows` specify the terminal cell dimensions (0 = auto).
pub fn display_image_bytes(data: &[u8], cols: u32, rows: u32) -> Result<()> {
    let img = image::load_from_memory(data)?;
    let resized = if cols > 0 && rows > 0 {
        resize_for_terminal(&img, cols, rows)
    } else {
        img
    };
    display_image(&resized, cols, rows)
}

/// Display a `DynamicImage` via Kitty graphics protocol.
pub fn display_image(img: &DynamicImage, cols: u32, rows: u32) -> Result<()> {
    // Encode as PNG into a buffer
    let mut png_buf: Vec<u8> = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_buf), ImageFormat::Png)?;

    let encoded = B64.encode(&png_buf);
    let mut stdout = std::io::stdout().lock();

    // Kitty APC escape: <ESC>_G<payload><ESC>\
    // Split into chunks of 4096 bytes (base64)
    let chunk_size = 4096;
    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(chunk_size)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let more = if i + 1 < chunks.len() { 1 } else { 0 };
        let action = if i == 0 { "a=T," } else { "" };
        let size_str = if i == 0 && (cols > 0 || rows > 0) {
            format!("c={},r={},", cols, rows)
        } else {
            String::new()
        };
        let first = if i == 0 {
            format!("{}{}f=100,m={}", action, size_str, more)
        } else {
            format!("m={}", more)
        };
        write!(stdout, "\x1b_G{};{}\x1b\\", first, chunk)?;
    }

    writeln!(stdout)?;
    stdout.flush()?;
    Ok(())
}

/// Clear any displayed Kitty images in the current terminal.
pub fn clear_images() -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    // Delete all images
    write!(stdout, "\x1b_Ga=d\x1b\\")?;
    stdout.flush()?;
    Ok(())
}

/// Resize image to fit within given dimensions.
pub fn resize_for_terminal(img: &DynamicImage, max_cols: u32, max_rows: u32) -> DynamicImage {
    // Rough approximation: 1 terminal cell ≈ 8x16 pixels
    let max_w = max_cols * 8;
    let max_h = max_rows * 16;
    img.thumbnail(max_w, max_h)
}
