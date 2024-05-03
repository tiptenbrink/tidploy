use std::{env::current_dir, path::PathBuf};

use relative_path::RelativePathBuf;
use tracing::{debug, instrument};

use super::{
    config::{traverse_configs, ConfigAddress, ConfigVar, StateConfig}, errors::{AddressError, StateError, StateErrorKind, WrapStateErr}, git::git_root_dir
};

#[derive(Debug)]
pub(crate) enum InferContext {
    Cwd,
    Git,
}

impl Default for InferContext {
    fn default() -> Self {
        Self::Git
    }
}

#[derive(Default, Debug)]
pub(crate) struct StateIn {
    pub(crate) context: InferContext,
    pub(crate) state_path: Option<String>,
    pub(crate) state_root: Option<String>
}

impl StateIn {
    pub(crate) fn from_args(cwd_context: bool, state_path: Option<String>, state_root: Option<String>) -> Self {
        let context = if cwd_context {
            InferContext::Cwd
        } else {
            InferContext::Git
        };
        
        Self {
            context,
            state_path,
            state_root
        }
    }
}

#[derive(Debug)]
pub(crate) struct StatePaths {
    pub(crate) context_root: PathBuf,
    pub(crate) state_root: RelativePathBuf,
    pub(crate) state_path: RelativePathBuf,
}

impl StatePaths {
    /// Creates a StatePaths struct with the context root set to the current directory. The executable
    /// is set to a default of "entrypoint.sh".
    fn new(state_in: StateIn) -> Result<Self, StateError> {
        let current_dir =
            current_dir().to_state_err("Getting current dir for new StatePaths".to_owned())?;
        let context_root = match state_in.context {
            InferContext::Cwd => current_dir,
            InferContext::Git => PathBuf::from(
                git_root_dir(&current_dir)
                    .to_state_err("Getting Git root dir for new StatePaths".to_owned())?,
            ),
        };
        let state_root = state_in.state_root.map(|s| RelativePathBuf::from(s)).unwrap_or_default();
        let state_path = state_in.state_path.map(|s| RelativePathBuf::from(s)).unwrap_or_default();

        Ok(StatePaths {
            context_root,
            state_path,
            state_root,
        })
    }
}

/// Parses the list of strings given and interprets them as each pair of two being a secret key and target
/// env name.
pub(crate) fn parse_cli_vars(envs: Vec<String>) -> Vec<ConfigVar> {
    // Our chunk size is 2 so we know first and second exist
    // Any final element that does not have something to pair with will be ommitted
    envs.chunks_exact(2)
        .map(|c| ConfigVar {
            key: c.first().unwrap().to_owned(),
            env_name: c.get(1).unwrap().to_owned(),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Address {
    Local(PathBuf),
    Git(GitAddress)
}

impl From<ConfigAddress> for Address {
    fn from(value: ConfigAddress) -> Self {
        match value {
            ConfigAddress::Git { url, git_ref } => Self::Git(GitAddress {
                url,
                git_ref
            }),
            ConfigAddress::Local { path } => Self::Local(PathBuf::from(path))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GitAddress {
    pub(crate) url: String,
    pub(crate) git_ref: String 
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct State {
    pub(crate) state_root: RelativePathBuf,
    pub(crate) state_path: RelativePathBuf,
    pub(crate) context_root: PathBuf,
    pub(crate) address: Option<Address>,
}

impl From<StatePaths> for State {
    fn from(value: StatePaths) -> Self {
        State {
            state_path: value.state_path,
            state_root: value.state_root,
            context_root: value.context_root,
            address: None
        }
    }
}

impl State {
    // fn merge(self, other: Self) -> Self {
    //     Self {
    //         state_path: other.state_path,
    //         state_root: other.state_root,
    //         context_root: other.context_root,
    //         address: other.address.or(self.address)
    //     }
    // }

    fn merge_config(&self, other: StateConfig) -> Self {
        Self {
            state_path: other.state_path.map(Into::into).unwrap_or(self.state_path.clone()),
            state_root: other.state_root.map(Into::into).unwrap_or(self.state_root.clone()),
            context_root: self.context_root.clone(),
            address: other.address.map(Into::into).or(self.address.clone())
        }
    }
}

#[derive(Debug)]
pub(crate) struct ResolveState {
    pub(crate) state_root: PathBuf,
    pub(crate) state_path: RelativePathBuf,
    pub(crate) resolve_root: PathBuf,
    pub(crate) name: String,
    pub(crate) sub: String,
    pub(crate) hash: String,
}

#[instrument(name = "converge_state", level = "debug", skip_all)]
fn converge_state(state: &State) -> Result<State, StateError> {
    let mut state = state.clone();
    let mut i = 0;
    let iter = loop {
        let state_root_path = state.state_root.to_path(&state.context_root);
        let config = traverse_configs(&state_root_path, &state.state_path).to_state_err("Failed to read configs for determining new state.".to_owned())?;
        let new_state = config.state.map(|c| (&state).merge_config(c)).unwrap_or(state.clone());
        if new_state == state {
            break i+1
        } else if i > 99 {
            break 100
        }
        i += 1;
        state = new_state;
    };
    debug!("Converged to state in {} iterations.", iter);
    
    Ok(state)
}

/// Parse a repo URL to extract a "name" from it, as well as encode the part before the name to still uniquely
/// identify it. Only supports forward slashes as path seperator.
pub(crate) fn parse_url_repo_name(url: String) -> Result<String, AddressError> {
    let url = url.strip_suffix('/').unwrap_or(&url).to_owned();
    // We want the final part, after the slash, as the "file name"
    let split_parts: Vec<&str> = url.split('/').collect();

    // If last does not exist then the string is empty so invalid
    let last_part = *split_parts
        .last()
        .ok_or(AddressError::RepoParse(url.to_owned()))?;

    // In case there is a file extension (such as `.git`), we don't want that part of the name
    let split_parts_dot: Vec<&str> = last_part.split('.').collect();
    let name = if split_parts_dot.len() <= 1 {
        // In this case no "." exists and we return just the entire "file name"
        last_part.to_owned()
    } else {
        // We get only the part that comes before the first .
        (*split_parts_dot
            .first()
            .ok_or(AddressError::RepoParse(url.clone()))?)
        .to_owned()
    };

    Ok(name)
}

fn resolve_address(address: Address) -> Result<State, AddressError> {
    match address {
        Address::Git(GitAddress { url, git_ref }) => {
            let name = parse_url_repo_name(url)?;

            todo!()
        },
        Address::Local(path) => {
            todo!()
        }
    }

    Ok(todo!())
}

pub(crate) fn create_resolve_state(state_in: StateIn) -> Result<ResolveState, StateError> {
    let paths = StatePaths::new(state_in)?;

    let state = converge_state(&paths.into())?;

    let name = state
        .context_root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| {
            StateErrorKind::InvalidRoot(state.context_root.to_string_lossy().to_string())
        })
        .to_state_err("Getting context name from context root path for new state.".to_owned())?;



    Ok(ResolveState {
        state_root: state.state_root.to_path(&state.context_root),
        state_path: state.state_path,
        resolve_root: state.context_root,
        name,
        sub: "tidploy_root".to_owned(),
        hash: "todo_hash".to_owned(),
    })
}
