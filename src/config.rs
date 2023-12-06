use serde::Deserialize;
use std::fs;

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

pub(crate) fn load_dploy_config(file_path: &str) -> DployConfig {
    let toml_file = fs::read_to_string(file_path).unwrap();

    let dploy_config: DployConfig = toml::from_str(&toml_file).unwrap();

    dploy_config
}
