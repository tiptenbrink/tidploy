use std::{
    collections::HashMap,
    fs,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use relative_path::{RelativePath, RelativePathBuf};
use serde::Deserialize;
use tracing::debug;

use crate::{next::errors::WrapConfigErr, state::State};

use super::errors::ConfigError;

#[derive(Deserialize, Clone, Debug)]
pub(crate) struct ConfigVar {
    pub(crate) key: String,
    pub(crate) env_name: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ConfigScope {
    pub(crate) name: Option<String>,
    pub(crate) sub: Option<String>,
    pub(crate) service: Option<String>,
    pub(crate) require_hash: Option<bool>
}

#[derive(Deserialize, Debug, Default)]
pub(crate) struct ArgumentConfig {
    pub(crate) scope: Option<ConfigScope>,
    pub(crate) executable: Option<String>,
    pub(crate) execution_path: Option<String>,
    pub(crate) envs: Option<Vec<ConfigVar>>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum ConfigAddress {
    Local { path: String },
    Git { url: String, git_ref: String }
}

#[derive(Deserialize, Debug)]
pub(crate) struct StateConfig {
    pub(crate) state_root: Option<String>,
    pub(crate) state_path: Option<String>,
    pub(crate) address: Option<ConfigAddress>,
}

#[derive(Deserialize, Debug, Default)]
pub(crate) struct Config {
    pub(crate) argument: Option<ArgumentConfig>,
    pub(crate) state: Option<StateConfig>
}

pub(crate) fn load_dploy_config(config_dir_path: &Path) -> Result<Config, ConfigError> {
    let toml_path = config_dir_path.join("tidploy.toml");
    let json_path = config_dir_path.join("tidploy.json");
    let choose_json = json_path.exists();
    let file_path = if choose_json { json_path } else { toml_path };

    if !file_path.exists() {
        debug!("No config exists at path {:?}", file_path);
        return Ok(Config::default());
    }

    let config_str = fs::read_to_string(&file_path)
        .to_config_err(format!("Failed to read config file at {:?}", &file_path))?;

    let dploy_config: Config = if choose_json {
        serde_json::from_str(&config_str).to_config_err(format!(
            "Failed to deserialize file {:?} to JSON",
            &file_path
        ))?
    } else {
        toml::from_str(&config_str).to_config_err(format!(
            "Failed to deserialize file {:?} to JSON",
            &file_path
        ))?
    };

    debug!("Loaded config at path {:?}: {:?}", file_path, dploy_config);

    Ok(dploy_config)
}

pub(crate) fn overwrite_option<T>(original: Option<T>, replacing: Option<T>) -> Option<T> {
    match replacing {
        Some(replacing) => Some(replacing),
        None => original,
    }
}

pub(crate) fn merge_option<T>(
    original: Option<T>,
    replacing: Option<T>,
    merge_fn: &dyn Fn(T, T) -> T,
) -> Option<T> {
    match original {
        Some(original) => match replacing {
            Some(replacing) => Some(merge_fn(original, replacing)),
            None => Some(original),
        },
        None => replacing,
    }
}

fn overwrite_scope(original: ConfigScope, replacing: ConfigScope) -> ConfigScope {
    ConfigScope {
        name: overwrite_option(original.name, replacing.name),
        sub: overwrite_option(original.sub, replacing.sub),
        service: overwrite_option(original.service, replacing.service),
        require_hash: overwrite_option(original.require_hash, replacing.require_hash)
    }
}

pub(crate) fn merge_vars(
    root_vars: Vec<ConfigVar>,
    overwrite_vars: Vec<ConfigVar>,
) -> Vec<ConfigVar> {
    let mut vars_map: HashMap<String, String> = root_vars
        .iter()
        .map(|v| (v.key.clone(), v.env_name.clone()))
        .collect();

    for cfg_var in overwrite_vars {
        vars_map.insert(cfg_var.key, cfg_var.env_name);
    }

    vars_map
        .into_iter()
        .map(|(k, v)| ConfigVar {
            env_name: v,
            key: k,
        })
        .collect()
}

fn overwrite_arguments(
    root_config: ArgumentConfig,
    overwrite_config: ArgumentConfig,
) -> ArgumentConfig {
    let scope = merge_option(root_config.scope, overwrite_config.scope, &overwrite_scope);

    let execution_path =
        overwrite_option(root_config.execution_path, overwrite_config.execution_path);
    let executable = overwrite_option(root_config.executable, overwrite_config.executable);
    let envs = merge_option(root_config.envs, overwrite_config.envs, &merge_vars);

    ArgumentConfig {
        scope,
        executable,
        execution_path,
        envs,
    }
}

fn overwrite_state_config(base: StateConfig, replacing: StateConfig) -> StateConfig {
    StateConfig {
        state_path: replacing.state_path.or(base.state_path),
        state_root: replacing.state_root.or(base.state_root),
        address: replacing.address.or(base.address)
    }
}

fn overwrite_config(root_config: Config, overwrite_config: Config) -> Config {
    Config {
        argument: merge_option(
            root_config.argument,
            overwrite_config.argument,
            &overwrite_arguments,
        ),
        state: merge_option(root_config.state, overwrite_config.state, &overwrite_state_config)
    }
}

pub(crate) fn traverse_configs(
    start_path: &Path,
    final_path: &RelativePath,
) -> Result<Config, ConfigError> {
    debug!(
        "Traversing configs from {:?} to relative {:?}",
        start_path, final_path
    );

    let root_config = load_dploy_config(start_path)?;

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
            Err(source) => ControlFlow::Break(source),
        }
    });

    match combined_config {
        ControlFlow::Break(e) => Err(e),
        ControlFlow::Continue(config) => Ok(config),
    }
}
