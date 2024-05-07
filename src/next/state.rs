use std::env::current_dir;

use camino::{Utf8Path, Utf8PathBuf};
use relative_path::RelativePathBuf;
use tracing::{debug, instrument};

use crate::{filesystem::WrapToPath, next::git::git_root_origin_url};

use super::{
    config::{traverse_configs, ConfigAddress, ConfigVar, StateConfig},
    errors::{AddressError, StateError, StateErrorKind, WrapStateErr},
    fs::get_dirs,
    git::get_dir_from_git,
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
#[derive(Debug, Clone, Default)]
pub struct LocalAddressIn {
    pub resolve_root: Option<String>,
    pub state_path: Option<String>,
    pub state_root: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct GitAddressIn {
    pub url: Option<String>,
    pub git_ref: Option<String>,
    pub target_resolve_root: Option<String>,
    pub state_path: Option<String>,
    pub state_root: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AddressIn {
    Local(LocalAddressIn),
    Git(GitAddressIn),
}

impl AddressIn {
    pub(crate) fn from_run(
        resolve_root: Option<String>,
        state_path: Option<String>,
        state_root: Option<String>,
    ) -> Self {
        AddressIn::Local(LocalAddressIn {
            resolve_root,
            state_path,
            state_root,
        })
    }

    pub(crate) fn from_secret(
        resolve_root: Option<String>,
        state_path: Option<String>,
        state_root: Option<String>,
    ) -> Self {
        AddressIn::Local(LocalAddressIn {
            resolve_root,
            state_path,
            state_root,
        })
    }

    pub(crate) fn from_deploy(
        url: Option<String>,
        git_ref: Option<String>,
        target_resolve_root: Option<String>,
        state_path: Option<String>,
        state_root: Option<String>,
    ) -> Self {
        AddressIn::Git(GitAddressIn {
            url,
            git_ref,
            target_resolve_root,
            state_path,
            state_root,
        })
    }
}

// #[derive(Default, Debug)]
// pub(crate) struct StateIn {
//     pub(crate) context: InferContext,
//     pub(crate) resolve_root: Option<String>,
//     pub(crate) state_path: Option<String>,
//     pub(crate) state_root: Option<String>,
// }

// impl StateIn {
//     pub(crate) fn from_args(
//         cwd_context: bool,
//         resolve_root: Option<String>,
//         state_path: Option<String>,
//         state_root: Option<String>,
//     ) -> Self {
//         let context = if cwd_context {
//             InferContext::Cwd
//         } else {
//             InferContext::Git
//         };

//         Self {
//             context,
//             resolve_root,
//             state_path,
//             state_root,
//         }
//     }
// }

// #[derive(Debug)]
// pub(crate) struct StatePaths {
//     pub(crate) resolve_root: Utf8PathBuf,
//     pub(crate) state_root: RelativePathBuf,
//     pub(crate) state_path: RelativePathBuf,
// }

// /// This gets the initial resolve root to bootstrap the entire process. There are two contexts. The first
// /// is the "Cwd" or current working directory context. The second is the Git context.
// /// Cwd: if you give a relative path, it will be relative to the current path; absolute path it will just be that path
// /// Git: if you give a relative path, it will be relative to the Git dir; absolute path it will ERROR
// fn get_initial_resolve_root(resolve_root_option: Option<String>, context: InferContext) -> Result<Utf8PathBuf, StateError> {
//     let current_dir = get_current_dir()?;
//     let resolve_root_path = resolve_root_option.map(Utf8PathBuf::from);
//     let resolve_root = resolve_root_path.unwrap_or_default();
//     let resolve_root_rel = RelativePathBuf::from_path(&resolve_root).ok();

//     let resolve_root = match context {
//         InferContext::Cwd => match resolve_root_rel {
//             Some(resolve_root_rel) => resolve_root_rel.to_utf8_path(&current_dir),
//             None => resolve_root,
//         },
//         InferContext::Git => {
//             let git_dir = Utf8PathBuf::from(
//                 git_root_dir(&current_dir)
//                     .to_state_err("Getting Git root dir for new StatePaths".to_owned())?,
//             );
//             match resolve_root_rel {
//                 Some(resolve_root_rel) => resolve_root_rel.to_utf8_path(&git_dir),
//                 None => return Err(StateError {
//                     msg: "Cannot set initial resolve root to absolute path when using Git context".to_owned(),
//                     source: StateErrorKind::NoAbsoluteGit.into()
//                 }),
//             }
//         }
//     };

//     Ok(resolve_root)
// }

// impl StatePaths {
//     /// Creates a StatePaths struct with the context root set to the current directory. The executable
//     /// is set to a default of "entrypoint.sh".
//     pub(crate) fn new(state_in: StateIn) -> Result<Self, StateError> {
//         let current_dir =
//             current_dir().to_state_err("Getting current dir for new StatePaths".to_owned())?;
//         let current_dir = Utf8PathBuf::from_path_buf(current_dir).map_err(|_e| StateError {
//             msg: "Current directory is not valid UTF-8!".to_owned(),
//             source: StateErrorKind::InvalidPath.into(),
//         })?;
//         let resolve_root_path = state_in.resolve_root.map(Utf8PathBuf::from);
//         let resolve_root = resolve_root_path.unwrap_or_default();
//         let resolve_root_rel = RelativePathBuf::from_path(&resolve_root).ok();

//         let resolve_root = match state_in.context {
//             InferContext::Cwd => match resolve_root_rel {
//                 Some(resolve_root_rel) => resolve_root_rel.to_utf8_path(&current_dir),
//                 None => resolve_root,
//             },
//             InferContext::Git => {
//                 let git_dir = Utf8PathBuf::from(
//                     git_root_dir(&current_dir)
//                         .to_state_err("Getting Git root dir for new StatePaths".to_owned())?,
//                 );
//                 match resolve_root_rel {
//                     Some(resolve_root_rel) => resolve_root_rel.to_utf8_path(&git_dir),
//                     None => git_dir,
//                 }
//             }
//         };

//         // let resolve_root_path = state_in
//         //     .resolve_root
//         //     .map(|s| Utf8PathBuf::from(s))
//         //     .unwrap_or_default();
//         // resolve_root_rel.is_relative()
//         // let resolve_root_rel = state_in
//         //     .resolve_root
//         //     .map(|s| RelativePathBuf::from(s))
//         //     .unwrap_or_default();
//         // let resolve_root = (&resolve_root_rel).to_path_canon_checked(&context_root)
//         //     .to_state_err(format!("Error interpreting resolve_root {} as relative to the context_root {}", &resolve_root_rel, &context_root))?;
//         let state_root = state_in
//             .state_root
//             .map(RelativePathBuf::from)
//             .unwrap_or_default();
//         let state_path = state_in
//             .state_path
//             .map(RelativePathBuf::from)
//             .unwrap_or_default();

//         Ok(StatePaths {
//             resolve_root,
//             state_path,
//             state_root,
//         })
//     }
// }

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

#[derive(Debug, Clone)]
pub(crate) struct Address {
    pub(crate) root: AddressRoot,
    pub(crate) state_root: RelativePathBuf,
    pub(crate) state_path: RelativePathBuf,
}

#[derive(Debug, Clone)]
pub(crate) enum AddressRoot {
    /// An address is: either absolute or relative to the previous resolve_root
    Local(Utf8PathBuf),
    Git(GitAddress),
}

fn get_current_dir() -> Result<Utf8PathBuf, StateError> {
    Utf8PathBuf::from_path_buf(
        current_dir().to_state_err("Error getting current directory!".to_owned())?,
    )
    .map_err(|_e| StateError {
        msg: "Current directory is not UTF-8!".to_owned(),
        source: StateErrorKind::InvalidPath.into(),
    })
}

impl Address {
    fn from_config_addr(value: ConfigAddress, resolve_root: &Utf8Path) -> Self {
        debug!("Converting config_adress {:?} to address!", value);

        match value {
            ConfigAddress::Git {
                url,
                git_ref,
                target_path,
                state_path,
                state_root,
            } => Address {
                root: AddressRoot::Git(GitAddress {
                    url,
                    git_ref,
                    path: RelativePathBuf::from(target_path.unwrap_or_default()),
                }),
                state_path: RelativePathBuf::from(state_path.unwrap_or_default()),
                state_root: RelativePathBuf::from(state_root.unwrap_or_default()),
            },
            ConfigAddress::Local {
                path,
                state_path,
                state_root,
            } => {
                let address_root = Utf8PathBuf::from(path.clone());
                let address_rel = RelativePathBuf::from_path(&address_root).ok();
                let root = if let Some(address_rel) = address_rel {
                    address_rel.to_utf8_path(resolve_root)
                } else {
                    address_root
                };

                Address {
                    root: AddressRoot::Local(root),
                    state_path: RelativePathBuf::from(state_path.unwrap_or_default()),
                    state_root: RelativePathBuf::from(state_root.unwrap_or_default()),
                }
            }
        }
    }

    fn from_addr_in(value: AddressIn, infer_ctx: InferContext) -> Result<Self, StateError> {
        debug!("Converting config_adress {:?} to address!", value);

        let addr = match value {
            AddressIn::Git(GitAddressIn {
                url,
                git_ref,
                state_path,
                state_root,
                target_resolve_root,
            }) => {
                let url = if let Some(url) = url {
                    url
                } else {
                    let current_dir = get_current_dir()?;

                    match infer_ctx {
                        InferContext::Git => git_root_origin_url(&current_dir)
                            .to_state_err("Error resolving current Git repository.".to_owned())?,
                        InferContext::Cwd => current_dir.to_string(),
                    }

                    // If you want to start with a Git repo but use a different resolve root and not give the URL
                    // Just start in a different directory!
                };
                Address {
                    root: AddressRoot::Git(GitAddress {
                        url,
                        git_ref: git_ref.unwrap_or_else(|| "HEAD".to_owned()),
                        path: RelativePathBuf::from(target_resolve_root.unwrap_or_default()),
                    }),
                    state_path: RelativePathBuf::from(state_path.unwrap_or_default()),
                    state_root: RelativePathBuf::from(state_root.unwrap_or_default()),
                }
            }
            AddressIn::Local(LocalAddressIn {
                resolve_root,
                state_path,
                state_root,
            }) => {
                let resolve_root = resolve_root.map(Utf8PathBuf::from).unwrap_or_default();
                let resolve_root_rel = RelativePathBuf::from_path(&resolve_root).ok();
                // If you give an absolute path, that will be used
                // If you give a relative path, then it will be relative to current dir when infer is cwd, otherwise relative to git root
                let resolve_root = match resolve_root_rel {
                    Some(resolve_root_rel) => {
                        let current_dir = get_current_dir()?;
                        let base_dir = match infer_ctx {
                            InferContext::Cwd => current_dir,
                            InferContext::Git => {
                                Utf8PathBuf::from(git_root_origin_url(&current_dir).to_state_err(
                                    "Error resolving current Git repository.".to_owned(),
                                )?)
                            }
                        };

                        resolve_root_rel.to_utf8_path(base_dir)
                    }
                    None => resolve_root,
                };

                Address {
                    root: AddressRoot::Local(resolve_root),
                    state_path: RelativePathBuf::from(state_path.unwrap_or_default()),
                    state_root: RelativePathBuf::from(state_root.unwrap_or_default()),
                }
            }
        };

        debug!("Got address {:?}", addr);

        Ok(addr)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GitAddress {
    pub(crate) url: String,
    pub(crate) git_ref: String,
    pub(crate) path: RelativePathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct State {
    pub(crate) state_root: RelativePathBuf,
    pub(crate) state_path: RelativePathBuf,
    pub(crate) resolve_root: Utf8PathBuf,
    pub(crate) address: Option<Address>,
}

// impl From<StatePaths> for State {
//     fn from(value: StatePaths) -> Self {
//         State {
//             state_path: value.state_path,
//             state_root: value.state_root,
//             resolve_root: value.resolve_root,
//             address: None,
//         }
//     }
// }

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
        let address = other
            .address
            .map(|a| Address::from_config_addr(a, &self.resolve_root))
            .or(self.address.clone());

        Self {
            state_path: other
                .state_path
                .map(Into::into)
                .unwrap_or(self.state_path.clone()),
            state_root: other
                .state_root
                .map(Into::into)
                .unwrap_or(self.state_root.clone()),
            resolve_root: self.resolve_root.clone(),
            address,
        }
    }

    /// Checks if a state is different to another one for the purposes of converging to a state.
    fn same(&self, other: &Self) -> bool {
        self.resolve_root == other.resolve_root
            && self.state_path.normalize() == other.state_path.normalize()
            && self.state_root.normalize() == other.state_root.normalize()
    }
}

#[derive(Debug)]
pub(crate) struct ResolveState {
    pub(crate) state_root: Utf8PathBuf,
    pub(crate) state_path: RelativePathBuf,
    pub(crate) resolve_root: Utf8PathBuf,
    pub(crate) name: String,
    pub(crate) sub: String,
    pub(crate) hash: String,
}

#[instrument(name = "converge", level = "debug", skip_all)]
fn converge_state(state: &State) -> Result<State, StateError> {
    let mut state = state.clone();
    let mut i = 0;
    let iter = loop {
        let state_root_path = state.state_root.to_utf8_path(&state.resolve_root);
        let config = traverse_configs(&state_root_path, &state.state_path)
            .to_state_err("Failed to read configs for determining new state.".to_owned())?;
        let new_state = if let Some(config_state) = config.state {
            state.merge_config(config_state)
        } else {
            break i + 1;
        };
        debug!("New intermediate state {:?}", &new_state);

        let do_break = new_state.same(&state);
        state = new_state;
        if do_break {
            break i + 1;
        }

        i += 1;
    };
    debug!("Converged to state {:?} in {} iterations.", &state, iter);

    Ok(state)
}

/// Parse a repo URL to extract a "name" from it, as well as encode the part before the name to still uniquely
/// identify it. Only supports forward slashes as path seperator.
pub(crate) fn parse_url_repo_name(url: &str) -> Result<String, AddressError> {
    let url = url.strip_suffix('/').unwrap_or(url).to_owned();
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

fn resolve_address(address: Address, store_dir: &Utf8Path) -> Result<State, StateError> {
    let Address {
        state_path,
        state_root,
        root,
    } = address;

    match root {
        AddressRoot::Git(addr) => get_dir_from_git(addr, &state_path, &state_root, store_dir),
        AddressRoot::Local(path) => Ok(State {
            resolve_root: path,
            state_path,
            state_root,
            address: None,
        }),
    }
}

// fn set_current_dir(resolve_root: &Utf8Path) -> Result<(), StateError> {
//     debug!("Setting current dir to resolve root {}", resolve_root);

//     env::set_current_dir(resolve_root).to_state_err(format!("Failed to set current dir to context root {}", resolve_root))?;

//     Ok(())
// }

pub(crate) struct StateOptions {
    pub(crate) store_dir: Utf8PathBuf,
}

impl Default for StateOptions {
    fn default() -> Self {
        Self {
            store_dir: get_dirs().cache.clone(),
        }
    }
}

pub(crate) fn converge_address(address: Address, opt: StateOptions) -> Result<State, StateError> {
    let mut state = resolve_address(address, &opt.store_dir)?;
    state = converge_state(&state)?;

    while let Some(address) = state.address.clone() {
        state = resolve_address(address, &opt.store_dir)?;
        debug!("Moved to address, new state is {:?}", state);
        state = converge_state(&state)?;
    }

    Ok(state)
}

// #[instrument(name = "state", level = "debug", skip_all)]
// pub(crate) fn resolve_from_base_state(
//     mut state: State,
//     opt: StateOptions,
// ) -> Result<ResolveState, StateError> {

// }

pub(crate) fn create_resolve_state(
    addr_in: AddressIn,
    infer_ctx: InferContext,
    opt: StateOptions,
) -> Result<ResolveState, StateError> {
    let address = Address::from_addr_in(addr_in, infer_ctx)?;
    let state = converge_address(address, opt)?;

    let name = state
        .resolve_root
        .file_name()
        .map(|s| s.to_string())
        .ok_or_else(|| StateErrorKind::InvalidRoot(state.resolve_root.to_string()))
        .to_state_err("Getting context name from context root path for new state.".to_owned())?;

    Ok(ResolveState {
        state_root: state.state_root.to_utf8_path(&state.resolve_root),
        state_path: state.state_path,
        resolve_root: state.resolve_root,
        name,
        sub: "tidploy_root".to_owned(),
        hash: "todo_hash".to_owned(),
    })
}
