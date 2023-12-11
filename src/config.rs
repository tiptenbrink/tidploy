use crate::errors::FileError;
use serde::Deserialize;
use std::fs;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub(crate) enum ConfigError {
    #[error("Failure during download dealing with files!")]
    File(#[from] FileError),
    #[error("Failed to parse config TOML!")]
    TOMLDecode(#[from] toml::de::Error),
}

#[derive(Deserialize)]
pub(crate) struct DployConfig {
    secrets: DploySecrets,
    info: DployInfo,
}

impl DployConfig {
    pub(crate) fn get_secrets(self) -> Vec<String> {
        self.secrets.ids
    }

    pub(crate) fn latest_ref(self) -> String {
        self.info.latest
    }
}

#[derive(Deserialize)]
struct DployInfo {
    latest: String,
}

#[derive(Deserialize)]
struct DploySecrets {
    ids: Vec<String>,
}

pub(crate) fn load_dploy_config(file_path: &str) -> Result<DployConfig, ConfigError> {
    let toml_file = fs::read_to_string(file_path).map_err(FileError::IO)?;

    let dploy_config: DployConfig = toml::from_str(&toml_file).map_err(ConfigError::TOMLDecode)?;

    Ok(dploy_config)
}
