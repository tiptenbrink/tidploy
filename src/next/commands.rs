use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct NextSub {
    #[clap(subcommand)]
    pub subcommand: NextCommands,
}

#[derive(Subcommand, Debug)]
pub enum NextCommands {
    /// Save secret with key until reboot. Use the --tag option to scope it to a specific commit.
    Secret { key: String },

    /// Run an entrypoint or archive created by download/deploy and load secrets
    Run {
        #[arg(short = 'x', long = "exe")]
        executable: Option<String>,

        /// Variables to load. Supply as many pairs of <key> <env var name> as needed.
        #[arg(short, num_args = 2)]
        variables: Vec<String>,
    },
}
