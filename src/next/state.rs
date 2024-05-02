use std::{env::current_dir, path::PathBuf};

use relative_path::RelativePathBuf;

use super::{
    config::ConfigVar, errors::{StateError, StateErrorKind, WrapStateErr}, git::git_root_dir
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
}

impl StateIn {
    pub(crate) fn from_args(cwd_context: bool) -> Self {
        let mut base = Self::default();
        let ctx = if cwd_context {
            InferContext::Cwd
        } else {
            InferContext::Git
        };
        base.context = ctx;

        base
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
    fn new(ctx_infer: InferContext) -> Result<Self, StateError> {
        let current_dir =
            current_dir().to_state_err("Getting current dir for new StatePaths".to_owned())?;
        let context_root = match ctx_infer {
            InferContext::Cwd => current_dir,
            InferContext::Git => PathBuf::from(
                git_root_dir(&current_dir)
                    .to_state_err("Getting Git root dir for new StatePaths".to_owned())?,
            ),
        };
        let state_root = RelativePathBuf::new();
        let state_path = RelativePathBuf::new();

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

#[derive(Debug)]
pub(crate) struct ResolveState {
    pub(crate) state_root: PathBuf,
    pub(crate) state_path: RelativePathBuf,
    pub(crate) resolve_root: PathBuf,
    pub(crate) name: String,
    pub(crate) sub: String,
    pub(crate) hash: String,
}

pub(crate) fn create_resolve_state(state_in: StateIn) -> Result<ResolveState, StateError> {
    let paths = StatePaths::new(state_in.context)?;

    let name = paths
        .context_root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| {
            StateErrorKind::InvalidRoot(paths.context_root.to_string_lossy().to_string())
        })
        .to_state_err("Getting context name from context root path for new state.".to_owned())?;

    Ok(ResolveState {
        state_root: paths.state_root.to_path(&paths.context_root),
        state_path: paths.state_path,
        resolve_root: paths.context_root,
        name,
        sub: "tidploy_root".to_owned(),
        hash: "todo_hash".to_owned(),
    })
}
