#![forbid(unsafe_code)]

#[macro_use]
mod cli;

mod app;
mod config;
mod jobs;
mod logging;
mod service;
mod subscriptions;

use std::io;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches};
use owo_colors::{OwoColorize, Stream, Style};

use crate::cli::args::{self, Args};

async fn run() -> Result<()> {
    let matches = Args::command().get_matches();
    let command = matches.subcommand_name().unwrap_or("service").to_owned();
    let args = Args::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    args::route(args, &command).await
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        if is_broken_pipe(&error) {
            return;
        }
        let label = "Error:".if_supports_color(Stream::Stderr, |label| label.style(Style::new().red().bold()));
        eprintln!("{label} {error:?}");
        std::process::exit(1);
    }
}

/// The reader of stdout went away, e.g. `yokoku root list | head`.
fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<io::Error>())
        .any(|error| error.kind() == io::ErrorKind::BrokenPipe)
}
