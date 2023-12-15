
use crate::commands::{DEFAULT, DEFAULT_INFER};
use crate::config::{load_dploy_config, DployConfig, traverse_configs, ConfigError};
use crate::errors::{GitError, ProcessError, RelPathError};
use crate::filesystem::{FileError, get_current_dir};
use crate::git::{git_root_origin_url, relative_to_git_root, RepoParseError, Repo, parse_repo_url};
use crate::secret_store::{get_password, set_password};
use crate::secrets::SecretOutput;
use clap::{Parser, Subcommand, ValueEnum};
use keyring::Error as KeyringError;
use rpassword::prompt_password;
use spinoff::{spinners, Spinner};
use std::env::VarError;
use std::ffi::OsString;
use std::fs::{self};
use std::path::PathBuf;
use std::process::Output;
use std::{
    collections::HashMap,
    env,
    io::BufRead,
    io::BufReader,
    io::Error as IOError,
    path::Path,
    process::{Command as Cmd, Stdio},
};
use thiserror::Error as ThisError;
use relative_path::RelativePathBuf;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub(crate) enum StateContext {
    None,
    Git
}

impl StateContext {
    fn as_str(&self) -> &'static str {
        match self {
            StateContext::None => "none",
            StateContext::Git => "git"
        }
    }

    fn from_str(s: &str) -> Option<StateContext> {
        match s {
            "none" => Some(StateContext::None),
            "git" => Some(StateContext::Git),
            _ => None
        }
    }
}



struct State {
    network: bool,
    context: StateContext,
    repo: Repo,
    deploy_dir: RelativePathBuf,
    commit_sha: String,
    envs: HashMap<String, String>,
    exe_name: String
}

#[derive(Debug, ThisError)]
pub(crate) enum LoadError {
    #[error("Failure running Git during load! {0}")]
    Git(#[from] GitError),
    #[error("Failure creating relative path during load! {0}")]
    RelPath(#[from] RelPathError),
    #[error("Failure to read env variable {var} as unicode during load!")]
    VarNotUnicode {
        var: String
    },
    #[error("{msg}")]
    BadValue {
        msg: String,
    },
    #[error("Failure with file during load! {0}")]
    File(#[from] FileError),
    #[error("Failure loading config during load! {0}")]
    Config(#[from] ConfigError),
    #[error("Failure parsing Git url during load! {0}")]
    RepoParse(#[from] RepoParseError)
}



struct CliEnvState {
    context: Option<String>,
    no_network: Option<bool>,
    repo_url: Option<String>,
    deploy_path: Option<String>,
    tag: Option<String>,
}



fn load_state_vars() -> CliEnvState {
    let mut env_state = CliEnvState {
        context: None,
        no_network: None,
        repo_url: None,
        deploy_path: None,
        tag: None,
    };

    for (k, v) in env::vars() {
        match k.as_str() {
            "TIDPLOY_REPO" => env_state.repo_url = Some(v),
            "TIDPLOY_NETWORK" => env_state.no_network = Some(v == "0"),
            "TIDPLOY_TAG" => env_state.tag = Some(v),
            "TIDPLOY_PTH" => env_state.deploy_path = Some(v)
        }
    }

    env_state
}

fn merge_options<T: Clone>(original: Option<T>, preferred: Option<T>, most_preferred: Option<T>) -> Option<T> {
    if most_preferred.is_some() {
        return most_preferred.clone()
    }
    if preferred.is_some() {
        return preferred.clone()
    }
    original.clone()
}

fn merge_state(config: DployConfig, envs: CliEnvState, cli: CliEnvState) -> CliEnvState {
    CliEnvState {
        // Already set
        context: None,
        no_network: merge_options(config.network.map(|b| !b), envs.no_network, cli.no_network),
        repo_url: merge_options(config.repo_url, envs.repo_url, cli.repo_url),
        deploy_path: merge_options(config.deploy_path, envs.deploy_path, cli.deploy_path),
        tag: merge_options(config.tag, envs.tag, cli.tag),
    }
}



fn create_state(cli_state: CliEnvState) -> Result<State, LoadError> {
    let mut state = State {
        network: true,
        context: StateContext::Git,
        repo: Repo {
            name: DEFAULT.to_owned(),
            url: "".to_owned(),
            encoded_url: "".to_owned()
        },
        deploy_dir: RelativePathBuf::new(),
        commit_sha: DEFAULT.to_owned(),
        envs: HashMap::<String, String>::new(),
        exe_name: DEFAULT.to_owned()
    };

    let env_state = load_state_vars();

    state.context = match cli_state.context {
        None => match env::var("TIDPLOY_CONTEXT") {
            Ok(val) => StateContext::from_str(&val).ok_or(LoadError::BadValue { msg: "Environment value TIDPLOY_CONTEXT is not one of \"none\" or \"git\"!".to_owned() })?,
            Err(VarError::NotUnicode(_)) => return Err(LoadError::VarNotUnicode { var: "TIDPLOY_CONTEXT".to_owned() }),
            _ => StateContext::Git
        },
        Some(cli_context) => StateContext::from_str(&cli_context).ok_or(LoadError::BadValue { msg: "Argument for context is not one of \"none\" or \"git\"!".to_owned() })?,
    };

    //let state_env_vars = load_state_vars()?;
    let current_dir = get_current_dir().map_err(|source| FileError { source, msg: "Failed to get current dir to use for loading configs!".to_owned() })?;
    match state.context {
        StateContext::Git => {
            let git_root_relative = relative_to_git_root()?;
            let git_root_relative = RelativePathBuf::from_path(&git_root_relative).unwrap();
            let dploy_config = traverse_configs(current_dir, git_root_relative)?;
            
            let merged_state = merge_state(dploy_config, env_state, cli_state);

            let repo_url = match merged_state.repo_url {
                Some(value) if value == DEFAULT_INFER => git_root_origin_url()?,
                Some(value) => value,
                None => git_root_origin_url()?
            };

            match repo_url.as_str() {
                DEFAULT => { /* Keep as default */ },
                other => state.repo = parse_repo_url(repo_url)?
            }

            


            Ok()
        },
        StateContext::None => {
            let dploy_config = load_dploy_config(current_dir)
                .map_err(|source| ConfigError { source, msg: "Failed to load config of current dir when loading with context none!".to_owned()})?;

            let merged_state = merge_state(dploy_config, env_state, cli_state);

            Ok()
        }
    }
}