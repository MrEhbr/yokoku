use common::App;
use rstest::rstest;
use yokoku_core::media::{
    Approval, Episodes, Import, ImportRow, ImportStatus, MediaError, Problem, Resolution, RowMatch,
    detect::Conflict,
    ports::{Changes, MediaRepo},
};
use yokoku_domain::{
    Confidence, DownloadId, EpisodeRef, EpisodeSpan, FileTarget, ImportId, ItemId, MovieId, SeriesId,
    events::{FilesImported, LinkedFile},
};

use crate::common;

/// Scans three unrecognised files in Frieren's folder and returns their import.
async fn pending(app: &App) -> ImportId {
    for name in ["a", "b", "c"] {
        app.write(&format!("tv/Frieren (2023)/{name}.mkv"), 10);
    }
    let report = app.scanner.scan().await.unwrap();
    report.needs_review[0]
}

/// Saves a download's import waiting for review, one checked row per name and match.
async fn download(app: &App, folder: &str, rows: &[(&str, RowMatch)]) -> ImportId {
    let import = Import {
        id: ImportId::generate(),
        source: app.path(&format!("downloads/{folder}")),
        download: Some(DownloadId::generate()),
        status: ImportStatus::NeedsReview,
        error: None,
        rows: rows
            .iter()
            .map(|&(name, matched)| ImportRow {
                path: app.path(&format!("downloads/{folder}/{name}")),
                size: 10,
                matched,
                confidence: Confidence::Unknown,
                skipped: false,
                resolution: Resolution::Unresolved,
            })
            .collect(),
        created_at: common::now(),
    };
    MediaRepo::save(&app.db, &Changes { imports: vec![import.clone()], ..Changes::default() }).await.unwrap();
    import.id
}

fn frieren(app: &App, season: Option<u16>, episodes: Option<(u16, u16)>) -> RowMatch {
    RowMatch::Series {
        series: app.frieren.id,
        season,
        episodes: episodes.map(|(first, last)| Episodes { first, last }),
    }
}

async fn targets(app: &App, id: ImportId) -> Vec<Option<FileTarget>> {
    app.reviewer.get(id).await.unwrap().rows.iter().map(|row| row.row.target()).collect()
}

#[tokio::test]
async fn approving_links_matched_rows_and_leaves_unchecked_ones() {
    let app = App::new().await;
    let id = pending(&app).await;
    app.reviewer.match_row(id, 1, app.episodes(1, 1, 2)).await.unwrap();
    app.reviewer.match_row(id, 2, app.movie()).await.unwrap();

    let Approval::Linked(files) = app.reviewer.approve(id).await.unwrap() else { panic!("scan imports are linked") };

    assert_eq!(
        files.iter().map(|file| (file.path.clone(), file.target)).collect::<Vec<_>>(),
        [
            (app.path("tv/Frieren (2023)/a.mkv"), app.episodes(1, 1, 2)),
            (app.path("tv/Frieren (2023)/b.mkv"), app.movie()),
        ]
    );
    assert_eq!(app.db_files().await.len(), 2);
    let linked = files.iter().map(|file| LinkedFile { file: file.id, path: file.path.clone(), target: file.target });
    assert_eq!(
        app.events().await.last(),
        Some(&FilesImported { import: id, download: None, files: linked.collect(), sources: Vec::new() }.into())
    );
    assert!(app.reviewer.pending().await.unwrap().is_empty());
}

#[tokio::test]
async fn approved_and_unchecked_files_are_not_reviewed_again() {
    let app = App::new().await;
    let id = pending(&app).await;
    app.reviewer.match_row(id, 1, app.episodes(1, 1, 1)).await.unwrap();
    app.reviewer.approve(id).await.unwrap();

    let report = app.scanner.scan().await.unwrap();

    assert!(report.needs_review.is_empty());
}

