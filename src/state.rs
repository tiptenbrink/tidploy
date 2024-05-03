use crate::commands::{DEFAULT_GIT_LOCAL, DEFAULT_GIT_REMOTE, TIDPLOY_DEFAULT};
use crate::config::{merge_vars, traverse_configs, ConfigError, ConfigVar, DployConfig};
use crate::errors::{GitError, RelPathError, RepoParseError};
use crate::filesystem::{get_current_dir, FileError, WrapToPath};
use crate::git::{git_root_dir, git_root_origin_url, parse_repo_url, rev_parse_tag, Repo};
use crate::secret::{get_secret, AuthError};

use camino::{Utf8Path, Utf8PathBuf};
use clap::ValueEnum;

use relative_path::{RelativePath, RelativePathBuf};

use std::env::VarError;
use std::{collections::HashMap, env};
use thiserror::Error as ThisError;

use tracing::{debug, span, Level};

/// The different contexts that tidploy will use to populate its configuration. 'None' means it will
/// not consider that it is currently in a Git project and will only pick up configuration in its
/// current directory.
#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum StateContext {
    None,
    GitRemote,
    GitLocal,
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
            "git_local" => Some(StateContext::GitLocal),
            "git_remote" => Some(StateContext::GitRemote),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct State {
    pub(crate) context: StateContext,
    pub(crate) repo: Repo,
    pub(crate) deploy_path: RelativePathBuf,
    pub(crate) tag: String,
    pub(crate) commit_sha: String,
    pub(crate) envs: HashMap<String, String>,
    pub(crate) exe_name: String,
    pub(crate) root_dir: Utf8PathBuf,
}

