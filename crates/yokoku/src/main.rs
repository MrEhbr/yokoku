#![forbid(unsafe_code)]

mod app;
mod args;
mod commands;
mod config;
mod logging;

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
        eprintln!("Error: {:?}", error);
        std::process::exit(1);
    }
}
