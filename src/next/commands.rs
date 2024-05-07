use std::process::ExitCode;

use clap::{Args, Command, Subcommand};
use color_eyre::eyre::Report;

use super::{run::run_command, secrets::secret_command, state::AddressIn};

#[derive(Debug, Args)]
pub struct NextSub {
    #[clap(subcommand)]
    pub subcommand: NextCommands,

    // /// Contexts other than git-remote (default) are not fully supported.
    // #[arg(long, value_enum, global = true)]
    // context: Option<StateContext>,

    // /// Set the repository URL, defaults to 'default_infer', in which case it is inferred from the current repository.
    // /// Set to 'default' to not set it.
    // /// Falls back to environment variable using TIDPLOY_REPO and then to config with key 'repo_url'
    // /// For infering, it looks at the URL set to the 'origin' remote.
    // #[arg(short, long, global = true)]
    // repo: Option<String>,

    // /// The git reference (commit or tag) to use.
    // #[arg(short, long, global = true)]
    // tag: Option<String>,

    // /// The path inside the repository that should be used as the primary config source.
    // #[arg(short, long, global = true)]
    // deploy_pth: Option<String>,
    /// Directory to start resolving from. Can either be an absolute path (this requires --cwd), or relative to
    /// the current directory or Git root dir
    #[arg(long = "resolve-root")]
    resolve_root: Option<String>,

    /// Location relative to state root to stop reading configs, inclusive.
    #[arg(long = "state-root")]
    state_root: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum NextCommands {
    /// Save secret with key until reboot.
    Secret {
        key: String,

        /// By default, tidploy searches for the root directory of the Git repository that the command is called
        /// from and takes all other inputs as relative to there. To instead ignore the current Git repository
        /// and simply take the current working directory as the root, enable this flag.
        #[arg(short = 'c', long = "cwd")]
        cwd_infer: bool,

        #[arg(long = "state-path")]
        state_path: Option<String>,
    },

    /// Run an entrypoint or archive created by download/deploy and load secrets
    Run {
        executable: Option<String>,

        #[arg(long = "state-path")]
        state_path: Option<String>,

        /// Working directory for execution of the executable relative to the resolution root.
        #[arg(long = "exn-path")]
        execution_path: Option<String>,

        /// Variables to load. Supply as many pairs of <key> <env var name> as needed.
        #[arg(short, num_args = 2)]
        variables: Vec<String>,

        #[arg(short = 'G', long = "GR")]
        git_infer: bool,
    },

    Deploy {
        git_ref: Option<String>,

        state_path: Option<String>,

        /// Relative path of the executable relative to the resolution root.
        #[arg(short = 'x', long = "exe")]
        executable: Option<String>,

        /// Working directory for execution of the executable relative to the resolution root.
        #[arg(long = "exn-path")]
        execution_path: Option<String>,

        /// Variables to load. Supply as many pairs of <key> <env var name> as needed.
        #[arg(short, num_args = 2)]
        variables: Vec<String>,

        #[arg(short = 'c', long = "cwd")]
        cwd_infer: bool,

        #[arg(long = "repo")]
        repo: Option<String>,
    },
    // Address {
    //     /// Location relative to resolve root where you want to begin reading configs. Defaults to be equal
    //     /// to resolve root.
    //     #[arg(long = "a-state-root")]
    //     address_state_root: Option<String>,

    //     /// Location relative to state root to stop reading configs, inclusive.
    //     #[arg(long = "a-state-path")]
    //     address_state_path: Option<String>,

    //     #[clap(subcommand)]
    //     subcommand: AddressSubCommands,
    // }
}

#[derive(Subcommand, Debug)]
pub enum AddressSubCommands {
    Git {
        url: String,

        git_ref: String,

        #[arg(long = "a-target-path")]
        target_path: Option<String>,
    },
    Local {
        path: String,
    },
}

pub fn match_command(next_sub: NextSub, _cmd: Command) -> Result<ExitCode, Report> {
    let NextSub {
        subcommand,
        state_root,
        resolve_root,
    } = next_sub;

    match subcommand {
        NextCommands::Secret {
            key,
            cwd_infer,
            state_path,
        } => {
            let addr_in = AddressIn::from_secret(resolve_root, state_path, state_root);

            secret_command(addr_in, cwd_infer, None, None, key, None)?;

            Ok(ExitCode::from(0))
        }
        NextCommands::Run {
            executable,
            variables,
            execution_path,
            git_infer,
            state_path,
        } => {
            let addr_in = AddressIn::from_run(resolve_root, state_path, state_root);
            let out = run_command(
                addr_in,
                git_infer,
                None,
                executable,
                execution_path,
                variables,
            )?;
            // If [process::ExitCode::from_raw] gets stabilized this can be simplified
            let code = u8::try_from(out.exit.code().unwrap_or(0))?;

            Ok(ExitCode::from(code))
        }
        NextCommands::Deploy {
            executable,
            variables,
            execution_path,
            cwd_infer,
            repo,
            git_ref,
            state_path,
        } => {
            let addr_in =
                AddressIn::from_deploy(repo, git_ref, resolve_root, state_path, state_root);
            let out = run_command(
                addr_in,
                !cwd_infer,
                None,
                executable,
                execution_path,
                variables,
            )?;
            // If [process::ExitCode::from_raw] gets stabilized this can be simplified
            let code = u8::try_from(out.exit.code().unwrap_or(0))?;

            Ok(ExitCode::from(code))
        }
        // NextCommands::Address {
        //     address_state_root,
        //     address_state_path,
        //     subcommand: addr_subcommands } => match addr_subcommands {
        //         AddressSubCommands::Git {
        //             url,
        //             git_ref,
        //             target_path
        //         } => {
        //             let addr_in = AddressIn::Git {
        //                 url,
        //                 git_ref,
        //                 target_path,
        //                 state_path: address_state_path,
        //                 state_root: address_state_root
        //             };

        //             let state_in = StateIn::from_args(true, resolve_root, state_path, state_root);

        //             Ok(())
        //         },
        //         AddressSubCommands::Local { path } => {
        //             let addr_in = AddressIn::Local {
        //                 path,
        //                 state_path: address_state_path,
        //                 state_root: address_state_root
        //             };

        //             let state_in = StateIn::from_args(true, resolve_root, state_path, state_root);

        //             Ok(())
        //         },
        //     }
    }
}
