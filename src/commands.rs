use std::path::{Path, PathBuf};

use crate::archives::{extract_archive, make_archive};
use crate::errors::{ProcessError, RepoError};
use crate::filesystem::get_dirs;
use crate::git::{checkout, checkout_path, repo_clone, Repo};
use crate::next::run::run_command;
use crate::secret::{secret_command, AuthError};

use crate::next::process::run_entrypoint as next_run_entrypoint;
use crate::process::run_entrypoint;
use crate::state::{
    create_state_create, create_state_run, extra_envs, CliEnvState, LoadError, State, StateContext
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64USNP;
use base64::Engine;
use clap::{Parser, Subcommand};
use color_eyre::eyre::Report;

use std::fmt::Debug;
use std::time::Instant;

use thiserror::Error as ThisError;
use tracing::span;
use tracing::{debug, Level};

pub(crate) const DEFAULT_GIT_REMOTE: &str = "tidploy_default_git_remote";
pub(crate) const DEFAULT_GIT_LOCAL: &str = "tidploy_default_git_local";
pub(crate) const TIDPLOY_DEFAULT: &str = "tidploy_default";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Contexts other than git-remote (default) are not fully supported.
    #[arg(long, value_enum, global = true)]
    context: Option<StateContext>,

    /// Set the repository URL, defaults to 'default_infer', in which case it is inferred from the current repository.
    /// Set to 'default' to not set it.
    /// Falls back to environment variable using TIDPLOY_REPO and then to config with key 'repo_url'
    /// For infering, it looks at the URL set to the 'origin' remote.
    #[arg(short, long, global = true)]
    repo: Option<String>,

    /// The git reference (commit or tag) to use.
    #[arg(short, long, global = true)]
    tag: Option<String>,

    /// The path inside the repository that should be used as the primary config source.
    #[arg(short, long, global = true)]
    deploy_pth: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Save secret with key until reboot. Use the --tag option to scope it to a specific commit.
    Secret { key: String },
    /// Download tag or version with specific env, run automatically if using deploy
    Download {
        #[arg(long)]
        repo_only: bool,
    },

    /// Deploy tag or version with specific env
    Deploy {
        #[arg(short = 'x', long = "exe")]
        executable: Option<String>,

        /// Don't clone a fresh repository. Will fail if it does not exist. WARNING: The repository might not be up-to-date.
        #[arg(long)]
        no_create: bool,

        /// Variables to load. Supply as many pairs of <key> <env var name> as needed.
        #[arg(short, num_args = 2)]
        variables: Vec<String>,
    },
    /// Run an entrypoint or archive created by download/deploy and load secrets
    Run {
        #[arg(short = 'x', long = "exe")]
        executable: Option<String>,

        /// Variables to load. Supply as many pairs of <key> <env var name> as needed.
        #[arg(short, num_args = 2)]
        variables: Vec<String>,

        /// Give the exact name of the archive using the format:
        /// <repo name final path element without extension>_<commit sha>_<base64url-encoded url without name>
        #[arg(long)]
        archive: Option<String>,
    },
}

