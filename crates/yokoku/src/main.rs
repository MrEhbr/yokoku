#![forbid(unsafe_code)]

mod app;
mod args;
mod commands;
mod config;
mod logging;

use std::io;

use anyhow::Result;
use clap::Parser;

use crate::args::Args;

async fn run() -> Result<()> {
    let args = Args::parse();
    args::route(args).await
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
