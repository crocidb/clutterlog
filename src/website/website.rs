use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::website_info::{WebsiteInfo, WebsiteInfoError, SITE_TOML};
use super::website_item::{GenerationResult, WebsiteItem};

const DEFAULT_BUILD_DIR: &str = "build";
const DEFAULT_MEDIA_DIR: &str = "media";
const DEFAULT_PUBLIC_DIR: &str = "public";

const TEMPLATE_INDEX: &str = include_str!("../../template/index.html");
const TEMPLATE_STYLE: &str = include_str!("../../template/public/style.css");
const TEMPLATE_JS: &str = include_str!("../../template/public/clutterlog.js");

pub struct BuildReport {
    pub items_processed: usize,
    pub total_media_size: u64,
    pub total_thumbs_size: u64,
    pub processing_time: Duration,
}

impl BuildReport {
    fn from_results(results: Vec<GenerationResult>, processing_time: Duration) -> Self {
        let items_processed = results.len();
        let total_media_size = results.iter().map(|r| r.media_size).sum();
        let total_thumbs_size = results.iter().map(|r| r.thumb_size).sum();
        Self {
            items_processed,
            total_media_size,
            total_thumbs_size,
            processing_time,
        }
    }
}

impl std::fmt::Display for BuildReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Build report:")?;
        writeln!(f, "  Items processed: {}", self.items_processed)?;
        writeln!(
            f,
            "  Total media size: {}",
            format_size(self.total_media_size)
        )?;
        writeln!(
            f,
            "  Total thumbs size: {}",
            format_size(self.total_thumbs_size)
        )?;
        write!(
            f,
            "  Processing time: {}",
            format_duration(self.processing_time)
        )
    }
}

fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs_f64();
    if total_secs < 1.0 {
        format!("{:.0}ms", duration.as_millis())
    } else if total_secs < 60.0 {
        format!("{:.2}s", total_secs)
    } else {
        let mins = duration.as_secs() / 60;
        let secs = total_secs - (mins as f64 * 60.0);
        format!("{}m {:.2}s", mins, secs)
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Debug)]
pub struct Website {
    pub info: WebsiteInfo,
    pub path: PathBuf,
}

impl Website {
    pub fn load(path: &Path) -> Result<Self, WebsiteError> {
        if !path.join(SITE_TOML).exists() {
            let abs = path
                .canonicalize()
                .map_err(|_| WebsiteError::NotAPath(path.to_path_buf()))?;
            return Err(WebsiteError::NotASite(abs.to_path_buf()));
        }

        let info = WebsiteInfo::from_file(path)?;
        Ok(Self {
            info,
            path: path.to_path_buf(),
        })
    }

    pub fn new(path: &Path) -> Result<Self, WebsiteError> {
        if path.join(SITE_TOML).exists() {
            let info = WebsiteInfo::from_file(path)?;
            Ok(Self {
                info,
                path: path.to_path_buf(),
            })
        } else {
            fs::create_dir_all(path).map_err(|e| WebsiteError::Io(path.to_path_buf(), e))?;

            let media_path = path.join(DEFAULT_MEDIA_DIR);
            fs::create_dir_all(&media_path)
                .map_err(|e| WebsiteError::Io(media_path.to_path_buf(), e))?;

            let title = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("untitled");

            let info = WebsiteInfo::new(title);
            let toml_content = toml::to_string_pretty(&info).map_err(WebsiteError::Serialize)?;
            let file_path = path.join(SITE_TOML);
            fs::write(&file_path, &toml_content).map_err(|e| WebsiteError::Io(file_path, e))?;

            Ok(Self {
                info,
                path: path.to_path_buf(),
            })
        }
    }