#[derive(ThisError, Debug)]
#[error(transparent)]
pub struct Error(#[from] ErrorRepr);

#[derive(ThisError, Debug)]
enum ErrorRepr {
    #[error("Load error failure! {0}")]
    Load(#[from] LoadError),
    #[error("Auth failure! {0}")]
    Auth(#[from] AuthError),
    #[error("Error running executable! {0}")]
    Exe(#[from] ProcessError),
    #[error("Error creating repository! {0}")]
    Repo(#[from] RepoError),
}

fn create_repo(repo: Repo) -> Result<PathBuf, RepoError> {
    let cache_dir = get_dirs().cache.as_path();
    let repo_name = repo.dir_name();
    let repo_path = cache_dir.join(&repo_name);

    repo_clone(cache_dir, &repo_name, &repo.url)?;

    Ok(repo_path)
}

fn switch_to_revision(
    cli_state: CliEnvState,
    state: State,
    repo_path: &Path,
) -> Result<State, ErrorRepr> {
    let commit_short = &state.commit_sha[0..7];
    let deploy_path_str = format!("{:?}", state.deploy_path);
    let checkedout_span = span!(
        Level::DEBUG,
        "checked_out",
        sha = commit_short,
        path = deploy_path_str
    );
    let _enter = checkedout_span.enter();

    // Checks out the correct commit
    checkout(repo_path, &state.commit_sha).map_err(ErrorRepr::Repo)?;
    // Does a sparse checkout of the deploy path
    checkout_path(repo_path, &state.deploy_path).map_err(ErrorRepr::Repo)?;

    // Creates state from the newly checked out path and state, which should now contain the correct config
    let state = create_state_create(
        cli_state,
        Some(repo_path),
        Some(state.deploy_path.as_relative_path()),
        true,
    )
    .map_err(ErrorRepr::Load)?;

    Ok(state)
}

fn prepare_from_state(state: &State, repo_path: &Path) -> Result<(), ErrorRepr> {
    let cache_dir = get_dirs().cache.as_path();
    let archives = cache_dir.join("archives");
    let deploy_encoded = B64USNP.encode(state.deploy_path.as_str());
    let archive_name = format!(
        "{}_{}_{}",
        state.repo.name, state.commit_sha, deploy_encoded
    );

    make_archive(
        &archives,
        cache_dir,
        repo_path.file_name().unwrap().to_string_lossy().as_ref(),
        &archive_name,
    )
    .map_err(ErrorRepr::Repo)?;

    Ok(())
}

fn download_command(
    cli_state: CliEnvState,
    repo: Repo,
    repo_only: bool,
) -> Result<Option<State>, ErrorRepr> {
    // This will be exited when `download_command` returns
    let download_span = span!(Level::DEBUG, "download");
    let _dl_enter = download_span.enter();

    let repo_path = create_repo(repo).map_err(ErrorRepr::Repo)?;

    if repo_only {
        return Ok(None);
    }

    // The preswitch stage creates state from the recently created repo, determining which commit sha to use for
    // the checkout and which deploy path to use
    let head_span = span!(Level::DEBUG, "preswitch").entered();
    let state = create_state_create(cli_state.clone(), Some(&repo_path), None, true)
        .map_err(ErrorRepr::Load)?;
    head_span.exit();

    let state = switch_to_revision(cli_state, state, &repo_path)?;

    prepare_from_state(&state, &repo_path)?;

    Ok(Some(state))
}

fn prepare_command(
    cli_state: CliEnvState,
    no_create: bool,
    repo: Repo,
) -> Result<Option<State>, ErrorRepr> {
    // This will be exited when `download_command` returns
    let prepare_san = span!(Level::DEBUG, "prepare");
    let _prep_enter = prepare_san.enter();

    let repo_path = if no_create {
        let cache_dir = get_dirs().cache.as_path();
        let repo_path = cache_dir.join(repo.dir_name());

        if !repo_path.exists() {
            return Err(RepoError::NotCreated.into());
        }

        repo_path
    } else {
        create_repo(repo).map_err(ErrorRepr::Repo)?
    };

    // The preswitch stage creates state from the recently created repo, determining which commit sha to use for
    // the checkout and which deploy path to use
    let head_span = span!(Level::DEBUG, "preswitch").entered();
    let state = create_state_create(cli_state.clone(), Some(&repo_path), None, true)
        .map_err(ErrorRepr::Load)?;
    head_span.exit();

    let state = switch_to_revision(cli_state, state, &repo_path)?;

    prepare_from_state(&state, &repo_path)?;

    Ok(Some(state))
}

pub(crate) fn run_cli() -> Result<(), Report> {
    // We get our CLI arguments using the clap crate. This allows us to state all our arguments using
    // a set of structs, indicating the structure of our commands
    // Note that it uses the Cargo.toml description as the main help command description
    let args = Cli::parse();

    let cli_state = CliEnvState {
        context: args.context,
        repo_url: args.repo,
        deploy_path: args.deploy_pth,
        tag: args.tag,
    };

    debug!("Parsed CLI state as {:?}", cli_state);

    match args.command {
        Commands::Secret { key } => {
            let auth_span = span!(Level::DEBUG, "auth");
            let _auth_enter = auth_span.enter();
            let state =
                create_state_create(cli_state, None, None, false).map_err(ErrorRepr::Load)?;

            secret_command(&state, key).map_err(ErrorRepr::Auth)?;

            Ok(())
        }
        Commands::Download { repo_only } => {
            let state = create_state_create(cli_state.clone(), None, None, false)
                .map_err(ErrorRepr::Load)?;
            download_command(cli_state, state.repo, repo_only)?;

            Ok(())
        }
        Commands::Deploy {
            executable,
            no_create,
            variables,
        } => {
            // We drop the dpl_enter when exiting this scope
            let deploy_span = span!(Level::DEBUG, "deploy");
            let _dpl_enter = deploy_span.enter();

            // This one we must manually exit so we use 'entered'. The preprepare stage determines the "repo".
            let enter_dl = span!(Level::DEBUG, "preprepare").entered();
            let state = create_state_create(cli_state.clone(), None, None, false)
                .map_err(ErrorRepr::Load)?;
            enter_dl.exit();

            let state = prepare_command(cli_state.clone(), no_create, state.repo)?.unwrap();
            let cache_dir = get_dirs().cache.as_path();
            let tmp_dir = get_dirs().tmp.as_path();
            let deploy_encoded = B64USNP.encode(state.deploy_path.as_str());
            let archive_name = format!(
                "{}_{}_{}",
                state.repo.name, state.commit_sha, deploy_encoded
            );
            let archive_path = cache_dir
                .join("archives")
                .join(&archive_name)
                .with_extension("tar.gz");
            extract_archive(&archive_path, tmp_dir, &archive_name).map_err(ErrorRepr::Repo)?;

            let target_path_root = tmp_dir.join(archive_name);
            let state = create_state_run(
                cli_state,
                executable,
                variables,
                Some(target_path_root.as_path()),
                Some(state.deploy_path.as_relative_path()),
                true,
            )
            .map_err(ErrorRepr::Load)?;

            let state = extra_envs(state);

            run_entrypoint(state.deploy_dir(), &state.exe_name, state.envs)
                .map_err(ErrorRepr::Exe)?;

            Ok(())
        }
        Commands::Run {
            executable,
            variables,
            archive,
        } => {
            run_command(cli_state, executable, variables, archive)
        }
    }
}
