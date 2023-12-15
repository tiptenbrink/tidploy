mod archives;
mod auth;
mod commands;
mod config;
mod errors;
mod filesystem;
mod git;
mod process;
mod secret_store;
mod state;

fn main() {
    let program = commands::run_cli();
    if let Err(program_err) = program {
        eprintln!("Error: {}", program_err);
        std::process::exit(1)
    }
}
