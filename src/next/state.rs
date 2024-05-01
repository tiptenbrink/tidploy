use std::{collections::HashMap, env::current_dir, path::PathBuf};

use color_eyre::eyre::{ContextCompat, Report};

use relative_path::{RelativePath, RelativePathBuf};
use tracing::{debug, span, Level};

use crate::{config::ConfigVar, next::secrets::get_secret};

use super::errors::SecretError;

/// Parses the list of strings given and interprets them as each pair of two being a secret key and target
/// env name.
fn parse_cli_vars(envs: Vec<String>) -> Vec<ConfigVar> {
    // Our chunk size is 2 so we know first and second exist
    // Any final element that does not have something to pair with will be ommitted
    envs.chunks_exact(2)
        .map(|c| ConfigVar {
            key: c.first().unwrap().to_owned(),
            env_name: c.get(1).unwrap().to_owned(),
        })
        .collect()
}

pub(crate) struct StatePaths {
    pub(crate) context_root: PathBuf,
    pub(crate) state_root: RelativePathBuf,
    pub(crate) state_path: RelativePathBuf,
    pub(crate) exe_dir: RelativePathBuf,
    pub(crate) exe_path: RelativePathBuf,
}

pub(crate) struct StateOut {
    pub(crate) context_name: String,
    pub(crate) paths: StatePaths,
    pub(crate) envs: HashMap<String, String>,
}

impl StateOut {
    fn state_name<'a>(&'a self) -> &'a str {
        self.paths.state_path.as_str()
    }
}

fn secret_vars_to_envs(state: &StateOut, vars: Vec<ConfigVar>) -> Result<HashMap<String, String>, SecretError> {
    let mut envs = HashMap::<String, String>::new();
    for e in vars {
        debug!("NOT YET IMPLEMENTED Getting pass for {:?}", e);
        let pass = get_secret(Some(&state.context_name), Some(state.state_name()), "todo_hash", &e.key)?;

        envs.insert(e.env_name, pass);
    }
    Ok(envs)
}

fn state_paths(exe_path: Option<&str>) -> StatePaths {
    let context_root = current_dir().unwrap();
    let state_root = RelativePathBuf::new();
    let state_path = RelativePathBuf::new();
    let exe_dir = RelativePathBuf::new();
    let exe_path = RelativePathBuf::from(exe_path.unwrap_or("entrypoint.sh"));

    StatePaths {
        context_root,
        state_path,
        state_root,
        exe_dir,
        exe_path
    }
}

pub(crate) struct StatePathsResolved {
    pub(crate) context_root: PathBuf,
    pub(crate) state_root: PathBuf,
    pub(crate) state_path: RelativePathBuf,
    pub(crate) exe_dir: PathBuf,
    pub(crate) exe_path: RelativePathBuf,
}

pub(crate) fn resolve_paths(state_paths: StatePaths) -> StatePathsResolved {
    StatePathsResolved {
        state_root: state_paths.state_root.to_path(&state_paths.context_root),
        state_path: state_paths.state_path,
        exe_dir: state_paths.exe_dir.to_path(&state_paths.context_root),
        exe_path: state_paths.exe_path,
        context_root: state_paths.context_root
    }
}

/// Creates the state that is used to run the executable.
pub(crate) fn create_state_run(
    exe_path: Option<&str>,
    envs: Vec<String>,
) -> Result<StateOut, Report> {
    // Exits when the function returns
    let run_state_span = span!(Level::DEBUG, "run_state");
    let _enter = run_state_span.enter();

    let paths = state_paths(exe_path);
    let secret_vars = parse_cli_vars(envs);
    let mut state = StateOut {
        context_name: paths.context_root.to_str().wrap_err(format!("Path {:?} is not valid Unicode!", paths.context_root))?.to_owned(),
        paths,
        envs: HashMap::new(),
    };

    state.envs = secret_vars_to_envs(&state, secret_vars).unwrap();
    Ok(state)
}
