mod archives;
mod commands;
mod config;
mod errors;
mod filesystem;
mod git;
mod process;
mod secret;
mod secret_store;
mod state;
mod next;

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::fmt;

    let fmt_layer = fmt::layer().with_target(false);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
    // use tracing_error::ErrorLayer;
    // use tracing_subscriber::prelude::*;

    // // Default layer for showing debug traces
    // let fmt_layer = tracing_subscriber::fmt::layer();

    // // We add the tracing error error layer
    // tracing_subscriber::registry()
    //     .with(fmt_layer)
    //     .with(ErrorLayer::default())
    //     .init();
}

use color_eyre::eyre::Report;

fn main() -> Result<(), Report> {
    //tracing_subscriber::fmt::init();
    install_tracing();

    color_eyre::install()?;

    commands::run_cli()
}
