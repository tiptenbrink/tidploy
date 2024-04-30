use std::process::ExitCode;

use tidploy::commands;

use color_eyre::eyre::Report;
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;

/// Sets up the tracing crate infrastructure, which is what actually collects all debug info and prints it
fn install_tracing() {
    // We have to add the error layer (see the examples in color-eyre), so we can't just use the default init
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

fn main() -> Result<ExitCode, Report> {
    install_tracing();
    color_eyre::install()?;

    commands::run_cli()
}
