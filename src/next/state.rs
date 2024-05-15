use std::env::current_dir;

use camino::{Utf8Path, Utf8PathBuf};
use relative_path::{RelativePath, RelativePathBuf};
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
    pub local: bool,
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
        local: bool,
        git_ref: Option<String>,
        target_resolve_root: Option<String>,
        state_path: Option<String>,
        state_root: Option<String>,
    ) -> Self {
        AddressIn::Git(GitAddressIn {
            url,
            local,
            git_ref,
            target_resolve_root,
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

#[derive(Debug, Clone)]
pub(crate) struct Address {
    pub(crate) name: String,
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

fn url_local(url: String, local: bool, relative: &Utf8Path) -> String {
    if local {
        let path = Utf8Path::new(&url);
        if path.is_relative() {
            RelativePath::new(&url).to_utf8_path(relative).as_str().to_owned()
        } else {
            url
        }
    } else {
        url
    }
}

impl Address {
    fn from_config_addr(value: ConfigAddress, resolve_root: &Utf8Path) -> Result<Self, StateError> {
        debug!("Converting config_adress {:?} to address!", value);

        let addr = match value {
            ConfigAddress::Git {
                url,
                local,
                git_ref,
                target_path,
                state_path,
                state_root,
            } => {
                let local = local.unwrap_or_default();
                let url = url_local(url, local, resolve_root);
                let name = parse_url_name(&url).to_state_err("Cannot get name from url.")?;
                Address {
                    name,
                    root: AddressRoot::Git(GitAddress {
                        url,
                        local,
                        git_ref,
                        path: RelativePathBuf::from(target_path.unwrap_or_default()),
                    }),
                    state_path: RelativePathBuf::from(state_path.unwrap_or_default()),
                    state_root: RelativePathBuf::from(state_root.unwrap_or_default()),
                }
        }   ,
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

                let name = parse_url_name(root.as_str()).to_state_err("Error getting name from address root!")?;

                Address {
                    name,
                    root: AddressRoot::Local(root),
                    state_path: RelativePathBuf::from(state_path.unwrap_or_default()),
                    state_root: RelativePathBuf::from(state_root.unwrap_or_default()),
                }
            }
        };

        Ok(addr)
    }

    fn from_addr_in(value: AddressIn, infer_ctx: InferContext) -> Result<Self, StateError> {
        debug!("Converting config_adress {:?} to address!", value);

        let addr = match value {
            AddressIn::Git(GitAddressIn {
                url,
                local,
                git_ref,
                state_path,
                state_root,
                target_resolve_root,
            }) => {
                let current_dir = get_current_dir()?;
                
                let url = if let Some(url) = url {
                    url_local(url, local, &current_dir)
                } else {
                    match infer_ctx {
                        InferContext::Git => git_root_origin_url(&current_dir)
                            .to_state_err("Error resolving current Git repository.")?,
                        InferContext::Cwd => current_dir.to_string(),
                    }

                    // If you want to start with a Git repo but use a different resolve root and not give the URL
                    // Just start in a different directory!
                };
                let name = parse_url_name(&url).to_state_err("Cannot get name from url.")?;
                Address {
                    name,
                    root: AddressRoot::Git(GitAddress {
                        url,
                        local,
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
                let name = parse_url_name(resolve_root.as_str())
                    .to_state_err("Cannot get name from resolve root.")?;
                Address {
                    name,
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
    pub(crate) local: bool,
    pub(crate) git_ref: String,
    pub(crate) path: RelativePathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct State {
    pub(crate) name: String,
    pub(crate) state_root: RelativePathBuf,
    pub(crate) state_path: RelativePathBuf,
    pub(crate) resolve_root: Utf8PathBuf,
    pub(crate) address: Option<Address>,
}

impl State {
    fn merge_config(&self, other: StateConfig) -> Result<Self, StateError> {
        let address = other
            .address
            .map(|a| Address::from_config_addr(a, &self.resolve_root))
            .transpose()?.or(self.address.clone());

        let state = Self {
            name: self.name.clone(),
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
        };

        Ok(state)
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
            .to_state_err("Failed to read configs for determining new state.")?;
        let new_state = if let Some(config_state) = config.state {
            state.merge_config(config_state)?
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
pub(crate) fn parse_url_name(url: &str) -> Result<String, AddressError> {
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
    debug!("Resolving address {:?}", address);
    
    let Address {
        name,
        state_path,
        state_root,
        root,
    } = address;
    

    match root {
        AddressRoot::Git(addr) => get_dir_from_git(addr, &state_path, &state_root, store_dir),
        AddressRoot::Local(path) => Ok(State {
            name,
            resolve_root: path,
            state_path,
            state_root,
            address: None,
        }),
    }
}

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

pub(crate) fn create_resolve_state(
    addr_in: AddressIn,
    infer_ctx: InferContext,
    opt: StateOptions,
) -> Result<ResolveState, StateError> {
    let address = Address::from_addr_in(addr_in, infer_ctx)?;
    let state = converge_address(address, opt)?;

    

    // let name = state
    //     .resolve_root
    //     .file_name()
    //     .map(|s| s.to_string())
    //     .ok_or_else(|| StateErrorKind::InvalidRoot(state.resolve_root.to_string()))
    //     .to_state_err("Getting context name from context root path for new state.")?;

    let resolve_state = ResolveState {
        state_root: state.state_root.to_utf8_path(&state.resolve_root),
        state_path: state.state_path,
        resolve_root: state.resolve_root,
        name: state.name,
        sub: "tidploy_root".to_owned(),
        hash: "todo_hash".to_owned(),
    };

    debug!("Created resolve state as {:?}", resolve_state);

    Ok(resolve_state)
}