    pub fn build(&self) -> Result<BuildReport, WebsiteError> {
        let start = Instant::now();
        let build_path = self.path.join(DEFAULT_BUILD_DIR);
        fs::create_dir_all(&build_path).map_err(|e| WebsiteError::Io(build_path.clone(), e))?;

        let build_media_path = build_path.join(DEFAULT_MEDIA_DIR);
        fs::create_dir_all(&build_media_path)
            .map_err(|e| WebsiteError::Io(build_media_path.clone(), e))?;

        let public_path = build_path.join(DEFAULT_PUBLIC_DIR);
        fs::create_dir_all(&public_path).map_err(|e| WebsiteError::Io(public_path.clone(), e))?;

        // Scan source media directory, copy files, generate thumbnails, and collect data entries
        let source_media_path = self.path.join(DEFAULT_MEDIA_DIR);
        let (clutterlog_data, generation_results) =
            self.scan_and_copy_media(&source_media_path, &build_media_path)?;

        // Render index.html from template
        let rendered = TEMPLATE_INDEX
            .replace("{{title}}", &escape_html(&self.info.title))
            .replace("{{description}}", &escape_html(&self.info.description))
            .replace("{{author}}", &escape_html(&self.info.author))
            .replace("{{clutterlog_data}}", &clutterlog_data);

        let index_path = build_path.join("index.html");
        fs::write(&index_path, &rendered).map_err(|e| WebsiteError::Io(index_path, e))?;

        // Write static assets
        let style_path = public_path.join("style.css");
        fs::write(&style_path, TEMPLATE_STYLE).map_err(|e| WebsiteError::Io(style_path, e))?;

        let js_path = public_path.join("clutterlog.js");
        fs::write(&js_path, TEMPLATE_JS).map_err(|e| WebsiteError::Io(js_path, e))?;

        Ok(BuildReport::from_results(
            generation_results,
            start.elapsed(),
        ))
    }

    fn scan_and_copy_media(
        &self,
        source_path: &Path,
        dest_path: &Path,
    ) -> Result<(String, Vec<GenerationResult>), WebsiteError> {
        let mut entries: Vec<String> = Vec::new();
        let mut results: Vec<GenerationResult> = Vec::new();
        let base_url = self.info.url.trim_end_matches('/');

        if !source_path.exists() {
            return Ok(("[]".to_string(), results));
        }

        let dir_entries = fs::read_dir(source_path)
            .map_err(|e| WebsiteError::Io(source_path.to_path_buf(), e))?;

        for entry in dir_entries {
            let entry = entry.map_err(|e| WebsiteError::Io(source_path.to_path_buf(), e))?;
            let path = entry.path();

            let item = match WebsiteItem::from_path(&path) {
                Some(item) => item,
                None => continue,
            };

            let result = item.copy_and_generate_thumb(dest_path)?;
            results.push(result);
            entries.push(item.to_json_entry(base_url, DEFAULT_MEDIA_DIR));
        }

        let json = if entries.is_empty() {
            "[]".to_string()
        } else {
            format!("[\n{}\n        ]", entries.join(",\n"))
        };

        Ok((json, results))
    }
}

// Error

#[derive(Debug)]
pub enum WebsiteError {
    NotASite(PathBuf),
    NotAPath(PathBuf),
    Info(WebsiteInfoError),
    Io(PathBuf, io::Error),
    Serialize(toml::ser::Error),
    Image(PathBuf, image::ImageError),
    Ffmpeg(PathBuf, String),
    FfmpegNotFound(String),
}

impl std::fmt::Display for WebsiteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebsiteError::NotASite(path) => {
                write!(
                    f,
                    "'{}' is not a clutterlog site (missing site.toml)",
                    path.display()
                )
            }
            WebsiteError::NotAPath(path) => {
                let abs = if path.is_absolute() {
                    path.to_path_buf()
                } else if let Ok(currentdir) = std::env::current_dir() {
                    currentdir.join(path)
                } else {
                    path.to_path_buf()
                };

                write!(f, "'{}' is not a valid path", abs.display())
            }
            WebsiteError::Info(err) => write!(f, "{}", err),
            WebsiteError::Io(path, err) => {
                write!(f, "failed to write '{}': {}", path.display(), err)
            }
            WebsiteError::Serialize(err) => write!(f, "failed to serialize site info: {}", err),
            WebsiteError::Image(path, err) => {
                write!(f, "failed to process image '{}': {}", path.display(), err)
            }
            WebsiteError::Ffmpeg(path, err) => {
                write!(
                    f,
                    "failed to extract video frame from '{}': {}",
                    path.display(),
                    err
                )
            }
            WebsiteError::FfmpegNotFound(err) => {
                write!(
                    f,
                    "looks like `ffmpeg` is not installed: {}",
                    err
                )
            }
        }
    }
}

impl std::error::Error for WebsiteError {}

impl From<WebsiteInfoError> for WebsiteError {
    fn from(err: WebsiteInfoError) -> Self {
        WebsiteError::Info(err)
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
