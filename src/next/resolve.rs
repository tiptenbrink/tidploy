use std::{env, fmt::Debug};

use camino::{Utf8Path, Utf8PathBuf};
use relative_path::{RelativePath, RelativePathBuf};
use tracing::{debug, instrument};

use crate::filesystem::WrapToPath;

use super::{
    config::{merge_vars, traverse_configs, ArgumentConfig, ConfigScope, ConfigVar},
    errors::ResolutionError,
    state::ResolveState,
};

#[derive(Default)]
pub(crate) struct SecretScopeArguments {
    pub(crate) name: Option<String>,
    pub(crate) sub: Option<String>,
    pub(crate) service: Option<String>,
    pub(crate) require_hash: Option<bool>,
}

impl SecretScopeArguments {
    /// Overrides fields with other if other has them defined
    fn merge(self, other: Self) -> Self {
        Self {
            service: other.service.or(self.service),
            sub: other.sub.or(self.sub),
            name: other.name.or(self.name),
            require_hash: other.require_hash.or(self.require_hash),
        }
    }
}

impl From<ConfigScope> for SecretScopeArguments {
    fn from(value: ConfigScope) -> Self {
        Self {
            service: value.service,
            name: value.name,
            sub: value.sub,
            require_hash: value.require_hash,
        }
    }
}

#[derive(Default)]
pub(crate) struct RunArguments {
    pub(crate) executable: Option<String>,
    pub(crate) execution_path: Option<String>,
    pub(crate) envs: Vec<ConfigVar>,
    pub(crate) scope_args: SecretScopeArguments,
}

impl RunArguments {
    /// Overrides fields with other if other has them defined
    fn merge(self, other: Self) -> Self {
        Self {
            executable: other.executable.or(self.executable),
            execution_path: other.execution_path.or(self.execution_path),
            envs: merge_vars(self.envs, other.envs),
            scope_args: self.scope_args.merge(other.scope_args),
        }
    }
}

impl From<ArgumentConfig> for RunArguments {
    fn from(value: ArgumentConfig) -> Self {
        RunArguments {
            executable: value.executable,
            execution_path: value.execution_path,
            envs: value.envs.unwrap_or_default(),
            scope_args: value.scope.map(|s| s.into()).unwrap_or_default(),
        }
    }
}

pub(crate) struct SecretArguments {
    pub(crate) key: String,
    pub(crate) scope_args: SecretScopeArguments,
}

#[derive(Debug)]
pub(crate) struct SecretScope {
    pub(crate) service: String,
    pub(crate) name: String,
    pub(crate) sub: String,
    pub(crate) hash: String,
}

#[derive(Debug)]
pub(crate) struct RunResolved {
    pub(crate) executable: Utf8PathBuf,
    pub(crate) execution_path: Utf8PathBuf,
    pub(crate) envs: Vec<ConfigVar>,
    pub(crate) scope: SecretScope,
}

#[derive(Debug)]
pub(crate) struct SecretResolved {
    pub(crate) key: String,
    pub(crate) scope: SecretScope,
}

fn env_scope_args() -> SecretScopeArguments {
    let mut scope_args = SecretScopeArguments::default();

    for (k, v) in env::vars() {
        match k.as_str() {
            "TIDPLOY_SECRET_SCOPE_NAME" => scope_args.name = Some(v),
            "TIDPLOY_SECRET_SCOPE_SUB" => scope_args.sub = Some(v),
            "TIDPLOY_SECRET_SERVICE" => scope_args.service = Some(v),
            "TIDPLOY_SECRET_REQUIRE_HASH" => scope_args.require_hash = Some(v.len() > 0),
            _ => {}
        }
    }

    scope_args
}

/// Note that `key` cannot be set from env and must thus always be replaced with some sensible value.
fn env_secret_args() -> SecretArguments {
    SecretArguments {
        key: "".to_owned(),
        scope_args: env_scope_args(),
    }
}

