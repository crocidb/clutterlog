use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::website_info::{SITE_TOML, WebsiteInfo, WebsiteInfoError};

const DEFAULT_MEDIA_DIR: &str = "media";

#[derive(Debug)]
pub struct Website {
    pub info: WebsiteInfo,
    pub path: PathBuf,
}

impl Website {
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
}

// Error

#[derive(Debug)]
pub enum WebsiteError {
    Info(WebsiteInfoError),
    Io(PathBuf, io::Error),
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for WebsiteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