#[tokio::test]
async fn unmatched_files_start_unchecked_keeping_the_series_of_their_folder() {
    let app = App::new().await;
    let id = pending(&app).await;

    let rows = app.reviewer.get(id).await.unwrap().rows;

    let unmatched = |row: &yokoku_core::media::ReviewRow| {
        row.row.skipped && row.row.matched == frieren(&app, None, None) && row.problem == Some(Problem::NoSeason)
    };
    assert!(rows.iter().all(unmatched), "{rows:?}");
}

#[tokio::test]
async fn every_checked_row_needs_a_match() {
    let app = App::new().await;
    let id = pending(&app).await;
    app.reviewer.include_rows(id, &[1, 3], true).await.unwrap();
    app.reviewer.match_row(id, 2, app.movie()).await.unwrap();

    let error = app.reviewer.approve(id).await.unwrap_err();

    assert!(matches!(error, MediaError::UnmatchedRows(ref rows) if rows == &[1, 3]), "{error}");
    assert!(app.db_files().await.is_empty());
}

#[tokio::test]
async fn rows_sharing_an_episode_or_holding_a_linked_one_conflict() {
    let app = App::new().await;
    app.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E03.mkv", 10);
    let id = pending(&app).await;
    app.reviewer.match_row(id, 1, app.episodes(1, 1, 2)).await.unwrap();
    app.reviewer.match_row(id, 2, app.episodes(1, 2, 2)).await.unwrap();
    app.reviewer.match_row(id, 3, app.episodes(1, 3, 3)).await.unwrap();

    let review = app.reviewer.get(id).await.unwrap();
    let error = app.reviewer.approve(id).await.unwrap_err();

    let conflicts: Vec<_> = review.rows.iter().map(|row| row.conflicts.clone()).collect();
    assert_eq!(conflicts, [vec![Conflict::SharedTarget], vec![Conflict::SharedTarget], vec![Conflict::AlreadyHasFile]]);
    assert!(matches!(error, MediaError::ConflictingRows(ref rows) if rows == &[1, 2, 3]), "{error}");
}

#[tokio::test]
async fn a_row_kept_beside_the_others_neither_has_nor_causes_conflicts() {
    let app = App::new().await;
    app.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E03.mkv", 10);
    let id = pending(&app).await;
    app.reviewer.match_row(id, 1, app.episodes(1, 2, 2)).await.unwrap();
    app.reviewer.match_row(id, 2, app.episodes(1, 2, 2)).await.unwrap();
    app.reviewer.match_row(id, 3, app.episodes(1, 3, 3)).await.unwrap();

    app.reviewer.keep_both_row(id, 2).await.unwrap();
    app.reviewer.keep_both_row(id, 3).await.unwrap();

    let review = app.reviewer.get(id).await.unwrap();
    assert!(review.rows.iter().all(|row| row.conflicts.is_empty()), "{review:?}");
    assert_eq!(review.rows[2].row.resolution, Resolution::KeepBoth);
    app.reviewer.match_row(id, 3, app.episodes(1, 3, 3)).await.unwrap();
    assert_eq!(app.reviewer.get(id).await.unwrap().rows[2].conflicts, [Conflict::AlreadyHasFile]);
}

#[tokio::test]
async fn a_scanned_file_kept_beside_the_library_file_is_linked_where_it_is() {
    let app = App::new().await;
    app.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E03.mkv", 10);
    let id = pending(&app).await;
    app.reviewer.match_row(id, 1, app.episodes(1, 3, 3)).await.unwrap();
    app.reviewer.keep_both_row(id, 1).await.unwrap();

    let Approval::Linked(files) = app.reviewer.approve(id).await.unwrap() else { panic!("scan imports are linked") };

    assert_eq!(files.iter().map(|file| file.path.clone()).collect::<Vec<_>>(), [app.path("tv/Frieren (2023)/a.mkv")]);
    let episode = app.db_files().await.into_iter().filter(|file| file.target == app.episodes(1, 3, 3)).count();
    assert_eq!(episode, 2);
}

