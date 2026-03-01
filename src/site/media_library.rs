use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use super::website_item::SUPPORTED_EXTENSIONS;

const CLUTTERLOG_DIR: &str = ".clutterlog";
const METAMEDIA_TOML: &str = "metamedia.toml";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetaMedia {
    pub name: String,
    pub datetime: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetaMediaFile {
    #[serde(default)]
    media: Vec<MetaMedia>,
}

pub struct UpdateReport {
    pub added: usize,
    pub removed: usize,
}

impl std::fmt::Display for UpdateReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} added, {} removed", self.added, self.removed)
    }
}

pub struct MediaLibrary {
    pub entries: Vec<MetaMedia>,
    path: PathBuf,
}

impl MediaLibrary {
    pub fn new(site_path: &Path) -> Result<Self, MediaLibraryError> {
        let dir_path = site_path.join(CLUTTERLOG_DIR);
        let file_path = dir_path.join(METAMEDIA_TOML);

        if file_path.exists() {
            let content = fs::read_to_string(&file_path)
                .map_err(|e| MediaLibraryError::Io(file_path.clone(), e))?;
            let meta_file: MetaMediaFile = toml::from_str(&content)
                .map_err(|e| MediaLibraryError::Parse(file_path.clone(), Box::new(e)))?;
            Ok(Self {
                entries: meta_file.media,
                path: file_path,
            })
        } else {
            fs::create_dir_all(&dir_path)
                .map_err(|e| MediaLibraryError::Io(dir_path.to_path_buf(), e))?;
            Ok(Self {
                entries: Vec::new(),
                path: file_path,
            })
        }
    }

    pub fn update_metadata(
        &mut self,
        media_path: &Path,
    ) -> Result<UpdateReport, MediaLibraryError> {
        let mut added: usize = 0;

        if !media_path.exists() {
            // Remove all entries since there's no media directory
            let removed = self.entries.len();
            self.entries.clear();
            self.save()?;
            return Ok(UpdateReport { added, removed });
        }

        // Collect current media filenames
        let mut current_files: Vec<String> = Vec::new();
        let dir_entries = fs::read_dir(media_path)
            .map_err(|e| MediaLibraryError::Io(media_path.to_path_buf(), e))?;

        for entry in dir_entries {
            let entry = entry.map_err(|e| MediaLibraryError::Io(media_path.to_path_buf(), e))?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let extension = match path.extension().and_then(|ext| ext.to_str()) {
                Some(ext) => ext.to_lowercase(),
                None => continue,
            };

            if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
                continue;
            }

            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                current_files.push(filename.to_string());
            }
        }

        // Add new files that aren't in metadata yet
        for filename in &current_files {
            let already_exists = self.entries.iter().any(|e| e.name == *filename);
            if !already_exists {
                let file_path = media_path.join(filename);
                let datetime = extract_oldest_date(&file_path);
                self.entries.push(MetaMedia {
                    name: filename.clone(),
                    datetime,
                });
                added += 1;
            }
        }

        // Remove stale entries for files that no longer exist
        let before_len = self.entries.len();
        self.entries.retain(|e| current_files.contains(&e.name));
        let removed = before_len - self.entries.len();

        self.save()?;

        Ok(UpdateReport { added, removed })
    }

    pub fn get_datetime(&self, filename: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.name == filename)
            .map(|e| e.datetime.as_str())
    }

    fn save(&self) -> Result<(), MediaLibraryError> {
        let meta_file = MetaMediaFile {
            media: self.entries.clone(),
        };
        let content = toml::to_string_pretty(&meta_file).map_err(MediaLibraryError::Serialize)?;
        fs::write(&self.path, &content).map_err(|e| MediaLibraryError::Io(self.path.clone(), e))?;
        Ok(())
    }
}

fn extract_oldest_date(path: &Path) -> String {
    let mut candidates: Vec<DateTime<Utc>> = Vec::new();

    // Try EXIF metadata for supported image formats
    if let Some(exif_dt) = extract_exif_date(path) {
        candidates.push(exif_dt);
    }

    // Filesystem timestamps
    if let Ok(metadata) = fs::metadata(path) {
        if let Ok(created) = metadata.created() {
            let dt: DateTime<Utc> = created.into();
            candidates.push(dt);
        }
        if let Ok(modified) = metadata.modified() {
            let dt: DateTime<Utc> = modified.into();
            candidates.push(dt);
        }
    }

    match candidates.iter().min() {
        Some(oldest) => oldest.format("%Y-%m-%dT%H:%M:%S").to_string(),
        None => "1970-01-01T00:00:00".to_string(),
    }
}

/// Attempts to extract a date from EXIF metadata embedded in an image file.
/// Checks DateTimeOriginal, DateTimeDigitized, and DateTime fields in that order.
/// Returns `None` for unsupported formats, missing EXIF data, or parse errors.
fn extract_exif_date(path: &Path) -> Option<DateTime<Utc>> {
    let extension = path.extension()?.to_str()?.to_lowercase();
    if !["jpg", "jpeg", "webp", "tiff", "tif"].contains(&extension.as_str()) {
        return None;
    }

    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let exif_data = exif::Reader::new().read_from_container(&mut reader).ok()?;

    let date_tags = [
        exif::Tag::DateTimeOriginal,
        exif::Tag::DateTimeDigitized,
        exif::Tag::DateTime,
    ];

    for tag in &date_tags {
        if let Some(field) = exif_data.get_field(*tag, exif::In::PRIMARY) {
            let value = field.display_value().to_string();
            // EXIF dates are formatted as "YYYY-MM-DD HH:MM:SS"
            if let Ok(naive) = NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S") {
                return Some(naive.and_utc());
            }
        }
    }

    None
}

// Error

#[derive(Debug)]
pub enum MediaLibraryError {
    Io(PathBuf, io::Error),
    Parse(PathBuf, Box<toml::de::Error>),
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for MediaLibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaLibraryError::Io(path, err) => {
                write!(f, "media library I/O error '{}': {}", path.display(), err)
            }
            MediaLibraryError::Parse(path, err) => {
                write!(f, "failed to parse '{}': {}", path.display(), err)
            }
            MediaLibraryError::Serialize(err) => {
                write!(f, "failed to serialize media metadata: {}", err)
            }
        }
    }
}

impl std::error::Error for MediaLibraryError {}
