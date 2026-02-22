use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use image::imageops::FilterType;
use image::ImageFormat;

use super::website::WebsiteError;

const ANIMATED_EXTENSIONS: &[&str] = &["gif", "webm", "mp4"];
const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "webp", "gif", "webm", "mp4"];

const THUMB_SIZE: u32 = 350;

pub struct WebsiteItem {
    pub filename: String,
    pub title: String,
    pub datetime: String,
    pub extension: String,
    pub source_path: PathBuf,
}

impl WebsiteItem {
    pub fn from_path(path: &Path) -> Option<Self> {
        if !path.is_file() {
            return None;
        }

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())?;

        let is_supported = SUPPORTED_EXTENSIONS.contains(&extension.as_str());

        if !is_supported {
            return None;
        }

        let filename = path.file_name().and_then(|n| n.to_str())?.to_string();

        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let datetime = fs::metadata(path)
            .and_then(|m| m.modified())
            .map(format_system_time)
            .unwrap_or_else(|_| "1970-01-01T00:00:00".to_string());

        Some(Self {
            filename,
            title,
            datetime,
            extension,
            source_path: path.to_path_buf(),
        })
    }

    fn is_animated(&self) -> bool {
        ANIMATED_EXTENSIONS.contains(&self.extension.as_str())
    }

    pub fn thumb_filename(&self) -> String {
        if self.is_animated() {
            format!("{}_thumb.webp", self.title)
        } else {
            format!("{}_thumb.{}", self.title, self.extension)
        }
    }

    pub fn copy_and_generate_thumb(&self, dest_media: &Path) -> Result<(), WebsiteError> {
        let dest_file = dest_media.join(&self.filename);
        fs::copy(&self.source_path, &dest_file).map_err(|e| WebsiteError::Io(dest_file, e))?;

        let thumb_path = dest_media.join(self.thumb_filename());

        if self.is_animated() {
            self.generate_ffmpeg_thumb(&thumb_path)?;
        } else {
            self.generate_image_thumb(&thumb_path)?;
        }

        Ok(())
    }

    fn generate_image_thumb(&self, thumb_path: &Path) -> Result<(), WebsiteError> {
        let img = image::open(&self.source_path)
            .map_err(|e| WebsiteError::Image(self.source_path.clone(), e))?;

        let thumb = center_crop_resize(&img, THUMB_SIZE);

        let format = match self.extension.as_str() {
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "webp" => ImageFormat::WebP,
            _ => ImageFormat::Jpeg,
        };

        thumb
            .save_with_format(thumb_path, format)
            .map_err(|e| WebsiteError::Image(thumb_path.to_path_buf(), e))?;

        Ok(())
    }

    /// Generate an animated thumbnail for a gif/video file (gif, webm, mp4).
    /// Uses ffmpeg to produce a center-cropped 350x350 animated WebP of max 2 seconds.
    fn generate_ffmpeg_thumb(&self, thumb_path: &Path) -> Result<(), WebsiteError> {
        let filter = format!(
            "crop=min(iw\\,ih):min(iw\\,ih):(iw-min(iw\\,ih))/2:(ih-min(iw\\,ih))/2,scale={}:{}",
            THUMB_SIZE, THUMB_SIZE
        );

        let output = Command::new("ffmpeg")
            .args([
                "-i",
                self.source_path.to_str().unwrap_or(""),
                "-t",
                "2",
                "-vf",
                &filter,
                "-c:v",
                "libwebp_anim",
                "-loop",
                "0",
                "-an",
                "-y",
                thumb_path.to_str().unwrap_or(""),
            ])
            .output()
            .map_err(|e| WebsiteError::Ffmpeg(self.source_path.clone(), e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WebsiteError::Ffmpeg(
                self.source_path.clone(),
                format!("ffmpeg exited with {}: {}", output.status, stderr),
            ));
        }

        Ok(())
    }

    pub fn to_json_entry(&self, base_url: &str, media_dir: &str) -> String {
        let image_url = format!("{}/{}/{}", base_url, media_dir, self.filename);
        let thumb_url = format!("{}/{}/{}", base_url, media_dir, self.thumb_filename());

        format!(
            "            {{ \"image_url\": \"{}\", \"thumb_url\": \"{}\", \"title\": \"{}\", \"description\": \"\", \"datetime\": \"{}\" }}",
            escape_js(&image_url),
            escape_js(&thumb_url),
            escape_js(&self.title),
            escape_js(&self.datetime),
        )
    }
}

fn center_crop_resize(img: &image::DynamicImage, size: u32) -> image::DynamicImage {
    let (width, height) = (img.width(), img.height());
    let min_dim = width.min(height);

    let x = (width - min_dim) / 2;
    let y = (height - min_dim) / 2;

    let cropped = img.crop_imm(x, y, min_dim, min_dim);
    cropped.resize_exact(size, size, FilterType::Lanczos3)
}

fn format_system_time(time: SystemTime) -> String {
    let dt: DateTime<Utc> = time.into();
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn escape_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