#[tokio::test]
async fn unchecking_a_row_clears_its_conflicts() {
    let app = App::new().await;
    let id = pending(&app).await;
    app.reviewer.match_row(id, 1, app.movie()).await.unwrap();
    app.reviewer.match_row(id, 2, app.movie()).await.unwrap();
    app.reviewer.include_rows(id, &[2], false).await.unwrap();

    let review = app.reviewer.get(id).await.unwrap();

    assert!(review.rows.iter().all(|row| row.conflicts.is_empty()));
    app.reviewer.approve(id).await.unwrap();
}

#[rstest]
#[case::missing_episode(|app: &App| app.episodes(1, 3, 4), "S01E03-E04 is not in the series")]
#[case::missing_series(|_: &App| FileTarget::Episodes { series: SeriesId::generate(), span: EpisodeSpan::new(1, 1, 1).unwrap() }, "is not in the library")]
#[case::missing_movie(|_: &App| FileTarget::Movie(MovieId::generate()), "is not in the library")]
#[tokio::test]
async fn matches_must_be_in_the_library(#[case] target: fn(&App) -> FileTarget, #[case] message: &str) {
    let app = App::new().await;
    let id = pending(&app).await;

    let error = app.reviewer.match_row(id, 1, target(&app)).await.unwrap_err();

    assert!(error.to_string().contains(message), "{error}");
    assert_eq!(app.reviewer.get(id).await.unwrap().rows[0].row.target(), None);
}

#[rstest]
#[case(0)]
#[case(4)]
#[tokio::test]
async fn rows_are_numbered_from_one(#[case] row: usize) {
    let app = App::new().await;
    let id = pending(&app).await;

    let error = app.reviewer.include_rows(id, &[1, row], true).await.unwrap_err();

    assert!(matches!(error, MediaError::RowNotFound(number) if number == row), "{error}");
    assert!(app.reviewer.get(id).await.unwrap().rows[0].row.skipped, "nothing changes");
}

#[tokio::test]
async fn done_and_unknown_imports_cannot_be_reviewed() {
    let app = App::new().await;
    let id = pending(&app).await;
    app.reviewer.approve(id).await.unwrap();

    let done = app.reviewer.approve(id).await.unwrap_err();
    let unknown = app.reviewer.get(ImportId::generate()).await.unwrap_err();

    assert!(matches!(done, MediaError::NotInReview(_)), "{done}");
    assert!(matches!(unknown, MediaError::ImportNotFound(_)), "{unknown}");
    let stored = MediaRepo::import(&app.db, id).await.unwrap().unwrap();
    assert_eq!(stored.status, ImportStatus::Done);
}

#[rstest]
#[case::nothing(|_: &App| RowMatch::None, Problem::NoMatch)]
#[case::no_season(|app: &App| frieren(app, None, Some((1, 1))), Problem::NoSeason)]
#[case::no_episodes(|app: &App| frieren(app, Some(1), None), Problem::NoEpisodes)]
#[case::season_not_in_series(|app: &App| frieren(app, Some(13), Some((1, 1))), Problem::SeasonNotInSeries(13))]
#[case::episodes_not_in_series(|app: &App| frieren(app, Some(1), Some((3, 4))), Problem::EpisodesNotInSeries(EpisodeSpan::new(1, 3, 4).unwrap()))]
#[case::series_gone(|_: &App| RowMatch::Series { series: SeriesId::generate(), season: Some(1), episodes: None }, Problem::Gone)]
#[case::movie_gone(|_: &App| RowMatch::Movie(MovieId::generate()), Problem::Gone)]
#[tokio::test]
async fn a_row_names_what_its_match_lacks(#[case] matched: fn(&App) -> RowMatch, #[case] problem: Problem) {
    let app = App::new().await;
    let id = download(&app, "Frieren", &[("Frieren.mkv", matched(&app))]).await;

    let review = app.reviewer.get(id).await.unwrap();
    let error = app.reviewer.approve(id).await.unwrap_err();

    assert_eq!(review.rows[0].problem, Some(problem));
    assert!(matches!(error, MediaError::UnmatchedRows(ref rows) if rows == &[1]), "{error}");
}

