#![forbid(unsafe_code)]

mod args;
mod commands;
mod config;
mod logging;

use anyhow::Result;
use clap::Parser;

use crate::args::Args;

fn run() -> Result<()> {
    let args = Args::parse();
    args::route(args)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {:?}", error);
        std::process::exit(1);
    }
}
