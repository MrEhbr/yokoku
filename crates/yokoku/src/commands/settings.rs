use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde_json::Value;
use yokoku_db::Database;

use crate::{
    config::{self, Config},
    secret::REDACTED,
};

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
            let effective: Config = config::load(config_path, &stored).context("Failed to load configuration")?;
            say!("{}", effective.setting(&key)?)?;
        },
        Command::Set { key, value } => {
            Config::editable(&key)?;
            let value = serde_json::from_str(&value).unwrap_or(Value::String(value));
            let mut candidate: Vec<(String, Value)> = stored.into_iter().filter(|(stored, _)| *stored != key).collect();
            candidate.push((key.clone(), value.clone()));
            let effective: Config =
                config::load(config_path, &candidate).with_context(|| format!("{key} cannot be {value}"))?;
            effective.validate().with_context(|| format!("{key} cannot be {value}"))?;

            db.set_setting(&key, &value).await.context("Failed to store the setting")?;
            success!("Set {key} = {}", Config::shown(&key, &value))?;
            let variable = format!("APP__{}", key.to_uppercase().replace('.', "__"));
            if std::env::var_os(&variable).is_some() {
                say!("{variable} is set and takes precedence")?;
            }
        },
        Command::Unset { key } => match db.remove_setting(&key).await.context("Failed to remove the setting")? {
            true => success!("Unset {key}")?,
            false => say!("{key} is not stored")?,
        },
    }
    Ok(())
}

impl Config {
    /// The value of a known setting, e.g. `import.mode`.
    fn setting(&self, key: &str) -> Result<Value> {
        let config = serde_json::to_value(self)?;
        match key.split('.').try_fold(&config, |value, part| value.get(part)) {
            Some(value) if !value.is_object() => Ok(value.clone()),
            _ => bail!("{key} is not a setting"),
        }
    }

    /// `value` as `setting` shows it once loaded, so a secret reads `"<redacted>"`; a value that does
    /// not load is shown as stored.
    fn shown(key: &str, value: &Value) -> Value {
        match config::load::<Self>(None, &[(key.to_owned(), value.clone())]).and_then(|config| config.setting(key)) {
            Ok(loaded) if loaded == REDACTED => loaded,
            _ => value.clone(),
        }
    }

    /// Fails unless `key` is a setting the database can store.
    fn editable(key: &str) -> Result<()> {
        Self::default().setting(key)?;
        if ["database", "log"].contains(&key.split('.').next().unwrap_or_default()) {
            bail!("{key} is needed before the database opens; set it in the config file or environment");
        }
        Ok(())
    }
}
