use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub player: PlayerConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub library: LibraryConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerConfig {
    pub volume: f32,
    pub repeat: String,
    pub shuffle: bool,
    #[serde(default)]
    pub device: Option<String>,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            volume: 0.8,
            repeat: "all".to_string(),
            shuffle: false,
            device: None,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub artwork: bool,
    pub visualizer: String,
    #[serde(default = "default_true")]
    pub notifications: bool,
    #[serde(default = "default_true")]
    pub mpris: bool,
    #[serde(default = "default_true")]
    pub pet: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "Neon Rainbow".to_string(),
            artwork: true,
            visualizer: "bars".to_string(),
            notifications: true,
            mpris: true,
            pet: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LibraryConfig {
    pub paths: Vec<PathBuf>,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        let mut paths = Vec::new();
        if let Some(music) = dirs::audio_dir() {
            paths.push(music);
        }
        Self { paths }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            player: PlayerConfig::default(),
            ui: UiConfig::default(),
            library: LibraryConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("auri").join("config.toml"))
    }

    pub fn load() -> Self {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Self::default(),
        };

        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str::<AppConfig>(&content) {
                return cfg;
            }
        }

        // Create default config file if absent
        let cfg = Self::default();
        let _ = cfg.save();
        cfg
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}
