use anyhow::{Context, Result};
use log::info;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub theme: Theme,
    pub font_size: f32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    #[serde(rename = "light")]
    Light,

    #[serde(rename = "dark")]
    #[default]
    Dark,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            theme: Theme::default(),
            font_size: 16.0,
        }
    }
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("clauchat");

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create config dir")?;
        }

        Ok(config_dir.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        if !config_path.exists() {
            Ok(Self::default())
        } else {
            let config_file = File::open(&config_path).context("Failed to open config file")?;
            let config =
                serde_json::from_reader(config_file).context("Could not deserialize config")?;
            info!("Configuration loaded from {}", config_path.display());
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let json = serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        let mut file = File::create(&config_path)?;
        file.write_all(json.as_bytes())
            .context("Failed to write to file")?;
        info!("Configuration saved to {}", config_path.display());
        Ok(())
    }

}
