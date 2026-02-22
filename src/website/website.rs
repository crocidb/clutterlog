use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use super::website_info::{WebsiteInfo, WebsiteInfoError, SITE_TOML};

const DEFAULT_BUILD_DIR: &str = "build";
const DEFAULT_MEDIA_DIR: &str = "media";
const DEFAULT_PUBLIC_DIR: &str = "public";

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg"];

const TEMPLATE_INDEX: &str = include_str!("../../template/index.html");
const TEMPLATE_STYLE: &str = include_str!("../../template/public/style.css");
const TEMPLATE_JS: &str = include_str!("../../template/public/clutterlog.js");

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

    pub fn build(&self) -> Result<(), WebsiteError> {
        let build_path = self.path.join(DEFAULT_BUILD_DIR);
        fs::create_dir_all(&build_path).map_err(|e| WebsiteError::Io(build_path.clone(), e))?;

        let build_media_path = build_path.join(DEFAULT_MEDIA_DIR);
        fs::create_dir_all(&build_media_path)
            .map_err(|e| WebsiteError::Io(build_media_path.clone(), e))?;

        let public_path = build_path.join(DEFAULT_PUBLIC_DIR);
        fs::create_dir_all(&public_path).map_err(|e| WebsiteError::Io(public_path.clone(), e))?;

        // Scan source media directory for images, copy them, and collect data entries
        let source_media_path = self.path.join(DEFAULT_MEDIA_DIR);
        let clutterlog_data = self.scan_and_copy_media(&source_media_path, &build_media_path)?;

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

        Ok(())
    }

    fn scan_and_copy_media(
        &self,
        source_path: &Path,
        dest_path: &Path,
    ) -> Result<String, WebsiteError> {
        let mut entries: Vec<String> = Vec::new();
        let base_url = self.info.url.trim_end_matches('/');

        if !source_path.exists() {
            return Ok("[]".to_string());
        }

        let dir_entries = fs::read_dir(source_path)
            .map_err(|e| WebsiteError::Io(source_path.to_path_buf(), e))?;

        for entry in dir_entries {
            let entry = entry.map_err(|e| WebsiteError::Io(source_path.to_path_buf(), e))?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase());

            let is_image = extension
                .as_ref()
                .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.as_str()));

            if !is_image {
                continue;
            }

            let filename = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            // Get modified time from file metadata
            let datetime = fs::metadata(&path)
                .and_then(|m| m.modified())
                .map(|t| format_system_time(t))
                .unwrap_or_else(|_| "1970-01-01T00:00:00".to_string());

            // Copy file to build/media/
            let dest_file = dest_path.join(&filename);
            fs::copy(&path, &dest_file).map_err(|e| WebsiteError::Io(dest_file, e))?;

            // Build JSON entry
            let image_url = format!("{}/{}/{}", base_url, DEFAULT_MEDIA_DIR, filename);
            entries.push(format!(
                "            {{ \"image_url\": \"{}\", \"title\": \"{}\", \"description\": \"\", \"datetime\": \"{}\" }}",
                escape_js(&image_url),
                escape_js(&title),
                escape_js(&datetime),
            ));
        }

        if entries.is_empty() {
            Ok("[]".to_string())
        } else {
            Ok(format!("[\n{}\n        ]", entries.join(",\n")))
        }
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
        }
    }
}

impl std::error::Error for WebsiteError {}

impl From<WebsiteInfoError> for WebsiteError {
    fn from(err: WebsiteInfoError) -> Self {
        WebsiteError::Info(err)
    }
}

fn format_system_time(time: SystemTime) -> String {
    let dt: DateTime<Utc> = time.into();
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