#[tokio::test]
async fn a_season_set_on_rows_keeps_their_episode_numbers() {
    let app = App::new().await;
    let pack = [
        ("Frieren.S13E01.mkv", frieren(&app, Some(13), Some((1, 1)))),
        ("Frieren.S13E02.mkv", frieren(&app, Some(13), Some((2, 2)))),
        ("Frieren.S13E03.mkv", frieren(&app, Some(13), Some((3, 3)))),
    ];
    let id = download(&app, "Frieren.S13", &pack).await;

    app.reviewer.set_season(id, &[1, 2], 2).await.unwrap();

    let review = app.reviewer.get(id).await.unwrap();
    let problems: Vec<_> = review.rows.iter().map(|row| row.problem).collect();
    assert_eq!(problems, [None, None, Some(Problem::SeasonNotInSeries(13))]);
    assert_eq!(targets(&app, id).await[..2], [Some(app.episodes(2, 1, 1)), Some(app.episodes(2, 2, 2))]);
}

#[tokio::test]
async fn episodes_chosen_for_rows_are_given_in_order_one_each() {
    let app = App::new().await;
    let part = [
        ("Frieren.S01.Part.2.E01.mkv", frieren(&app, Some(1), Some((1, 1)))),
        ("Frieren.S01.Part.2.E02.mkv", frieren(&app, Some(1), Some((2, 2)))),
        ("extra.mkv", RowMatch::None),
    ];
    let id = download(&app, "Frieren.S01.Part.2", &part).await;
    let chosen = [EpisodeRef { season: 1, episode: 3 }, EpisodeRef { season: 2, episode: 1 }];

    app.reviewer.set_episodes(id, &[1, 2], app.frieren.id, &chosen).await.unwrap();

    assert_eq!(targets(&app, id).await, [Some(app.episodes(1, 3, 3)), Some(app.episodes(2, 1, 1)), None]);
}

#[rstest]
#[case::too_few(&[(1, 3)], "1 episodes for 2 files")]
#[case::not_in_series(&[(1, 3), (1, 4)], "S01E04 is not in the series")]
#[tokio::test]
async fn episodes_chosen_for_rows_must_be_one_each_in_the_series(#[case] chosen: &[(u16, u16)], #[case] message: &str) {
    let app = App::new().await;
    let id = download(&app, "Frieren", &[("a.mkv", RowMatch::None), ("b.mkv", RowMatch::None)]).await;
    let chosen: Vec<_> = chosen.iter().map(|&(season, episode)| EpisodeRef { season, episode }).collect();

    let error = app.reviewer.set_episodes(id, &[1, 2], app.frieren.id, &chosen).await.unwrap_err();

    assert!(error.to_string().contains(message), "{error}");
    assert_eq!(targets(&app, id).await, [None, None], "nothing changes");
}

#[tokio::test]
async fn a_season_needs_a_series_first() {
    let app = App::new().await;
    let rows = [("a.mkv", frieren(&app, Some(13), Some((1, 1)))), ("b.mkv", RowMatch::None)];
    let id = download(&app, "Mixed", &rows).await;

    let season = app.reviewer.set_season(id, &[1, 2], 1).await.unwrap_err();

    assert!(matches!(season, MediaError::RowsWithoutSeries(ref rows) if rows == &[2]), "{season}");
    assert_eq!(app.reviewer.get(id).await.unwrap().rows[0].row.matched, rows[0].1, "nothing changes");
}

#[tokio::test]
async fn setting_the_series_detects_the_rows_again_and_checks_them() {
    let app = App::new().await;
    let rows =
        [("Frieren - 01.mkv", RowMatch::None), ("Frieren.S13E02.mkv", RowMatch::None), ("extra.mkv", RowMatch::None)];
    let id = download(&app, "Frieren Pack", &rows).await;
    app.reviewer.include_rows(id, &[1, 2, 3], false).await.unwrap();

    app.reviewer.set_series(id, &[1, 2], app.frieren.id).await.unwrap();

    let review = app.reviewer.get(id).await.unwrap();
    let rows: Vec<_> = review.rows.iter().map(|row| (row.row.matched, row.row.confidence, row.row.skipped)).collect();
    assert_eq!(
        rows,
        [
            (app.episodes(1, 1, 1).into(), Confidence::Guess, false),
            (frieren(&app, Some(13), Some((2, 2))), Confidence::Unknown, false),
            (RowMatch::None, Confidence::Unknown, true),
        ]
    );
}

