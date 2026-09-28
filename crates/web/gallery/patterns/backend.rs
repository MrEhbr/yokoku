//! Server functions against in-memory demo state.

use dioxus::{fullstack::ServerEvents, prelude::*};
use serde::{Deserialize, Serialize};

/// A library series as the import dialog needs it: episode titles per season, season 1 first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeriesInfo {
    pub id: u64,
    pub title: String,
    pub year: Option<i16>,
    pub seasons: Vec<Vec<String>>,
}

/// A file waiting for import, with whatever the parser matched.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileRow {
    pub id: u64,
    pub path: String,
    pub series: Option<u64>,
    pub season: Option<u16>,
    pub episodes: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Match {
    pub file: u64,
    pub series: u64,
    pub season: u16,
    pub episodes: Vec<u16>,
}

#[get("/api/library")]
pub async fn library() -> Result<Vec<SeriesInfo>, ServerFnError> {
    Ok(state::library())
}

#[get("/api/pending")]
pub async fn pending_files() -> Result<Vec<FileRow>, ServerFnError> {
    Ok(state::PENDING.lock().unwrap().clone())
}

/// Imports the matched files; refuses the whole batch if one match is invalid or two claim one episode.
#[post("/api/import")]
pub async fn import_files(matches: Vec<Match>) -> Result<usize, ServerFnError> {
    let library = state::library();
    let mut claimed = std::collections::HashSet::new();
    for m in &matches {
        let series = library.iter().find(|series| series.id == m.series);
        let episodes = series.and_then(|series| series.seasons.get(usize::from(m.season).checked_sub(1)?));
        let known =
            episodes.is_some_and(|titles| m.episodes.iter().all(|e| (1..=titles.len()).contains(&usize::from(*e))));
        let unique = m.episodes.iter().all(|e| claimed.insert((m.series, m.season, *e)));
        if !known || !unique || m.episodes.is_empty() {
            return Err(ServerFnError::new("the import plan has invalid rows"));
        }
    }
    let mut pending = state::PENDING.lock().unwrap();
    let before = pending.len();
    pending.retain(|row| !matches.iter().any(|m| m.file == row.id));
    Ok(before - pending.len())
}

#[post("/api/reset")]
pub async fn reset_demo() -> Result<(), ServerFnError> {
    *state::PENDING.lock().unwrap() = state::demo_files();
    Ok(())
}

/// Runs a fake job that publishes 0..=100 in steps of 5 over three seconds.
#[post("/api/job")]
pub async fn start_job() -> Result<(), ServerFnError> {
    tokio::spawn(async {
        for percent in (0..=100).step_by(5) {
            state::PROGRESS.send_replace(Some(percent));
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
    });
    Ok(())
}

/// The job's progress: the current value at once, then every change.
#[get("/api/progress")]
pub async fn progress() -> Result<ServerEvents<Option<u32>>, ServerFnError> {
    let mut updates = state::PROGRESS.subscribe();
    Ok(ServerEvents::new(move |mut tx| async move {
        loop {
            let percent = *updates.borrow_and_update();
            if tx.send(percent).await.is_err() || updates.changed().await.is_err() {
                break;
            }
        }
    }))
}

#[cfg(feature = "server")]
mod state {
    use std::sync::{LazyLock, Mutex};

    use super::{FileRow, SeriesInfo};

    pub static PENDING: LazyLock<Mutex<Vec<FileRow>>> = LazyLock::new(|| Mutex::new(demo_files()));
    pub static PROGRESS: LazyLock<tokio::sync::watch::Sender<Option<u32>>> =
        LazyLock::new(|| tokio::sync::watch::channel(None).0);

    pub fn library() -> Vec<SeriesInfo> {
        let numbered = |count: u16| (1..=count).map(|n| format!("Episode {n}")).collect::<Vec<_>>();
        let orbital =
            ["Launch", "Drift", "Burn", "Coast", "Descent", "Landfall", "Survey", "Storm", "Ascent", "Return"];
        vec![
            SeriesInfo {
                id: 1,
                title: "Orbital".into(),
                year: Some(2024),
                seasons: vec![orbital.map(str::to_owned).to_vec(), numbered(8)],
            },
            SeriesInfo { id: 2, title: "Harbor Lights".into(), year: Some(2019), seasons: vec![numbered(6); 3] },
            SeriesInfo { id: 3, title: "The Quiet Engine".into(), year: Some(2022), seasons: vec![numbered(8)] },
        ]
    }

    pub fn demo_files() -> Vec<FileRow> {
        let row = |id, path: &str, series, season, episodes: &[u16]| FileRow {
            id,
            path: path.into(),
            series,
            season,
            episodes: episodes.to_vec(),
        };
        vec![
            row(1, "Orbital.S01E03.1080p.WEB.mkv", Some(1), Some(1), &[3]),
            row(2, "Orbital.S01E04.1080p.WEB.mkv", Some(1), Some(1), &[4]),
            row(3, "orbital.2x01.mkv", Some(1), Some(2), &[]),
            row(4, "Harbor.Lights.S02E05.720p.mkv", Some(2), Some(2), &[5]),
            row(5, "Harbor.Lights.S02E06.720p.mkv", Some(2), Some(2), &[6]),
            row(6, "quiet_engine_1.mkv", None, None, &[]),
            row(7, "quiet_engine_2.mkv", None, None, &[]),
            row(8, "quiet_engine_10.mkv", None, None, &[]),
            row(9, "Quiet.Engine.S01E04E05.mkv", Some(3), Some(1), &[4, 5]),
            row(10, "sample.mkv", None, None, &[]),
        ]
    }
}
