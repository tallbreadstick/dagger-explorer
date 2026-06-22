use std::fs;
use std::io;

use serde::{Deserialize, Serialize};

use super::paths::preferences_path;
use super::view::{FileViewOptions, ViewMode};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub view_mode: ViewMode,
    #[serde(default)]
    pub show_hidden_files: bool,
    #[serde(default = "default_show_extensions")]
    pub show_file_extensions: bool,
}

fn default_show_extensions() -> bool {
    true
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::default(),
            show_hidden_files: false,
            show_file_extensions: true,
        }
    }
}

impl Preferences {
    pub fn load() -> Self {
        let path = preferences_path();
        let Ok(text) = fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let path = preferences_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, text)
    }

    pub fn apply_to(&self, options: &mut FileViewOptions) {
        options.view_mode = self.view_mode;
        options.show_hidden_files = self.show_hidden_files;
        options.show_file_extensions = self.show_file_extensions;
    }

    pub fn from_view_options(options: &FileViewOptions) -> Self {
        Self {
            view_mode: options.view_mode,
            show_hidden_files: options.show_hidden_files,
            show_file_extensions: options.show_file_extensions,
        }
    }
}
