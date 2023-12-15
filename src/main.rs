mod cli;
mod config;
mod errors;
mod secret_store;
mod secrets;
mod commands;
mod git;
mod filesystem;
mod state;

fn main() {
    let program = cli::run_cli();
    if let Err(program_err) = program {
        eprintln!("Error: {}", program_err);
        std::process::exit(1)
    }
}
