use std::{collections::HashMap, default, env::current_dir, path::PathBuf};

use color_eyre::eyre::{ContextCompat, Report};

use keyring::Entry;
use relative_path::{RelativePath, RelativePathBuf};
use tracing::{debug, span, Level};

use crate::{config::ConfigVar, next::secrets::get_secret};

use super::errors::{SecretError, StateError};


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

#[derive(Default)]
pub(crate) struct StateIn {
    pub(crate) service: Option<String>,
}

#[derive(Debug)]
pub(crate) struct StatePaths {
    pub(crate) context_root: PathBuf,
    pub(crate) state_root: RelativePathBuf,
    pub(crate) state_path: RelativePathBuf,
    pub(crate) exe_dir: RelativePathBuf,
    pub(crate) exe_path: RelativePathBuf,
}

impl StatePaths {
    /// Creates a StatePaths struct with the context root set to the current directory. The executable
    /// is set to a default of "entrypoint.sh". 
    fn new() -> Result<Self, StateError> {
        let context_root = current_dir()?;
        let state_root = RelativePathBuf::new();
        let state_path = RelativePathBuf::new();
        let exe_dir = RelativePathBuf::new();
        let exe_path = RelativePathBuf::from("entrypoint.sh");

        Ok(StatePaths {
            context_root,
            state_path,
            state_root,
            exe_dir,
            exe_path
        })
    }
}

#[derive(Debug)]
pub(crate) struct StateOut {
    pub(crate) context_name: String,
    pub(crate) paths: StatePaths,
    pub(crate) envs: HashMap<String, String>,
    /// This defaults to 'tidploy' almost everywhere, it is mostly used for testing
    pub(crate) service: String
}

impl StateOut {
    /// Creates a new state, initializing the context root as the current directory. The context name is
    /// derived from the directory name, with non-UTF-8 characters replaced by � (U+FFFD)
    fn new(state_in: StateIn) -> Result<Self, StateError> {
        let paths = StatePaths::new()?;

        let service = state_in.service.unwrap_or("tidploy".to_owned());

        let context_name = paths.context_root.file_name().map(|s| s.to_string_lossy().to_string()).ok_or_else(|| {
            StateError::InvalidRoot(paths.context_root.to_string_lossy().to_string())
        })?;

        Ok(StateOut {
            context_name,
            paths,
            envs: HashMap::new(),
            service
        })
    }

    pub(crate) fn state_name<'a>(&'a self) -> &'a str {
        let rel_name = self.paths.state_path.as_str();
        if rel_name.len() == 0 {
            "tidploy_root"
        } else {
            rel_name
        }
    }

    pub(crate) fn state_hash(&self) -> Result<String, StateError> {
        Ok("todo_hash".to_owned())
    }
}

fn secret_vars_to_envs(state: &StateOut, vars: Vec<ConfigVar>) -> Result<HashMap<String, String>, StateError> {
    let mut envs = HashMap::<String, String>::new();
    for e in vars {
        debug!("Getting pass for {:?}", e);
        let pass = get_secret(&state.service, Some(&state.context_name), Some(state.state_name()), &state.state_hash()?, &e.key)?;

        envs.insert(e.env_name, pass);
    }
    Ok(envs)
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
    state_in: StateIn,
    exe_path: Option<&str>,
    envs: Vec<String>,
) -> Result<StateOut, StateError> {
    // Exits when the function returns
    let run_state_span = span!(Level::DEBUG, "run_state");
    let _enter = run_state_span.enter();

    let mut state = create_state(state_in)?;
    let secret_vars = parse_cli_vars(envs);
    state.envs = secret_vars_to_envs(&state, secret_vars)?;
    if let Some(exe_path) = exe_path {
        state.paths.exe_path = RelativePathBuf::from(exe_path);
    }
    debug!("Created run state is {:?}", state);
    Ok(state)
}

pub(crate) fn create_state(state_in: StateIn) -> Result<StateOut, StateError> {
    let state = StateOut::new(state_in)?;

    debug!("Created state is {:?}", state);
    Ok(state)
}
