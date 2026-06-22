use std::io::{self, Cursor, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use image::ImageReader;
use image::imageops::FilterType;

use super::db::{ThumbnailDb, ThumbnailRecord};
use super::ffmpeg::resolve_ffmpeg;
use super::media::MediaKind;

pub const THUMB_MAX_PX: u32 = 128;

pub fn mtime_ns(modified: SystemTime) -> u128 {
    modified
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

pub fn generate_thumbnail(
    db: &ThumbnailDb,
    path: &Path,
    mtime_ns: u128,
    kind: MediaKind,
) -> io::Result<Option<ThumbnailRecord>> {
    if let Some(record) = db.get(path, mtime_ns)? {
        return Ok(Some(record));
    }

    let png = match kind {
        MediaKind::Image => generate_image_png(path)?,
        MediaKind::Video => generate_video_png(path)?,
    };

    let Some(png) = png else {
        return Ok(None);
    };

    let (width, height) = png_dimensions(&png)?;
    db.put(path, mtime_ns, width, height, &png)?;
    Ok(Some(ThumbnailRecord {
        width,
        height,
        png,
    }))
}

fn generate_image_png(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let reader = ImageReader::open(path).map_err(io::Error::other)?;
    let image = reader.decode().map_err(io::Error::other)?;
    encode_png(&fit_max_dimension(image)).map(Some)
}

/// Extract one frame via ffmpeg stdout pipe (kernel buffer between processes).
fn generate_video_png(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let Some(ffmpeg) = resolve_ffmpeg() else {
        return Ok(None);
    };

    let path_str = path.to_string_lossy();
    let attempts: &[&[&str]] = &[
        &[
            "-nostdin",
            "-i",
            &path_str,
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-an",
            "-f",
            "image2pipe",
            "-vcodec",
            "png",
            "pipe:1",
        ],
        &[
            "-nostdin",
            "-ss",
            "0.5",
            "-i",
            &path_str,
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-an",
            "-f",
            "image2pipe",
            "-vcodec",
            "png",
            "pipe:1",
        ],
    ];

    for args in attempts {
        if let Some(png) = run_ffmpeg_pipe(&ffmpeg, args)? {
            if png.is_empty() {
                continue;
            }
            let image = image::load_from_memory(&png).map_err(io::Error::other)?;
            return encode_png(&fit_max_dimension(image)).map(Some);
        }
    }

    Ok(None)
}

fn run_ffmpeg_pipe(ffmpeg: &Path, args: &[&str]) -> io::Result<Option<Vec<u8>>> {
    let mut command = Command::new(ffmpeg);
    command
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn()?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("ffmpeg stdout unavailable"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("ffmpeg stderr unavailable"))?;

    let mut png = Vec::new();
    stdout.read_to_end(&mut png)?;

    let mut err = String::new();
    let _ = stderr.read_to_string(&mut err);

    let status = child.wait()?;
    if !status.success() {
        if !err.is_empty() {
            eprintln!(
                "[dagger-explorer] ffmpeg failed for {}: {}",
                args.iter()
                    .find(|arg| !arg.starts_with('-'))
                    .copied()
                    .unwrap_or("?"),
                err.lines().next().unwrap_or(&err)
            );
        }
        return Ok(None);
    }

    Ok(Some(png))
}

/// Scale down so the longest side is at most `THUMB_MAX_PX`, preserving aspect ratio.
fn fit_max_dimension(image: image::DynamicImage) -> image::DynamicImage {
    let (width, height) = (image.width(), image.height());
    let longest = width.max(height);
    if longest <= THUMB_MAX_PX {
        return image;
    }

    let scale = THUMB_MAX_PX as f32 / longest as f32;
    let new_width = ((width as f32 * scale).round() as u32).max(1);
    let new_height = ((height as f32 * scale).round() as u32).max(1);
    image.resize(new_width, new_height, FilterType::Triangle)
}

fn encode_png(image: &image::DynamicImage) -> io::Result<Vec<u8>> {
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(io::Error::other)?;
    Ok(png)
}

fn png_dimensions(png: &[u8]) -> io::Result<(u32, u32)> {
    let image = image::load_from_memory(png).map_err(io::Error::other)?;
    Ok((image.width(), image.height()))
}

pub fn png_to_color_image(png: &[u8]) -> io::Result<egui::ColorImage> {
    let image = image::load_from_memory(png).map_err(io::Error::other)?;
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    ))
}

/// Fit pixel dimensions into a box with `max_side` on the longest edge.
pub fn display_size(pixel_width: u32, pixel_height: u32, max_side: f32) -> egui::Vec2 {
    if pixel_width == 0 || pixel_height == 0 {
        return egui::vec2(max_side, max_side);
    }

    let width = pixel_width as f32;
    let height = pixel_height as f32;
    let scale = max_side / width.max(height);
    egui::vec2(width * scale, height * scale)
}