/// Note that `envs` cannot be set from env and must thus always be replaced with some sensible value.
fn env_run_args() -> RunArguments {
    let scope_args = env_scope_args();
    let mut run_arguments = RunArguments {
        scope_args,
        ..Default::default()
    };

    for (k, v) in env::vars() {
        match k.as_str() {
            "TIDPLOY_RUN_EXECUTABLE" => run_arguments.executable = Some(v),
            "TIDPLOY_RUN_EXECUTION_PATH" => run_arguments.execution_path = Some(v),
            _ => {}
        }
    }

    run_arguments
}

pub(crate) trait Resolve<Resolved>: Sized {
    fn merge_env_config(
        self,
        state_root: &Utf8Path,
        state_path: &RelativePath,
    ) -> Result<Self, ResolutionError>;

    fn resolve(self, resolve_root: &Utf8Path, name: &str, sub: &str, hash: &str) -> Resolved;
}

fn resolve_scope(
    scope_args: SecretScopeArguments,
    name: &str,
    sub: &str,
    hash: &str,
) -> SecretScope {
    SecretScope {
        service: scope_args.service.unwrap_or("tidploy".to_owned()),
        name: scope_args.name.unwrap_or(name.to_owned()),
        sub: scope_args.sub.unwrap_or(sub.to_owned()),
        hash: if scope_args.require_hash.unwrap_or(false) {
            hash.to_owned()
        } else {
            "tidploy_default_hash".to_owned()
        },
    }
}

impl Resolve<RunResolved> for RunArguments {
    fn merge_env_config(
        self,
        state_root: &Utf8Path,
        state_path: &RelativePath,
    ) -> Result<Self, ResolutionError> {
        let config = traverse_configs(state_root, state_path)?;

        let run_args_env = env_run_args();

        let merged_args = run_args_env.merge(self);

        let config_run = config.argument.map(RunArguments::from).unwrap_or_default();

        Ok(config_run.merge(merged_args))
    }

    fn resolve(self, resolve_root: &Utf8Path, name: &str, sub: &str, hash: &str) -> RunResolved {
        let scope = resolve_scope(self.scope_args, name, sub, hash);

        let relative_exe = RelativePathBuf::from(self.executable.unwrap_or("".to_owned()));
        let relative_exn_path = RelativePathBuf::from(self.execution_path.unwrap_or("".to_owned()));
        RunResolved {
            executable: relative_exe.to_utf8_path(resolve_root),
            execution_path: relative_exn_path.to_utf8_path(resolve_root),
            envs: self.envs,
            scope,
        }
    }
}

impl Resolve<SecretResolved> for SecretArguments {
    fn merge_env_config(
        self,
        state_root: &Utf8Path,
        state_path: &RelativePath,
    ) -> Result<Self, ResolutionError> {
        let config = traverse_configs(state_root, state_path)?;

        let secret_args_env = env_secret_args();

        let mut merged_args = SecretArguments {
            key: self.key,
            scope_args: secret_args_env.scope_args.merge(self.scope_args),
        };

        let config_scope = config
            .argument
            .and_then(|a| a.scope)
            .map(SecretScopeArguments::from)
            .unwrap_or_default();

        merged_args.scope_args = config_scope.merge(merged_args.scope_args);

        Ok(merged_args)
    }

    fn resolve(
        self,
        _resolve_root: &Utf8Path,
        name: &str,
        sub: &str,
        hash: &str,
    ) -> SecretResolved {
        let scope = resolve_scope(self.scope_args, name, sub, hash);

        SecretResolved {
            key: self.key,
            scope,
        }
    }
}

/// Loads config, environment variables and resolves the final arguments to make them ready for final use
#[instrument(name = "merge_resolve", level = "debug", skip_all)]
pub(crate) fn merge_and_resolve<T: Debug>(
    unresolved_args: impl Resolve<T>,
    state: ResolveState,
) -> Result<T, ResolutionError> {
    let merged_args = unresolved_args.merge_env_config(&state.state_root, &state.state_path)?;

    let resolved = merged_args.resolve(&state.resolve_root, &state.name, &state.sub, &state.hash);
    debug!("Resolved as {:?}", resolved);
    Ok(resolved)
}