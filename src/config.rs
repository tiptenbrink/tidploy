use camino::Utf8Path;
use relative_path::{RelativePath, RelativePathBuf};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    io::Error as IOError,
    ops::ControlFlow,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;
use tracing::debug;

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct ConfigError {
    pub(crate) msg: String,
    pub(crate) source: ConfigErrorKind,
}

#[derive(Debug, ThisError)]
pub(crate) enum ConfigErrorKind {
    #[error("IO error during config load! {0}")]
    IO(#[from] IOError),
    #[error("Failed to parse config TOML! {0}")]
    TOMLDecode(#[from] toml::de::Error),
    #[error("Failed to parse config JSON! {0}")]
    JSONDecode(#[from] serde_json::Error),
}

#[derive(Deserialize, Debug)]
pub(crate) struct DployConfig {
    pub(crate) repo_url: Option<String>,
    pub(crate) deploy_path: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) vars: Option<Vec<ConfigVar>>,
    pub(crate) exe_name: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub(crate) struct ConfigVar {
    pub(crate) key: String,
    pub(crate) env_name: String,
}

pub(crate) fn load_dploy_config<P: AsRef<Path>>(
    file_path_dir: P,
) -> Result<DployConfig, ConfigErrorKind> {
    let dir_path = file_path_dir.as_ref();
    let toml_path = dir_path.join("tidploy.toml");
    let json_path = dir_path.join("tidploy.json");
    let choose_json = json_path.exists();
    let file_path = if choose_json { json_path } else { toml_path };

    if !file_path.exists() {
        debug!("No config exists at path {:?}", file_path);
        return Ok(DployConfig {
            repo_url: None,
            deploy_path: None,
            tag: None,
            exe_name: None,
            vars: None,
        });
    }

    let config_str = fs::read_to_string(&file_path)?;

    let dploy_config: DployConfig = if choose_json {
        serde_json::from_str(&config_str)?
    } else {
        toml::from_str(&config_str)?
    };

    debug!("Loaded config at path {:?}: {:?}", file_path, dploy_config);

    Ok(dploy_config)
}

fn overwrite_option<T: Clone>(original: Option<T>, replacing: Option<T>) -> Option<T> {
    if replacing.is_some() {
        return replacing;
    }
    original
}

pub(crate) fn merge_vars(
    root_vars: Option<Vec<ConfigVar>>,
    overwrite_vars: Option<Vec<ConfigVar>>,
) -> Option<Vec<ConfigVar>> {
    if let Some(root_vars) = root_vars {
        if let Some(overwrite_vars) = overwrite_vars {
            let mut vars_map: HashMap<String, String> = root_vars
                .iter()
                .map(|v| (v.key.clone(), v.env_name.clone()))
                .collect();

            for cfg_var in overwrite_vars {
                vars_map.insert(cfg_var.key, cfg_var.env_name);
            }

            Some(
                vars_map
                    .into_iter()
                    .map(|(k, v)| ConfigVar {
                        env_name: v,
                        key: k,
                    })
                    .collect(),
            )
        } else {
            Some(root_vars)
        }
    } else {
        overwrite_vars.clone()
    }
}

fn overwrite_config(root_config: DployConfig, overwrite_config: DployConfig) -> DployConfig {
    DployConfig {
        repo_url: overwrite_option(root_config.repo_url, overwrite_config.repo_url),
        deploy_path: overwrite_option(root_config.deploy_path, overwrite_config.deploy_path),
        tag: overwrite_option(root_config.tag, overwrite_config.tag),
        vars: merge_vars(root_config.vars, overwrite_config.vars),
        exe_name: overwrite_option(root_config.exe_name, overwrite_config.exe_name),
    }
}

/// Looks at config at start_path and appends levels from final_path, looking at a config at every level. It then
/// combines them.
pub(crate) fn traverse_configs(
    start_path: &Utf8Path,
    final_path: &RelativePath,
) -> Result<DployConfig, ConfigError> {
    debug!(
        "Traversing configs from {:?} to relative {:?}",
        start_path, final_path
    );

    let root_config = load_dploy_config(start_path).map_err(|source| {
        let msg = format!(
            "Failed to load root config at path {:?} while traversing configs.",
            start_path
        );
        ConfigError { msg, source }
    })?;

    let paths: Vec<PathBuf> = final_path
        .components()
        .scan(RelativePathBuf::new(), |state, component| {
            state.push(component);
            Some(state.to_path(start_path))
        })
        .collect();

    let combined_config = paths.iter().try_fold(root_config, |state, path| {
        let inner_config = load_dploy_config(path);

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
        ControlFlow::Continue(config) => Ok(config),
    }
}
