use relative_path::RelativePathBuf;
use serde::Deserialize;
use std::{fs, path::{Path, PathBuf}, io::Error as IOError, ops::ControlFlow};
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct ConfigError {
    pub(crate) msg: String,
    pub(crate) source: ConfigErrorKind
}

#[derive(Debug, ThisError)]
pub(crate) enum ConfigErrorKind {
    #[error("IO error during config load! {0}")]
    IO(#[from] IOError),
    #[error("Failed to parse config TOML! {0}")]
    TOMLDecode(#[from] toml::de::Error),
    #[error("Failed to parse config JSON! {0}")]
    JSONDecode(#[from] serde_json::Error),
    #[error("env_var must be set if using run or running dployer!")]
    NoEnvVar,
}

#[derive(Deserialize)]
pub(crate) struct DployConfig {
    pub(crate) network: Option<bool>,
    pub(crate) repo_url: Option<String>,
    pub(crate) deploy_path: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) vars: Option<Vec<ConfigVar>>,
    pub(crate) exe_name: Option<String>
}

#[derive(Deserialize, Clone)]
pub(crate) struct ConfigVar {
    key: String,
    env_name: String
}

// impl DployConfig {
//     pub(crate) fn get_secrets(&self) -> Vec<String> {
//         self.secrets.ids.clone()
//     }

//     pub(crate) fn latest_ref(&self) -> String {
//         self.info.latest.clone()
//     }

//     pub(crate) fn uses_dployer(&self) -> bool {
//         self.dployer.clone().unwrap_or(true)
//     }

//     pub(crate) fn get_entrypoint(&self) -> String {
//         let default = if self.uses_dployer() { "dployer.sh" } else { "entrypoint.sh" };

//         self.entrypoint.clone().unwrap_or(default.to_owned())
//     }

//     pub(crate) fn get_env_var(&self) -> Option<String> {
//         self.secrets.env_var.clone()
//     }
// }

pub(crate) fn load_dploy_config<P: AsRef<Path>>(file_path_dir: P) -> Result<DployConfig, ConfigErrorKind> {
    let dir_path = file_path_dir.as_ref();
    let toml_path = dir_path.join("tidploy.toml");
    let json_path = dir_path.join("tidploy.json");
    let choose_json = json_path.exists();
    let file_path = if choose_json { json_path } else { toml_path };

    let config_str = fs::read_to_string(file_path)?;

    let dploy_config: DployConfig = if choose_json {
        serde_json::from_str(&config_str)?
    } else {
        toml::from_str(&config_str)?
    };

    Ok(dploy_config)
}

fn overwrite_option<T: Clone>(original: Option<T>, replacing: Option<T>) -> Option<T> {
    if replacing.is_some() {
        return replacing.clone()
    }
    original.clone()
}

fn overwrite_config(root_config: DployConfig, overwrite_config: DployConfig) -> DployConfig {
    DployConfig {
        network: overwrite_option(root_config.network, overwrite_config.network),
        repo_url: overwrite_option(root_config.repo_url, overwrite_config.repo_url),
        deploy_path: overwrite_option(root_config.deploy_path, overwrite_config.deploy_path),
        tag: overwrite_option(root_config.tag, overwrite_config.tag),
        vars: overwrite_option(root_config.vars, overwrite_config.vars),
        exe_name: overwrite_option(root_config.exe_name, overwrite_config.exe_name)    
    }
}

pub(crate) fn traverse_configs(start_path: PathBuf, final_path: RelativePathBuf) -> Result<DployConfig, ConfigError> {
    let root_config = load_dploy_config(start_path).map_err(|source| {
        let msg = format!("Failed to load root config at path {:?} while traversing configs.", start_path);
        ConfigError { msg, source }
    })?;

    let paths: Vec<PathBuf> = final_path.components().scan(RelativePathBuf::new(), |state, component| {
        state.join(component.as_str());
        Some(state.to_path(&start_path))
    }).collect();

    let combined_config = paths.iter().try_fold(root_config, |state, path| {
        let inner_config = load_dploy_config(start_path);

        match inner_config {
            Ok(config) => ControlFlow::Continue(overwrite_config(state, config)),
            Err(source) => {
                let msg = format!("Failed to overwrite config at path {:?}", path);
                ControlFlow::Break(ConfigError { msg, source })
            }
        }
    });
    
    match combined_config {
        ControlFlow::Break(e) => Err(e),
        ControlFlow::Continue(config) => Ok(config)
    }
}