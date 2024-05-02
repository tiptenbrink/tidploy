use std::{collections::HashMap, env, path::{Path, PathBuf}};

use relative_path::RelativePath;

use super::{config::traverse_configs, errors::ResolutionError};

#[derive(Default)]
struct SecretScopeArguments {
    name: Option<String>,
    sub: Option<String>,
    service: Option<String>
}

impl SecretScopeArguments {
    fn merge(&self, other: Self) -> Self {
        Self {
            service: other.service.or(self.service),
            sub: other.sub.or(self.sub),
            name: other.name.or(self.name),
        }
    }
}

#[derive(Default)]
struct RunArguments {
    executable: Option<String>,
    execution_path: Option<String>,
    envs: Vec<String>,
    scope_args: SecretScopeArguments
}

struct SecretArguments {
    key: String,
    scope_args: SecretScopeArguments
}

struct SecretScope {
    service: String,
    name: String,
    sub: String,
    hash: String
}

struct RunResolved {
    executable: PathBuf,
    execution_path: PathBuf,
    envs: HashMap<String, String>,
    scope: SecretScope
}

struct SecretResolved {
    key: String,
    scope: SecretScope
}

enum OutArguments {
    Secret,
    Run
}

enum Arguments {
    Secret(SecretArguments),
    Run(RunArguments)
}

fn env_scope_args() -> SecretScopeArguments {
    let mut scope_args = SecretScopeArguments::default();

    for (k, v) in env::vars() {
        match k.as_str() {
            "TIDPLOY_SECRET_SCOPE_NAME" => scope_args.name = Some(v),
            "TIDPLOY_SECRET_SCOPE_SUB" => scope_args.sub = Some(v),
            "TIDPLOY_SECRET_SERVICE" => scope_args.service = Some(v),
            _ => {}
        }
    }

    scope_args
}

/// Note that `key` cannot be set from env and must thus always be replaced with some sensible value.
fn env_secret_args() -> SecretArguments {
    SecretArguments {
        key: "".to_owned(),
        scope_args: env_scope_args()
    }
}

/// Note that `envs` cannot be set from env and must thus always be replaced with some sensible value.
fn env_run_args() -> RunArguments {
    let scope_args = env_scope_args();
    let mut run_arguments = RunArguments::default();
    run_arguments.scope_args = scope_args;

    for (k, v) in env::vars() {
        match k.as_str() {
            "TIDPLOY_RUN_EXECUTABLE" => run_arguments.executable = Some(v),
            "TIDPLOY_RUN_EXECUTION_PATH" => run_arguments.execution_path = Some(v),
            _ => {}
        }
    }

    run_arguments
}

fn merge_scope(root_config: SecretScope, overwrite_config: SecretScope) -> SecretScope {
    ArgumentConfig {
        scope,
        executable,
        execution_path,
        envs
    }
}

fn resolve(state_root: &Path, state_path: &RelativePath, resolve_root: &Path, hash: String, args: Arguments) -> Result<(), ResolutionError> {
    let config = traverse_configs(state_root, state_path)?;

    match args {
        Arguments::Secret(secret_args) => {
            let secret_args_env = env_secret_args();

            let merged_args = SecretArguments {
                key: secret_args.key,
                scope_args: secret_args_env.scope_args.merge(secret_args.scope_args)
            };

            let merged_args = if let Some(config_args) = config.argument {
                if let Some(config_scope) = config_args.scope {
                    let scope_args = merged_args.scope_args;
                    SecretArguments {
                        key: merged_args.key,
                        scope_args: SecretScopeArguments {
                            service: scope_args.service.or(config_scope.service),
                            name: scope_args.name.or(config_scope.name),
                            sub: scope_args.sub.or(config_scope.sub),
                        }
                    }
                } else {
                    merged_args
                }
            } else {
                merged_args
            };
        }
    }

    Ok(())
}