mod cli;
mod config;
mod secret_store;
mod secrets;

fn main() {
    cli::run_cli().unwrap()
}
