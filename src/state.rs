use crate::auth::{auth_get_password, AuthError};
use crate::commands::{DEFAULT, DEFAULT_INFER, TIDPLOY_DEFAULT};
use crate::config::{
    load_dploy_config, merge_vars, traverse_configs, ConfigError, ConfigVar, DployConfig,
};
use crate::errors::{GitError, RelPathError};
use crate::filesystem::{get_current_dir, FileError};
use crate::git::{
    git_root_origin_url, parse_repo_url, relative_to_git_root, rev_parse_tag, Repo, RepoParseError,
};

use clap::ValueEnum;

use relative_path::RelativePathBuf;

use std::env::VarError;

use std::path::{Path, PathBuf};
use std::{collections::HashMap, env};
use thiserror::Error as ThisError;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub(crate) enum StateContext {
    None,
    Git,
}

impl StateContext {
    // fn as_str(&self) -> &'static str {
    //     match self {
    //         StateContext::None => "none",
    //         StateContext::Git => "git",
    //     }
    // }

    fn from_str(s: &str) -> Option<StateContext> {
        match s {
            "none" => Some(StateContext::None),
            "git" => Some(StateContext::Git),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct State {
    pub(crate) network: bool,
    pub(crate) context: StateContext,
    pub(crate) repo: Repo,
    pub(crate) deploy_path: RelativePathBuf,
    pub(crate) tag: String,
    pub(crate) commit_sha: String,
    pub(crate) envs: HashMap<String, String>,
    pub(crate) exe_name: String,
    pub(crate) current_dir: PathBuf,
}

#[derive(Debug, ThisError)]
pub(crate) enum LoadError {
    #[error("Failure running Git during load! {0}")]
    Git(#[from] GitError),
    #[error("Failure creating relative path during load! {0}")]
    RelPath(#[from] RelPathError),
    #[error("Failure to read env variable {var} as unicode during load!")]
    VarNotUnicode { var: String },
    #[error("{msg}")]
    BadValue { msg: String },
    #[error("Failure with file during load! {0}")]
    File(#[from] FileError),
    #[error("Failure loading config during load! {0}")]
    Config(#[from] ConfigError),
    #[error("Failure parsing Git url during load! {0}")]
    RepoParse(#[from] RepoParseError),
    #[error("Failure getting value of env! {0}")]
    Auth(#[from] AuthError),
}

struct CliEnvRunState {
    envs: Vec<ConfigVar>,
    exe_name: Option<String>,
}

#[derive(Clone)]
pub(crate) struct CliEnvState {
    pub(crate) context: Option<StateContext>,
    pub(crate) network: Option<bool>,
    pub(crate) repo_url: Option<String>,
    pub(crate) deploy_path: Option<String>,
    pub(crate) tag: Option<String>,
}

fn load_state_run_vars() -> CliEnvRunState {
    let mut envs_vec = Vec::new();

    let mut exe_name = None;

    for (k, v) in env::vars() {
        if k == "TIDPLOY_EXE" {
            exe_name = Some(v)
        } else if k.starts_with("TIDPLOY_VAR_") {
            let env_name = k.strip_prefix("TIDPLOY_VAR_").unwrap().to_owned();
            envs_vec.push(ConfigVar { env_name, key: v })
        }
    }

    CliEnvRunState {
        envs: envs_vec,
        exe_name,
    }
}

fn load_state_vars() -> CliEnvState {
    let mut env_state = CliEnvState {
        context: None,
        network: None,
        repo_url: None,
        deploy_path: None,
        tag: None,
    };

    for (k, v) in env::vars() {
        match k.as_str() {
            "TIDPLOY_REPO" => env_state.repo_url = Some(v),
            "TIDPLOY_NETWORK" => env_state.network = Some(v != "0" && v.to_lowercase() != "false"),
            "TIDPLOY_TAG" => env_state.tag = Some(v),
            "TIDPLOY_PTH" => env_state.deploy_path = Some(v),
            _ => {}
        }
    }

    env_state
}

fn merge_options<T: Clone>(
    original: Option<T>,
    preferred: Option<T>,
    most_preferred: Option<T>,
) -> Option<T> {
    if most_preferred.is_some() {
        return most_preferred;
    }
    if preferred.is_some() {
        return preferred;
    }
    original
}

fn merge_state(config: &DployConfig, envs: CliEnvState, cli: CliEnvState) -> CliEnvState {
    CliEnvState {
        // Already set
        context: None,
        network: merge_options(config.network, envs.network, cli.network),
        repo_url: merge_options(config.repo_url.clone(), envs.repo_url, cli.repo_url),
        deploy_path: merge_options(
            config.deploy_path.clone(),
            envs.deploy_path,
            cli.deploy_path,
        ),
        tag: merge_options(config.tag.clone(), envs.tag, cli.tag),
    }
}

fn merge_run_state(
    config: &DployConfig,
    envs: CliEnvRunState,
    cli: CliEnvRunState,
) -> CliEnvRunState {
    let envs_overwrite_config = merge_vars(config.vars.clone(), Some(envs.envs));
    let cli_overwrite_envs = merge_vars(envs_overwrite_config, Some(cli.envs)).unwrap();

    CliEnvRunState {
        exe_name: merge_options(config.exe_name.clone(), envs.exe_name, cli.exe_name),
        envs: cli_overwrite_envs,
    }
}

fn set_state(
    state: &mut State,
    merged_state: CliEnvState,
    merged_run_state: Option<CliEnvRunState>,
    load_tag: bool,
) -> Result<(), LoadError> {
    let repo_url = match state.context {
        StateContext::None => match merged_state.repo_url {
            Some(value) if value == DEFAULT_INFER => git_root_origin_url(&state.current_dir)?, // Only infer if explicitly set to infer
            Some(value) => value,
            None => DEFAULT.to_owned(), // Unset here defaults to just leaving it as 'default'
        },
        StateContext::Git => match merged_state.repo_url {
            Some(value) if value == DEFAULT_INFER => git_root_origin_url(&state.current_dir)?,
            Some(value) => value,
            None => git_root_origin_url(&state.current_dir)?,
        },
    };

    match repo_url.as_str() {
        DEFAULT => { /* Keep as default */ }
        _other => state.repo = parse_repo_url(repo_url)?,
    }

    if let Some(value) = merged_state.network {
        state.network = value
    };

    let tag = match merged_state.tag {
        Some(value) => value,
        None => TIDPLOY_DEFAULT.to_owned(),
    };

    if let Some(value) = merged_state.deploy_path {
        state.deploy_path = RelativePathBuf::from_path(&value).map_err(|e| {
            let msg = format!("Failed to get relative path for deploy path: {}!", value);
            RelPathError::from_knd(e, msg)
        })?
    };

    // TODO maybe infer the tag from the current folder or checked out tag

    // We only want to load the tag when we've actually downloaded the target repository

    if load_tag && tag != TIDPLOY_DEFAULT {
        state.commit_sha = rev_parse_tag(&tag, &state.current_dir)?;
    } else if load_tag {
        state.commit_sha = rev_parse_tag("HEAD", &state.current_dir)?;
    } else {
        state.commit_sha = tag.clone();
    }

    if tag != TIDPLOY_DEFAULT {
        state.tag = tag;
    }

    if let Some(merged_run_state) = merged_run_state {
        for e in merged_run_state.envs {
            let pass = auth_get_password(state, &e.key).map_err(|source| {
                let msg = format!("Failed to get password with key {} from passwords while loading envs into state!", e.key);
                AuthError { msg, source }
            })?;

            state.envs.insert(e.env_name, pass);
        }

        if let Some(exe_name) = merged_run_state.exe_name {
            state.exe_name = exe_name
        }

        if state.exe_name == TIDPLOY_DEFAULT {
            state.exe_name = "entrypoint.sh".to_owned();
        }
    }

    Ok(())
}

pub(crate) fn create_state_create(
    cli_state: CliEnvState,
    path: Option<&Path>,
    load_tag: bool,
) -> Result<State, LoadError> {
    create_state(cli_state, None, path, load_tag)
}

fn parse_cli_envs(envs: Vec<String>) -> Vec<ConfigVar> {
    envs.chunks_exact(2)
        .map(|c| ConfigVar {
            key: c.get(0).unwrap().to_owned(),
            env_name: c.get(1).unwrap().to_owned(),
        })
        .collect()
}

pub(crate) fn create_state_run(
    cli_state: CliEnvState,
    exe_name: Option<String>,
    envs: Vec<String>,
    path: Option<&Path>,
    load_tag: bool,
) -> Result<State, LoadError> {
    let cli_run_state = CliEnvRunState {
        exe_name,
        envs: parse_cli_envs(envs),
    };
    create_state(cli_state, Some(cli_run_state), path, load_tag)
}

fn create_state(
    cli_state: CliEnvState,
    cli_run_state: Option<CliEnvRunState>,
    path: Option<&Path>,
    load_tag: bool,
) -> Result<State, LoadError> {
    let current_dir = if let Some(path) = path {
        path.to_owned()
    } else {
        get_current_dir().map_err(|source| FileError {
            source,
            msg: "Failed to get current dir to use for loading configs!".to_owned(),
        })?
    };

    let mut state = State {
        network: true,
        context: StateContext::Git,
        repo: Repo {
            name: DEFAULT.to_owned(),
            url: "".to_owned(),
            encoded_url: "".to_owned(),
        },
        tag: "latest".to_owned(),
        deploy_path: RelativePathBuf::new(),
        commit_sha: TIDPLOY_DEFAULT.to_owned(),
        envs: HashMap::<String, String>::new(),
        exe_name: TIDPLOY_DEFAULT.to_owned(),
        current_dir,
    };

    let env_state = load_state_vars();
    let env_run_state = if cli_run_state.is_some() {
        Some(load_state_run_vars())
    } else {
        None
    };

    state.context = match cli_state.context {
        None => match env::var("TIDPLOY_CONTEXT") {
            Ok(val) => StateContext::from_str(&val).ok_or(LoadError::BadValue {
                msg: "Environment value TIDPLOY_CONTEXT is not one of \"none\" or \"git\"!"
                    .to_owned(),
            })?,
            Err(VarError::NotUnicode(_)) => {
                return Err(LoadError::VarNotUnicode {
                    var: "TIDPLOY_CONTEXT".to_owned(),
                })
            }
            _ => StateContext::Git,
        },
        Some(cli_context) => cli_context,
    };

    //let state_env_vars = load_state_vars()?;
    let dploy_config = match state.context {
        StateContext::Git => {
            let git_root_relative = relative_to_git_root()?;
            let git_root_relative = RelativePathBuf::from_path(git_root_relative).unwrap();
            traverse_configs(state.current_dir.clone(), git_root_relative)?
        }
        StateContext::None => {
            load_dploy_config(state.current_dir.clone()).map_err(|source| ConfigError {
                source,
                msg: "Failed to load config of current dir when loading with context none!"
                    .to_owned(),
            })?
        }
    };

    let merged_state = merge_state(&dploy_config, env_state, cli_state);

    if let Some(cli_run_state) = cli_run_state {
        let merged_run_state =
            merge_run_state(&dploy_config, cli_run_state, env_run_state.unwrap());
        set_state(&mut state, merged_state, Some(merged_run_state), load_tag)?;
    } else {
        set_state(&mut state, merged_state, None, load_tag)?;
    }

    Ok(state)
}
