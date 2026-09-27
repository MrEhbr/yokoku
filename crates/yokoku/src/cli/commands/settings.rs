use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;
use yokoku_config::Config;
use yokoku_db::Database;
use yokoku_domain::SettingsStore;
use yokoku_events::{Publisher, SettingsChanged};
use yokoku_system::FileSpool;

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

/// `config` comes from the config file and environment alone, so a stored value that no longer
/// loads can still be unset. A change publishes `SettingsChanged`, which a running service reloads on.
pub async fn run(config: &Config, config_path: Option<&Path>, args: Args) -> Result<()> {
    let path = &config.database.path;
    let db = Database::open(path).await.with_context(|| format!("Failed to open database: {}", path.display()))?;
    let stored = db.settings().await.context("Failed to read the stored settings")?;
    let events = Publisher::new(Arc::new(db.event_log()), Arc::new(FileSpool::new(path.with_extension("spool"))));

    match args.command {
        Command::List => {
            if stored.is_empty() {
                hint!("No stored settings.")?;
            }
            for (key, value) in &stored {
                say!("{key} = {}", Config::shown(key, value))?;
            }
        },
        Command::Get { key } => {
            let effective = Config::load(config_path, &stored).context("Failed to load configuration")?;
            say!("{}", effective.setting(&key)?)?;
        },
        Command::Set { key, value } => {
            Config::editable(&key)?;
            let value = serde_json::from_str(&value).unwrap_or(Value::String(value));
            let mut candidate: Vec<(String, Value)> = stored.into_iter().filter(|(stored, _)| *stored != key).collect();
            candidate.push((key.clone(), value.clone()));
            let effective =
                Config::load(config_path, &candidate).with_context(|| format!("{key} cannot be {value}"))?;
            effective.validate().with_context(|| format!("{key} cannot be {value}"))?;

            db.set_setting(&key, &value).await.context("Failed to store the setting")?;
            events.publish(SettingsChanged { key: key.clone() }).await;
            success!("Set {key} = {}", Config::shown(&key, &value))?;
            let variable = format!("APP__{}", key.to_uppercase().replace('.', "__"));
            if std::env::var_os(&variable).is_some() {
                say!("{variable} is set and takes precedence")?;
            }
        },
        Command::Unset { key } => match db.remove_setting(&key).await.context("Failed to remove the setting")? {
            true => {
                events.publish(SettingsChanged { key: key.clone() }).await;
                success!("Unset {key}")?;
            },
            false => say!("{key} is not stored")?,
        },
    }
    Ok(())
}
