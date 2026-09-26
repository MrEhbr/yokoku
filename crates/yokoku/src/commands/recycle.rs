use std::{
    io::{self, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use yokoku_media::Recycle;

use crate::{app::App, config::Config};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RecycleConfig {
    /// Deleted files move here instead of being removed; unset deletes them.
    pub folder: Option<PathBuf>,
    /// Days a recycled file is kept.
    pub keep_days: u32,
}

impl Default for RecycleConfig {
    fn default() -> Self {
        Self { folder: None, keep_days: 30 }
    }
}

impl RecycleConfig {
    pub fn recycle(&self) -> Option<Recycle> {
        self.folder.clone().map(|folder| Recycle { folder, keep_days: self.keep_days })
    }
}

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Remove recycled files kept longer than `recycle.keep_days`
    Clean,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let mut out = io::stdout();
    match args.command {
        Command::Clean if config.recycle.folder.is_none() => writeln!(out, "No recycle folder is set.")?,
        Command::Clean => {
            let removed = app.deleter.clean_recycle().await?;
            writeln!(out, "Removed {removed} days of recycled files")?;
        },
    }
    Ok(())
}
