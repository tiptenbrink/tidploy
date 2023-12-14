use crate::errors::FileError;
use serde::Deserialize;
use std::{fs, path::Path};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub(crate) enum ConfigError {
    #[error("Failure with files during config! {0}")]
    File(#[from] FileError),
    #[error("Failed to parse config TOML! {0}")]
    TOMLDecode(#[from] toml::de::Error),
    #[error("Failed to parse config JSON! {0}")]
    JSONDecode(#[from] serde_json::Error),
    #[error("env_var must be set if using run or running dployer!")]
    NoEnvVar,
}

#[derive(Deserialize)]
pub(crate) struct DployConfig {
    secrets: DploySecrets,
    info: DployInfo,
    dployer: Option<bool>,
    entrypoint: Option<String>
}

impl DployConfig {
    pub(crate) fn get_secrets(&self) -> Vec<String> {
        self.secrets.ids.clone()
    }

    pub(crate) fn latest_ref(&self) -> String {
        self.info.latest.clone()
    }

    pub(crate) fn uses_dployer(&self) -> bool {
        self.dployer.clone().unwrap_or(true)
    }

    pub(crate) fn get_entrypoint(&self) -> String {
        let default = if self.uses_dployer() { "dployer.sh" } else { "entrypoint.sh" };

        self.entrypoint.clone().unwrap_or(default.to_owned())
    }

    pub(crate) fn get_env_var(&self) -> Option<String> {
        self.secrets.env_var.clone()
    }
}

#[derive(Deserialize)]
struct DployInfo {
    latest: String,
}

#[derive(Deserialize)]
struct DploySecrets {
    ids: Vec<String>,
    env_var: Option<String>,
}

pub(crate) fn load_dploy_config<P: AsRef<Path>>(file_path_dir: P) -> Result<DployConfig, ConfigError> {
    let dir_path = file_path_dir.as_ref();
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
