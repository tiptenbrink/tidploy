mod archives;
mod secret;
mod commands;
mod config;
mod errors;
mod filesystem;
mod git;
mod process;
mod secret_store;
mod state;
use tracing_subscriber;

fn main() {
    tracing_subscriber::fmt::init();
    
    let program = commands::run_cli();
    if let Err(program_err) = program {
        eprintln!("Error: {}", program_err);
        std::process::exit(1)
    }
}
