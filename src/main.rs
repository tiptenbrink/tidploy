mod cli;
mod config;
mod secret_store;
mod secrets;
mod errors;

fn main() {
    cli::run_cli().unwrap()
}
