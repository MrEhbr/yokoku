mod common;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use common::{App, now};
use yokoku_domain::{ItemId, SubtitleTags};
use yokoku_events::{EventId, EventLog, Recorded, Subscriber};
use yokoku_media::{
    MediaError, MediaInfo, Prober, VideoStream,
    ports::{MediaProbe, ProbeError},
};
use yokoku_system::LocalFileSystem;

const DUNE: &str = "movies/Dune (2021)/Dune (2021).mkv";

/// Answers every path with a 1080p video, except paths it is told fail; `missing` acts as if
/// ffprobe were not installed.
#[derive(Default)]
struct ScriptedProbe {
    failing: Mutex<HashSet<PathBuf>>,
    missing: Mutex<bool>,
}

fn full_hd() -> MediaInfo {
    MediaInfo { video: Some(VideoStream { codec: "h264".into(), width: 1920, height: 1080 }), ..MediaInfo::default() }
}

#[async_trait]
impl MediaProbe for ScriptedProbe {
    async fn probe(&self, path: &Path) -> Result<MediaInfo, ProbeError> {
        if *self.missing.lock().unwrap() {
            return Err(ProbeError::Missing);
        }
        match self.failing.lock().unwrap().contains(path) {
            true => Err(ProbeError::Failed { path: path.to_owned(), reason: "Invalid data".into() }),
            false => Ok(full_hd()),
        }
    }
}

struct Setup {
    app: App,
    probe: Arc<ScriptedProbe>,
    prober: Prober,
}

async fn setup() -> Setup {
    let app = App::new().await;
    let probe = Arc::new(ScriptedProbe::default());
    let prober = Prober::new(Arc::new(app.db.clone()), Arc::new(LocalFileSystem), probe.clone());
    Setup { app, probe, prober }
}

impl Setup {
    /// Scans, then hands every recorded event to the prober.
    async fn scan_and_deliver(&self) {
        self.app.scanner.scan().await.unwrap();
        for recorded in self.app.db.event_log().read_after(None, 100).await.unwrap() {
            self.prober.handle(&recorded).await.unwrap();
        }
    }

    fn dune(&self) -> ItemId {
        ItemId::Movie(self.app.dune.id)
    }
}

#[tokio::test]
async fn found_files_are_probed_and_shown_with_their_subtitle_files() {
    let setup = setup().await;
    setup.app.write(DUNE, 10);
    setup.app.write("movies/Dune (2021)/Dune (2021).en.forced.srt", 1);

    setup.scan_and_deliver().await;
    let details = setup.prober.details(setup.dune()).await.unwrap();

    assert_eq!(details.len(), 1);
    assert_eq!(details[0].file.path, setup.app.path(DUNE));
    assert_eq!(details[0].info, Some(full_hd()));
    assert_eq!(details[0].subtitle_files, [SubtitleTags { language: Some("en".into()), sdh: false, forced: true }]);
    assert!(setup.prober.details(ItemId::Series(setup.app.frieren.id)).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_file_that_cannot_be_probed_is_tried_again_later() {
    let setup = setup().await;
    let path = setup.app.write(DUNE, 10);
    setup.probe.failing.lock().unwrap().insert(path.clone());

    setup.scan_and_deliver().await;
    let unknown = setup.prober.details(setup.dune()).await.unwrap()[0].info.clone();
    let failed = setup.prober.probe_missing().await.unwrap();
    setup.probe.failing.lock().unwrap().clear();
    let probed = setup.prober.probe_missing().await.unwrap();

    assert_eq!(unknown, None);
    let reason = format!("cannot probe {}: Invalid data", path.display());
    assert_eq!(failed.failed, [(path, reason)]);
    assert_eq!((probed.probed, probed.failed.len()), (1, 0));
    assert_eq!(setup.prober.details(setup.dune()).await.unwrap()[0].info, Some(full_hd()));
}

#[tokio::test]
async fn without_a_probe_new_files_stay_unknown_and_probing_fails() {
    let setup = setup().await;
    setup.app.write(DUNE, 10);
    *setup.probe.missing.lock().unwrap() = true;

    setup.scan_and_deliver().await;
    let error = setup.prober.probe_missing().await.unwrap_err();

    assert_eq!(setup.prober.details(setup.dune()).await.unwrap()[0].info, None);
    assert!(matches!(error, MediaError::Probe(ProbeError::Missing)), "{error}");
}

#[tokio::test]
async fn other_events_are_ignored() {
    let setup = setup().await;
    let recorded = Recorded {
        id: EventId(1),
        occurred_at: now(),
        event: yokoku_events::Event::MovieAdded { movie: setup.app.dune.id, title: "Dune".into() },
    };

    setup.prober.handle(&recorded).await.unwrap();

    assert_eq!(setup.prober.probe_missing().await.unwrap().probed, 0);
}
