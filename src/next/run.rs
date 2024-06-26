use color_eyre::eyre::{Context, Report};
use relative_path::RelativePathBuf;
use tracing::{debug, instrument};

use crate::{
    archives::extract_archive,
    filesystem::{get_dirs, WrapToPath},
    next::{
        resolve::{resolve_run, Resolved, RunArguments, SecretScopeArguments},
        secrets::secret_vars_to_envs,
        state::{create_resolve_state, parse_cli_vars, InferContext},
    },
    state::{create_state_create, create_state_run, CliEnvState},
};

use super::{
    process::{run_entrypoint, EntrypointOut},
    resolve::RunResolved,
    state::{AddressIn, StateOptions},
};

pub(crate) fn run_command(
    address_in: AddressIn,
    git_infer: bool,
    service: Option<String>,
    executable: Option<String>,
    execution_path: Option<String>,
    variables: Vec<String>,
) -> Result<EntrypointOut, Report> {
    run_command_input(
        address_in,
        git_infer,
        None,
        RunOptions {
            service,
            input_bytes: None,
        },
        executable,
        execution_path,
        variables,
    )
}

#[instrument(name = "run", level = "debug", skip_all)]
pub(crate) fn run_command_input_old_state(
    cli_state: CliEnvState,
    executable: Option<String>,
    variables: Vec<String>,
    archive: Option<String>,
    input_bytes: Option<Vec<u8>>,
) -> Result<EntrypointOut, Report> {
    // Only loads archive if it is given, otherwise path is None
    let state = if let Some(archive) = archive {
        let cache_dir = get_dirs().wrap_err("Cache is not UTF-8!")?.cache.as_path();
        let archive_path = cache_dir
            .join("archives")
            .join(&archive)
            .with_extension("tar.gz");

        let tmp_dir = get_dirs().wrap_err("Cache is not UTF-8!")?.tmp.as_path();
        let extracted_path =
            extract_archive(&archive_path, tmp_dir, &archive).wrap_err("Repo error.")?;
        debug!("Extracted and loaded archive at {:?}", &extracted_path);

        let state = create_state_create(cli_state.clone(), Some(&extracted_path), None, true)
            .wrap_err("Load error.")?;

        Some(state)
    } else {
        debug!("No archive provided to run command.");
        None
    };
    let root_dir = state.as_ref().map(|state| state.root_dir.as_path());
    let deploy_path = state
        .as_ref()
        .map(|state| state.deploy_path.as_relative_path());

    let state = create_state_run(
        cli_state,
        executable,
        variables,
        root_dir,
        deploy_path,
        true,
    )
    .wrap_err("Create state error.")?;

    let relative_path = RelativePathBuf::from(&state.exe_name);
    let exe_path = relative_path.to_utf8_path(state.deploy_dir());
    run_entrypoint(&state.deploy_dir(), &exe_path, state.envs, input_bytes)
}

pub(crate) struct RunOptions {
    pub(crate) service: Option<String>,
    pub(crate) input_bytes: Option<Vec<u8>>,
}

#[instrument(name = "run", level = "debug", skip_all)]
pub(crate) fn run_command_input(
    addr_in: AddressIn,
    git_infer: bool,
    state_options: Option<StateOptions>,
    run_options: RunOptions,
    executable: Option<String>,
    execution_path: Option<String>,
    variables: Vec<String>,
) -> Result<EntrypointOut, Report> {
    debug!("Run command called with addr_in {:?}, executable {:?}, variables {:?} and input_bytes {:?}", addr_in, executable, variables, run_options.input_bytes);

    let scope_args = SecretScopeArguments {
        service: run_options.service,
        ..Default::default()
    };
    let infer_ctx = if git_infer {
        InferContext::Git
    } else {
        InferContext::Cwd
    };
    let resolve_state =
        create_resolve_state(addr_in, infer_ctx, state_options.unwrap_or_default())?;
    let run_args = RunArguments {
        executable: executable.resolve(&resolve_state.resolve_root),
        execution_path: execution_path.resolve(&resolve_state.resolve_root),
        envs: parse_cli_vars(variables),
        scope_args,
    };

    let run_resolved = resolve_run(resolve_state, run_args)?;

    run_unit_input(run_resolved, run_options.input_bytes)
}

pub(crate) fn run_unit_input(
    run_resolved: RunResolved,
    input_bytes: Option<Vec<u8>>,
) -> Result<EntrypointOut, Report> {
    let secret_vars = secret_vars_to_envs(&run_resolved.scope, run_resolved.envs)?;

    run_entrypoint(
        &run_resolved.execution_path,
        &run_resolved.executable,
        secret_vars,
        input_bytes,
    )
}
