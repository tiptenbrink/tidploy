use crate::errors::FileError;
use serde::Deserialize;
use std::{fs, path::Path};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub(crate) enum ConfigError {
    #[error("Failure during download dealing with files!")]
    File(#[from] FileError),
    #[error("Failed to parse config TOML!")]
    TOMLDecode(#[from] toml::de::Error),
    #[error("Failed to parse config JSON!")]
    JSONDecode(#[from] serde_json::Error),
    #[error("dployer_env only available when dployer is set")]
    EnvNotDployer
}

#[derive(Deserialize)]
pub(crate) struct DployConfig {
    secrets: DploySecrets,
    info: DployInfo,
    dployer: Option<bool>
}

impl DployConfig {
    pub(crate) fn get_secrets(&self) -> Vec<String> {
        self.secrets.ids.clone()
    }

    pub(crate) fn latest_ref(&self) -> String {
        self.info.latest.clone()
    }

    pub(crate) fn uses_dployer(&self) -> bool {
        self.dployer.clone().unwrap_or(false)
    }

    pub(crate) fn get_dployer_env(&self) -> Result<Option<String>, ConfigError> {
        if !self.uses_dployer() {
            return Err(ConfigError::EnvNotDployer)
        }

        Ok(self.secrets.dployer_env.clone())
    }
}

#[derive(Deserialize)]
struct DployInfo {
    latest: String,
}

#[derive(Deserialize)]
struct DploySecrets {
    ids: Vec<String>,
    dployer_env: Option<String>
}

pub(crate) fn load_dploy_config(file_path_dir: &str) -> Result<DployConfig, ConfigError> {
    let dir_path = Path::new(file_path_dir);
    let toml_path = dir_path.join("tidploy.toml");
    let json_path = dir_path.join("tidploy.json");
    let choose_json = json_path.exists();
    let file_path = if choose_json { json_path } else { toml_path };

    let config_str = fs::read_to_string(file_path).map_err(FileError::IO)?;

    let dploy_config: DployConfig = if choose_json {
        serde_json::from_str(&config_str).map_err(ConfigError::JSONDecode)?
    } else {
        toml::from_str(&config_str).map_err(ConfigError::TOMLDecode)?
    };

    Ok(dploy_config)
}
