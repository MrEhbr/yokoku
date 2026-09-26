use std::{
    io::{self, Write},
    path::Path,
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde_json::Value;
use yokoku_db::Database;

use crate::config::{self, Config};

/// Needed before the database is open, so they stay in the config file and environment.
const BOOTSTRAP: [&str; 2] = ["database", "log"];
/// Read only from `APP__*` variables.
const SECRETS: [&str; 3] = ["metadata.tmdb_token", "transmission.password", "jellyfin.api_key"];

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List the settings stored in the database
    List,
    /// Show the value in effect, e.g. `import.mode`
    Get { key: String },
    /// Store a value, e.g. `import.mode copy`; it overrides the config file until unset
    Set { key: String, value: String },
    /// Remove a stored value
    Unset { key: String },
}

/// Settings stored in the database at `path`; none while it does not exist.
pub async fn stored(path: &Path) -> Result<Vec<(String, Value)>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let db = Database::open(path).await.with_context(|| format!("Failed to open database: {}", path.display()))?;
    db.settings().await.context("Failed to read the stored settings")
}

/// `config` comes from the config file and environment alone, so a stored value that no longer
/// loads can still be unset.
pub async fn run(config: &Config, config_path: Option<&Path>, args: Args) -> Result<()> {
    let path = &config.database.path;
    let db = Database::open(path).await.with_context(|| format!("Failed to open database: {}", path.display()))?;
    let stored = db.settings().await.context("Failed to read the stored settings")?;
    let mut out = io::stdout();

    match args.command {
        Command::List => {
            if stored.is_empty() {
                writeln!(out, "No stored settings.")?;
            }
            for (key, value) in &stored {
                writeln!(out, "{key} = {value}")?;
            }
        },
        Command::Get { key } => {
            let effective: Config = config::load(config_path, &stored).context("Failed to load configuration")?;
            let effective = serde_json::to_value(&effective)?;
            let value = setting(&effective, &key)?;
            match SECRETS.contains(&key.as_str()) {
                true if value.is_null() => writeln!(out, "not set")?,
                true => writeln!(out, "<redacted>")?,
                false => writeln!(out, "{value}")?,
            }
        },
        Command::Set { key, value } => {
            editable(&key)?;
            let value = serde_json::from_str(&value).unwrap_or(Value::String(value));
            let mut candidate: Vec<(String, Value)> = stored.into_iter().filter(|(stored, _)| *stored != key).collect();
            candidate.push((key.clone(), value.clone()));
            let effective: Config =
                config::load(config_path, &candidate).with_context(|| format!("{key} cannot be {value}"))?;
            effective.validate().with_context(|| format!("{key} cannot be {value}"))?;

            db.set_setting(&key, &value).await.context("Failed to store the setting")?;
            writeln!(out, "Set {key} = {value}")?;
            let variable = format!("APP__{}", key.to_uppercase().replace('.', "__"));
            if std::env::var_os(&variable).is_some() {
                writeln!(out, "{variable} is set and takes precedence")?;
            }
        },
        Command::Unset { key } => match db.remove_setting(&key).await.context("Failed to remove the setting")? {
            true => writeln!(out, "Unset {key}")?,
            false => writeln!(out, "{key} is not stored")?,
        },
    }
    Ok(())
}

/// The value of a known setting.
fn setting<'a>(config: &'a Value, key: &str) -> Result<&'a Value> {
    let value = key.split('.').try_fold(config, |value, part| value.get(part));
    match value {
        Some(value) if !value.is_object() => Ok(value),
        _ => bail!("{key} is not a setting"),
    }
}

fn editable(key: &str) -> Result<()> {
    setting(&serde_json::to_value(Config::default())?, key)?;
    if BOOTSTRAP.iter().any(|section| key.split('.').next() == Some(section)) {
        bail!("{key} is needed before the database opens; set it in the config file or environment");
    }
    if SECRETS.contains(&key) {
        bail!("{key} is a secret; set it through APP__{}", key.to_uppercase().replace('.', "__"));
    }
    Ok(())
}