impl State {
    pub(crate) fn deploy_dir(&self) -> Utf8PathBuf {
        let dir = self.deploy_path.to_utf8_path(&self.root_dir);
        debug!("Computed deploy_dir as {:?}", dir);
        dir
    }
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

#[derive(Clone, Debug)]
pub(crate) struct CliEnvRunState {
    pub(crate) envs: Vec<ConfigVar>,
    pub(crate) exe_name: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct CliEnvState {
    pub(crate) context: Option<StateContext>,
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

    debug!(
        "Loaded EnvRunState from env vars. exe_name: {:?}. envs: {:?}",
        exe_name, envs_vec
    );

    CliEnvRunState {
        envs: envs_vec,
        exe_name,
    }
}

/// Load all environment variables that for CliEnvState, except context which is loaded separately.
fn load_state_vars() -> CliEnvState {
    let mut env_state = CliEnvState {
        context: None,
        repo_url: None,
        deploy_path: None,
        tag: None,
    };

    for (k, v) in env::vars() {
        match k.as_str() {
            "TIDPLOY_REPO" => env_state.repo_url = Some(v),
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

    let merged_run_state = CliEnvRunState {
        exe_name: merge_options(config.exe_name.clone(), envs.exe_name, cli.exe_name),
        envs: cli_overwrite_envs,
    };
    debug!("Merged run state: {:?}", merged_run_state);
    merged_run_state
}

#[derive(Debug)]
enum ReadRepoMethod {
    Value(String),
    GitRoot,
    GitRootRemote,
    Default,
}

fn set_state(
    state: &mut State,
    merged_state: CliEnvState,
    merged_run_state: Option<CliEnvRunState>,
    load_tag: bool,
) -> Result<(), LoadError> {
    let read_repo_url_method = match merged_state.repo_url {
        Some(value) if value == DEFAULT_GIT_REMOTE => ReadRepoMethod::GitRootRemote,
        Some(value) if value == DEFAULT_GIT_LOCAL => ReadRepoMethod::GitRoot,
        Some(value) => ReadRepoMethod::Value(value),
        None => match state.context {
            StateContext::None => ReadRepoMethod::Default,
            StateContext::GitRemote => ReadRepoMethod::GitRootRemote,
            StateContext::GitLocal => ReadRepoMethod::GitRoot,
        },
    };
    debug!(
        "repo_url will be read using method: {:?}",
        read_repo_url_method
    );

    let repo_url = match read_repo_url_method {
        ReadRepoMethod::Value(value) => value,
        ReadRepoMethod::Default => TIDPLOY_DEFAULT.to_owned(),
        ReadRepoMethod::GitRootRemote => git_root_origin_url(&state.root_dir)?,
        ReadRepoMethod::GitRoot => state.root_dir.as_str().to_owned(),
    };

    match repo_url.as_str() {
        TIDPLOY_DEFAULT => {
            debug!("Keeping state repo as default.")
        }
        _other => {
            let parsed_repo_url = parse_repo_url(repo_url)?;
            debug!(
                "Setting state repo to parsed repo url {:?}",
                parsed_repo_url
            );
            state.repo = parsed_repo_url
        }
    }

    let tag = match merged_state.tag {
        Some(value) => value,
        None => TIDPLOY_DEFAULT.to_owned(),
    };
    debug!("Tag set to {}.", tag);

    if let Some(value) = merged_state.deploy_path {
        let deploy_path = RelativePathBuf::from_path(&value).map_err(|e| {
            let msg = format!("Failed to get relative path for deploy path: {}!", value);
            RelPathError::from_knd(e, msg)
        })?;
        debug!("Deploy path set to {:?}.", deploy_path);
        state.deploy_path = deploy_path
    };

    // TODO maybe infer the tag from the current folder or checked out tag

    // We only want to load the tag when we've actually downloaded the target repository

    if load_tag && tag != TIDPLOY_DEFAULT {
        debug!("Setting commit sha to commit associated with tag {}.", tag);
        state.commit_sha = rev_parse_tag(&tag, &state.root_dir)?;
    } else if load_tag {
        debug!("Setting commit sha to HEAD commit.");
        state.commit_sha = rev_parse_tag("HEAD", &state.root_dir)?;
    } else {
        debug!("Setting commit sha to tag.");
        state.commit_sha = tag.clone();
    }

    if tag != TIDPLOY_DEFAULT {
        state.tag = tag;
    }

    if let Some(merged_run_state) = merged_run_state {
        for e in merged_run_state.envs {
            debug!("Getting pass for {:?}", e);
            let pass = get_secret(state, &e.key).map_err(|source| {
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

    debug!("Final state is: {:?}", state);

    Ok(())
}

pub(crate) fn create_state_create(
    cli_state: CliEnvState,
    project_path: Option<&Utf8Path>,
    deploy_path: Option<&RelativePath>,
    load_tag: bool,
) -> Result<State, LoadError> {
    create_state(cli_state, None, project_path, deploy_path, load_tag)
}

fn parse_cli_envs(envs: Vec<String>) -> Vec<ConfigVar> {
    envs.chunks_exact(2)
        .map(|c| ConfigVar {
            key: c.first().unwrap().to_owned(),
            env_name: c.get(1).unwrap().to_owned(),
        })
        .collect()
}

/// Creates the state that is used to run the executable. Adds envs provided through CLI to `create_state`.
pub(crate) fn create_state_run(
    cli_state: CliEnvState,
    exe_name: Option<String>,
    envs: Vec<String>,
    path: Option<&Utf8Path>,
    deploy_path: Option<&RelativePath>,
    load_tag: bool,
) -> Result<State, LoadError> {
    // Exits when the function returns
    let run_state_span = span!(Level::DEBUG, "run_state");
    let _enter = run_state_span.enter();

    let cli_run_state = CliEnvRunState {
        exe_name,
        envs: parse_cli_envs(envs),
    };
    debug!("Parsed CLI envs as {:?}", cli_run_state);
    create_state(cli_state, Some(cli_run_state), path, deploy_path, load_tag)
}

/// Create a new state, merging the cli_state, env var state and config state and potentially loading it from the
/// context of the supplied path (or current directory if not provided). If cli_run_state is None, no run_state is
/// loaded.
pub(crate) fn create_state(
    cli_state: CliEnvState,
    cli_run_state: Option<CliEnvRunState>,
    project_path: Option<&Utf8Path>,
    deploy_path: Option<&RelativePath>,
    load_tag: bool,
) -> Result<State, LoadError> {
    let current_dir = if let Some(path) = project_path {
        path.to_owned()
    } else {
        debug!("Using current dir as path for creating state.");
        get_current_dir().map_err(|source| FileError {
            source,
            msg: "Failed to get current dir!".to_owned(),
        })?
    };
    debug!("Creating state with path {:?}", current_dir);

    // ######################
    // INITIAL STATE CREATION
    // ######################

    // By default it sets context to git remote, repo name to default with an empty url; tag to latest.
    // deploy path to root of the repository and _tidploy_default for commit and exe name.
    // current_dir is either the provided path or the directory that the command is called from
    let mut state = State {
        context: StateContext::GitRemote,
        repo: Repo {
            name: TIDPLOY_DEFAULT.to_owned(),
            url: "".to_owned(),
            encoded_url: "".to_owned(),
        },
        tag: "latest".to_owned(),
        deploy_path: deploy_path.map(RelativePath::to_owned).unwrap_or_default(),
        commit_sha: TIDPLOY_DEFAULT.to_owned(),
        envs: HashMap::<String, String>::new(),
        exe_name: TIDPLOY_DEFAULT.to_owned(),
        root_dir: Utf8PathBuf::new(), // always replaced
    };
    debug!("Starting state is {:?}", state);

    // Load environment variable state
    let env_state = load_state_vars();
    debug!("Loaded env_state from env vars: {:?}", env_state);
    // In case cli_run_state is None, this `create_state` does not need to determine any run state
    let env_run_state = if cli_run_state.is_some() {
        // Load environment variable run_state
        Some(load_state_run_vars())
    } else {
        None
    };

    // We load this environment variable value manually so we can immediately determine the context
    state.context = match cli_state.context {
        None => match env::var("TIDPLOY_CONTEXT") {
            Ok(val) => StateContext::from_str(&val).ok_or(LoadError::BadValue {
                msg: "Environment value TIDPLOY_CONTEXT is not one of \"none\" or \"git_local\" or \"git_remote\"!"
                    .to_owned(),
            })?,
            Err(VarError::NotUnicode(_)) => {
                return Err(LoadError::VarNotUnicode {
                    var: "TIDPLOY_CONTEXT".to_owned(),
                })
            }
            _ => StateContext::GitRemote,
        },
        Some(cli_context) => cli_context,
    };

    // When context is none we don't want to do any looking around, otherwise we use the root of the current dir/provided path
    state.root_dir = match state.context {
        StateContext::None => current_dir,
        StateContext::GitLocal | StateContext::GitRemote => {
            Utf8Path::new(&git_root_dir(&current_dir)?).to_owned()
        }
    };

    debug!("Loaded state context as {:?}", state.context);

    let dploy_config = traverse_configs(&state.root_dir, &state.deploy_path)?;

    let merged_state = merge_state(&dploy_config, env_state, cli_state);
    debug!(
        "Merged CliEnv state from config, env and CLI: {:?}",
        merged_state
    );

    if let Some(cli_run_state) = cli_run_state {
        let merged_run_state =
            merge_run_state(&dploy_config, env_run_state.unwrap(), cli_run_state);
        set_state(&mut state, merged_state, Some(merged_run_state), load_tag)?;
    } else {
        set_state(&mut state, merged_state, None, load_tag)?;
    }

    Ok(state)
}

/// Adds a number of useful environment variables, such as the commit sha (both full and the first 7 characters) as well as the tag.
pub(crate) fn extra_envs(mut state: State) -> State {
    let commit_long = state.commit_sha.clone();
    let commit_short = state.commit_sha[0..7].to_owned();

    debug!(
        "Setting state extra envs: sha: {}, sha_long: {}, tag: {}",
        commit_short, commit_long, state.tag
    );

    state.envs.insert("TIDPLOY_SHA".to_owned(), commit_short);
    state
        .envs
        .insert("TIDPLOY_SHA_LONG".to_owned(), commit_long);
    state
        .envs
        .insert("TIDPLOY_TAG".to_owned(), state.tag.clone());

    state
}
