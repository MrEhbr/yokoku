#![forbid(unsafe_code)]

mod app;
mod args;
mod commands;
mod config;
mod logging;
mod secret;
mod subscriptions;

use std::io;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches};

use crate::args::Args;

async fn run() -> Result<()> {
    let matches = Args::command().get_matches();
    let command = matches.subcommand_name().unwrap_or_default().to_owned();
    let args = Args::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    args::route(args, &command).await
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        if is_broken_pipe(&error) {
            return;
        }
        eprintln!("Error: {:?}", error);
        std::process::exit(1);
    }
}

/// The reader of stdout went away, e.g. `yokoku list | head`.
fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<io::Error>())
        .any(|error| error.kind() == io::ErrorKind::BrokenPipe)
}
