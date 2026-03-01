use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;
use std::{fs, io};

use chrono::{DateTime, Utc};
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};

use super::website::WebsiteError;

const ANIMATED_EXTENSIONS: &[&str] = &["gif", "webm", "mp4"];
pub const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "webm", "mp4"];

const THUMB_SIZE: u32 = 350;

pub struct GenerationResult {
    pub media_size: u64,
    pub thumb_size: u64,
    pub image_url: String,
}

pub struct WebsiteMedia {
    pub filename: String,
    pub title: String,
    pub description: String,
    pub datetime: String,
    pub extension: String,
    pub source_path: PathBuf,
}

impl WebsiteMedia {
    pub fn from_path(path: &Path, datetime: Option<&str>) -> Option<Self> {
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

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        // Check for a sidecar .txt file next to the media file
        let (title, description) = {
            let txt_path = path.with_extension("txt");
            if txt_path.is_file() {
                let content = fs::read_to_string(&txt_path).unwrap_or_default();
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() >= 2 {
                    let title = lines[0].trim().to_string();
                    let description = lines[1..].join("\n").trim().to_string();
                    (title, description)
                } else if lines.len() == 1 {
                    (stem.clone(), lines[0].trim().to_string())
                } else {
                    (stem.clone(), String::new())
                }
            } else {
                (stem.clone(), String::new())
            }
        };

        let datetime = match datetime {
            Some(dt) => dt.to_string(),
            None => fs::metadata(path)
                .and_then(|m| m.modified())
                .map(format_system_time)
                .unwrap_or_else(|_| "1970-01-01T00:00:00".to_string()),
        };

        Some(Self {
            filename,
            title,
            description,
            datetime,
            extension,
            source_path: path.to_path_buf(),
        })
    }

    fn is_animated(&self) -> bool {
        ANIMATED_EXTENSIONS.contains(&self.extension.as_str())
    }

    pub fn thumb_filename(&self) -> String {
        let stem = Path::new(&self.filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.filename);
        if self.is_animated() {
            format!("{}_thumb.webp", stem)
        } else {
            format!("{}_thumb.{}", stem, self.extension)
        }
    }

    pub fn copy_and_generate_thumb(
        &self,
        dest_media: &Path,
    ) -> Result<GenerationResult, WebsiteError> {
        let dest_file = dest_media.join(&self.filename);
        fs::copy(&self.source_path, &dest_file)
            .map_err(|e| WebsiteError::Io(dest_file.clone(), e))?;

        let thumb_path = dest_media.join(self.thumb_filename());

        if self.is_animated() {
            self.generate_ffmpeg_thumb(&thumb_path)?;
        } else {
            self.generate_image_thumb(&thumb_path)?;
        }

        let media_size = fs::metadata(&dest_file)
            .map_err(|e| WebsiteError::Io(dest_file.clone(), e))?
            .len();
        let thumb_size = fs::metadata(&thumb_path)
            .map_err(|e| WebsiteError::Io(thumb_path.clone(), e))?
            .len();

        Ok(GenerationResult {
            media_size,
            thumb_size,
            image_url: String::new(), // filled in by scan_and_copy_media
        })
    }

    fn generate_image_thumb(&self, thumb_path: &Path) -> Result<(), WebsiteError> {
        let mut decoder = ImageReader::open(&self.source_path)
            .map_err(|e| WebsiteError::Io(self.source_path.clone(), e))?
            .into_decoder()
            .map_err(|e| WebsiteError::Image(self.source_path.clone(), e))?;
        let orientation = decoder
            .orientation()
            .map_err(|e| WebsiteError::Image(self.source_path.clone(), e))?;
        let mut img = DynamicImage::from_decoder(decoder)
            .map_err(|e| WebsiteError::Image(self.source_path.clone(), e))?;
        img.apply_orientation(orientation);

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
            .map_err(|e: io::Error| {
                if e.kind() == io::ErrorKind::NotFound {
                    WebsiteError::FfmpegNotFound(e.to_string())
                } else {
                    WebsiteError::Ffmpeg(self.source_path.clone(), e.to_string())
                }
            })?;

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
            "            {{ \"image_url\": \"{}\", \"thumb_url\": \"{}\", \"title\": \"{}\", \"description\": \"{}\", \"datetime\": \"{}\" }}",
            escape_js(&image_url),
            escape_js(&thumb_url),
            escape_js(&self.title),
            escape_js(&self.description),
            escape_js(&self.datetime),
        )
    }

    pub fn image_url(&self, base_url: &str, media_dir: &str) -> String {
        format!("{}/{}/{}", base_url, media_dir, self.filename)
    }

    pub fn to_rss_item(&self, base_url: &str, media_dir: &str) -> String {
        let base_url = base_url.trim_end_matches('/');
        let image_url = self.image_url(base_url, media_dir);
        let item_link = format!("{}/#media={}", base_url, self.filename);
        let pub_date = datetime_to_rfc2822(&self.datetime);
        let mime = mime_type(&self.extension);
        let description = self.description.as_str();

        let media_html = if self.is_animated() && matches!(self.extension.as_str(), "webm" | "mp4")
        {
            format!("<video src=\"{}\" controls></video>", image_url)
        } else {
            format!(
                "<img src=\"{}\" alt=\"{}\"/>",
                image_url,
                escape_html_attr(&self.title)
            )
        };

        let html_content = format!(
            "<![CDATA[<h2>{}</h2>{}<p>{}</p>]]>",
            escape_html(&self.title),
            media_html,
            escape_html(description),
        );

        format!(
            "        <item>\n            <title>{}</title>\n            <link>{}</link>\n            <guid>{}</guid>\n            <pubDate>{}</pubDate>\n            <enclosure url=\"{}\" type=\"{}\" length=\"0\"/>\n            <description>{}</description>\n        </item>",
            escape_xml(&self.title),
            escape_xml(&item_link),
            escape_xml(&image_url),
            pub_date,
            escape_xml(&image_url),
            mime,
            html_content,
        )
    }
}

fn mime_type(extension: &str) -> &'static str {
    match extension {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "webm" => "video/webm",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn datetime_to_rfc2822(datetime: &str) -> String {
    use chrono::NaiveDateTime;
    NaiveDateTime::parse_from_str(datetime, "%Y-%m-%dT%H:%M:%S")
        .map(|ndt| {
            let dt = ndt.and_utc();
            dt.format("%a, %d %b %Y %H:%M:%S +0000").to_string()
        })
        .unwrap_or_else(|_| datetime.to_string())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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
