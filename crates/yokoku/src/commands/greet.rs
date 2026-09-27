//! Example command. To add your own:
//!
//! 1. Copy this file to `src/commands/<name>.rs`.
//! 2. Register it in `src/commands/mod.rs`.
//! 3. Add a variant to `Command` in `src/args.rs` and dispatch it in `route`.
//! 4. Add its config section to `Config` in `src/config.rs`.

use anyhow::{Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::config::Config;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GreetConfig {
    pub greeting: String,
    pub count: u8,
}

impl Default for GreetConfig {
    fn default() -> Self {
        Self { greeting: "Hello".to_string(), count: 1 }
    }
}

#[derive(Parser)]
pub struct Args {
    /// Name to greet
    pub name: String,

    /// Greeting word, overriding `greet.greeting`
    #[arg(long, short = 'g')]
    pub greeting: Option<String>,

    /// Times to repeat, overriding `greet.count`
    #[arg(long, short = 'n')]
    pub count: Option<u8>,
}

impl Args {
    /// Command flags take precedence over the resolved configuration.
    fn apply_overrides(&self, config: &GreetConfig) -> GreetConfig {
        let mut resolved = config.clone();

        if let Some(greeting) = &self.greeting {
            resolved.greeting = greeting.clone();
        }
        if let Some(count) = self.count {
            resolved.count = count;
        }

        resolved
    }
}

pub fn run(config: &Config, args: Args) -> Result<()> {
    ensure!(!args.name.trim().is_empty(), "name must not be empty");

    let settings = args.apply_overrides(&config.greet);
    debug!(?settings, "resolved command settings");
    for _ in 0..settings.count {
        info!(name = %args.name, "greeting");
        say!("{}, {}!", settings.greeting, args.name)?;
    }

    Ok(())
}
