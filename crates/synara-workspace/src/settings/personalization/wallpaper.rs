//! Bounded local wallpaper preparation. This never captures or reads the desktop.
use super::*;
use image::{DynamicImage, ImageFormat, ImageReader, Limits, imageops::FilterType};
use std::{io::Cursor, path::Path, time::Duration};
use tokio::sync::Semaphore;
static WORKER: Semaphore = Semaphore::const_new(1);

pub(super) async fn read(path: PathBuf, blur: u8) -> WorkspaceResult<WallpaperAsset> {
    let permit = tokio::time::timeout(Duration::from_secs(10), WORKER.acquire())
        .await
        .map_err(|_| invalid("Image reader is busy. Retry the wallpaper."))?
        .map_err(|_| WorkspaceError::Worker)?;
    // The permit lives in the blocking job, so cancelling the caller cannot admit
    // another decoder while this one is still running.
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        prepare(path, blur)
    })
    .await
    .map_err(|_| WorkspaceError::Worker)?
}
fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.into())
}
fn prepare(path: PathBuf, blur: u8) -> WorkspaceResult<WallpaperAsset> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Missing image directory."))?;
    let leaf = path
        .file_name()
        .ok_or_else(|| invalid("Missing image filename."))?;
    let fs = synara_runtime::WorkspaceFs::open(parent)?;
    let name = Path::new(leaf);
    if fs.file_length(name)? > 8 * 1024 * 1024 {
        return Err(invalid("Wallpaper exceeds 8 MiB."));
    }
    let bytes = fs.read_blob(name)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(invalid("Wallpaper exceeds 8 MiB."));
    }
    let (format, width, height) = crate::studio::image_size(&bytes)
        .ok_or_else(|| invalid("Only PNG and JPEG wallpapers are supported."))?;
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > 16_000_000
    {
        return Err(invalid(
            "Wallpaper exceeds 8192 pixels per side or 16 megapixels.",
        ));
    }
    if format == crate::PreviewImageFormat::Png {
        reject_animation(&bytes)?;
    }
    let mut reader = ImageReader::with_format(
        Cursor::new(&bytes),
        match format {
            crate::PreviewImageFormat::Png => ImageFormat::Png,
            crate::PreviewImageFormat::Jpeg => ImageFormat::Jpeg,
        },
    );
    let mut limits = Limits::default();
    limits.max_image_width = Some(width);
    limits.max_image_height = Some(height);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| invalid("Wallpaper is damaged or exceeds decoder limits."))?;
    if decoded.width() != width || decoded.height() != height {
        return Err(invalid("Wallpaper dimensions do not match its header."));
    }
    if blur == 0 {
        return Ok(WallpaperAsset {
            bytes,
            format,
            width,
            height,
        });
    }
    let mut pixels = decoded.resize(1024, 1024, FilterType::Triangle).to_rgba8();
    drop(decoded);
    for pixel in pixels.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in 0..3 {
            pixel[channel] = (u16::from(pixel[channel]) * alpha / 255) as u8;
        }
    }
    let mut pixels = image::imageops::fast_blur(&pixels, f32::from(blur));
    for pixel in pixels.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in 0..3 {
            if let Some(value) = (u16::from(pixel[channel]) * 255).checked_div(alpha) {
                pixel[channel] = value.min(255) as u8;
            }
        }
    }
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(pixels)
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| invalid("Unable to prepare the blurred wallpaper."))?;
    let bytes = output.into_inner();
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(invalid("Prepared wallpaper exceeds 8 MiB."));
    }
    Ok(WallpaperAsset {
        bytes,
        format: crate::PreviewImageFormat::Png,
        width,
        height,
    })
}
fn reject_animation(bytes: &[u8]) -> WorkspaceResult<()> {
    let mut at = 8usize;
    while at.checked_add(12).is_some_and(|end| end <= bytes.len()) {
        let length = u32::from_be_bytes(
            bytes[at..at + 4]
                .try_into()
                .map_err(|_| invalid("Invalid PNG."))?,
        ) as usize;
        if &bytes[at + 4..at + 8] == b"acTL" {
            return Err(invalid("Choose a still PNG or JPEG."));
        }
        let next = at
            .checked_add(12)
            .and_then(|n| n.checked_add(length))
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| invalid("Truncated PNG wallpaper."))?;
        if &bytes[at + 4..at + 8] == b"IEND" {
            return Ok(());
        }
        at = next;
    }
    Err(invalid("PNG wallpaper is missing its end marker."))
}
