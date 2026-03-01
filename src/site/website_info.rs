use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const SITE_TOML: &str = "site.toml";
const DEFAULT_DESCRIPTION: &str = "An uncurated timeline of unfinished projects";
const DEFAULT_AUTHOR: &str = "author-name";
const DEFAULT_URL: &str = "https://localhost:8088/";

#[derive(Debug, Serialize, Deserialize)]
pub struct WebsiteInfo {
    pub title: String,
    pub description: String,
    pub author: String,
    pub url: String,
}

impl WebsiteInfo {
    pub fn new(site_title: &str) -> Self {
        Self {
            title: site_title.to_string(),
            description: DEFAULT_DESCRIPTION.to_string(),
            author: DEFAULT_AUTHOR.to_string(),
            url: DEFAULT_URL.to_string(),
        }
    }

    pub fn from_file(path: &Path) -> Result<Self, WebsiteInfoError> {
        let file_path = path.join(SITE_TOML);
        let content = fs::read_to_string(&file_path)
            .map_err(|e| WebsiteInfoError::Io(file_path.clone(), e))?;
        let info: WebsiteInfo = toml::from_str(&content)
            .map_err(|e| WebsiteInfoError::Parse(file_path, Box::new(e)))?;
        Ok(info)
    }
}

/// Error

#[derive(Debug)]
pub enum WebsiteInfoError {
    Io(PathBuf, io::Error),
    Parse(PathBuf, Box<toml::de::Error>),
}

impl std::fmt::Display for WebsiteInfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebsiteInfoError::Io(path, err) => {
                write!(f, "failed to read '{}': {}", path.display(), err)
            }
            WebsiteInfoError::Parse(path, err) => {
                write!(f, "failed to parse '{}': {}", path.display(), err)
            }
        }
    }
}

impl std::error::Error for WebsiteInfoError {}
