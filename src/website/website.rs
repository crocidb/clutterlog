use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::website_info::{WebsiteInfo, WebsiteInfoError, SITE_TOML};

const DEFAULT_BUILD_DIR: &str = "build";
const DEFAULT_MEDIA_DIR: &str = "media";
const DEFAULT_PUBLIC_DIR: &str = "public";

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

        let media_path = build_path.join(DEFAULT_MEDIA_DIR);
        fs::create_dir_all(&media_path).map_err(|e| WebsiteError::Io(media_path, e))?;

        let public_path = build_path.join(DEFAULT_PUBLIC_DIR);
        fs::create_dir_all(&public_path).map_err(|e| WebsiteError::Io(public_path.clone(), e))?;

        // Write template files
        let index_path = build_path.join("index.html");
        fs::write(&index_path, TEMPLATE_INDEX).map_err(|e| WebsiteError::Io(index_path, e))?;

        let style_path = public_path.join("style.css");
        fs::write(&style_path, TEMPLATE_STYLE).map_err(|e| WebsiteError::Io(style_path, e))?;

        let js_path = public_path.join("clutterlog.js");
        fs::write(&js_path, TEMPLATE_JS).map_err(|e| WebsiteError::Io(js_path, e))?;

        Ok(())
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
