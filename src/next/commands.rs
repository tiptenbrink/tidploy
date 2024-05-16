use std::process::ExitCode;

use clap::{Args, Command, Subcommand};
use color_eyre::eyre::Report;

use super::{run::run_command, secrets::secret_command, state::AddressIn};

#[derive(Debug, Args)]
pub struct NextSub {
    #[clap(subcommand)]
    pub subcommand: NextCommands,

    /// Directory to start resolving from. Can either be an absolute path (this requires --cwd), or relative to
    /// the current directory or Git root dir
    #[arg(long = "resolve-root")]
    resolve_root: Option<String>,
    // /// Location relative to state root to stop reading configs, inclusive.
    // #[arg(long = "state-root")]
    // state_root: Option<String>,
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

        #[arg(long = "local")]
        local: bool,
    },
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
        resolve_root,
    } = next_sub;

    match subcommand {
        NextCommands::Secret {
            key,
            cwd_infer,
            state_path,
        } => {
            let addr_in = AddressIn::from_secret(resolve_root, state_path);

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
            let addr_in = AddressIn::from_run(resolve_root, state_path);
            let out = run_command(
                addr_in,
                git_infer,
                None,
                executable,
                execution_path,
                variables,
            )?;
            let code = u8::try_from(out.exit.code().unwrap_or(0))?;

            Ok(ExitCode::from(code))
        }
        NextCommands::Deploy {
            executable,
            variables,
            local,
            execution_path,
            cwd_infer,
            repo,
            git_ref,
            state_path,
        } => {
            let addr_in = AddressIn::from_deploy(repo, local, git_ref, resolve_root, state_path);
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
    }
}