#[tokio::test]
async fn a_planned_pack_whose_season_the_series_lacks_keeps_the_series() {
    let app = App::new().await;
    app.write("downloads/Frieren.S13.1080p/Frieren.S13E01.1080p.mkv", 10);
    app.write("downloads/Frieren.S13.1080p/Frieren.S13E02.1080p.mkv", 10);
    let item = Some(ItemId::Series(app.frieren.id));

    let import = app
        .planner
        .plan(DownloadId::generate(), &app.path("downloads/Frieren.S13.1080p"), item, None)
        .await
        .unwrap()
        .unwrap();
    app.reviewer.set_season(import.id, &[1, 2], 2).await.unwrap();

    assert_eq!(import.status, ImportStatus::NeedsReview);
    let planned: Vec<_> = import.rows.iter().map(|row| (row.matched, row.skipped)).collect();
    assert_eq!(planned, [(frieren(&app, Some(13), Some((1, 1))), true), (frieren(&app, Some(13), Some((2, 2))), true)]);
    assert_eq!(app.reviewer.approve(import.id).await.unwrap(), Approval::Queued);
}

#[tokio::test]
async fn approving_a_download_queues_it_for_placing() {
    let app = App::new().await;
    let id = download(&app, "Frieren.S01E01", &[("Frieren.S01E01.mkv", app.episodes(1, 1, 1).into())]).await;

    let approval = app.reviewer.approve(id).await.unwrap();

    assert_eq!(approval, Approval::Queued);
    let stored = MediaRepo::import(&app.db, id).await.unwrap().unwrap();
    assert_eq!(stored.status, ImportStatus::Approved);
    assert!(app.db_files().await.is_empty());
    assert!(app.events().await.is_empty());
}

#[tokio::test]
async fn a_download_row_can_replace_the_library_file() {
    let app = App::new().await;
    app.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.mkv", 10);
    app.scanner.scan().await.unwrap();
    let id = download(&app, "Frieren.S01E01", &[("Frieren.S01E01.mkv", app.episodes(1, 1, 1).into())]).await;
    let conflicts = |review: yokoku_core::media::ImportReview| review.rows[0].conflicts.clone();
    assert_eq!(conflicts(app.reviewer.get(id).await.unwrap()), [Conflict::AlreadyHasFile]);

    app.reviewer.replace_row(id, 1).await.unwrap();

    let review = app.reviewer.get(id).await.unwrap();
    assert_eq!(review.rows[0].row.resolution, Resolution::Replace);
    assert_eq!(conflicts(review), []);
    assert_eq!(app.reviewer.approve(id).await.unwrap(), Approval::Queued);
}

#[tokio::test]
async fn files_found_by_a_scan_cannot_replace_library_files() {
    let app = App::new().await;
    let id = pending(&app).await;

    let error = app.reviewer.replace_row(id, 1).await.unwrap_err();

    assert!(matches!(error, MediaError::ReplaceInPlace), "{error}");
}

#[tokio::test]
async fn every_review_change_is_announced() {
    let app = App::new().await;
    let id = pending(&app).await;
    let mut watch = app.changes.watch();
    let mut announced = Vec::new();

    app.reviewer.match_row(id, 1, app.episodes(1, 1, 1)).await.unwrap();
    announced.push(watch.has_changed().unwrap());
    watch.borrow_and_update();
    app.reviewer.include_rows(id, &[2], true).await.unwrap();
    announced.push(watch.has_changed().unwrap());
    watch.borrow_and_update();
    app.reviewer.include_rows(id, &[2], false).await.unwrap();
    watch.borrow_and_update();
    app.reviewer.approve(id).await.unwrap();
    announced.push(watch.has_changed().unwrap());

    assert_eq!(announced, [true, true, true]);
}
