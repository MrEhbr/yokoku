use std::collections::BTreeMap;

use jiff::{
    Timestamp, ToSpan,
    civil::{Date, date},
};
use proptest::prelude::*;
use rstest::rstest;
use yokoku_domain::{
    EpisodeMetadata, EpisodeRef, EpisodeSpan, ExternalId, FileStatus, ItemFolder, MediaFileId, MonitorPreset,
    SeasonMetadata, Series, SeriesMetadata, SeriesStatus, SourceStatus, events::RenumberedFile,
};

const TODAY: Date = date(2026, 9, 26);
const YESTERDAY: Date = date(2026, 9, 25);
const TOMORROW: Date = date(2026, 9, 27);

fn now() -> Timestamp {
    Timestamp::UNIX_EPOCH
}

/// Seasons as `(number, air dates)`; episodes are numbered from 1 and get unique source ids.
fn metadata(status: SourceStatus, seasons: &[(u16, &[Option<Date>])]) -> SeriesMetadata {
    let mut next_source_id = 100;
    let seasons = seasons
        .iter()
        .map(|&(number, dates)| SeasonMetadata {
            number,
            episodes: dates
                .iter()
                .zip(1..)
                .map(|(&air_date, episode)| {
                    next_source_id += 1;
                    EpisodeMetadata {
                        source_id: next_source_id,
                        number: episode,
                        title: format!("S{number}E{episode}"),
                        air_date,
                    }
                })
                .collect(),
        })
        .collect();
    SeriesMetadata {
        source: ExternalId::Tmdb(1),
        title: "Frieren".into(),
        original_title: "Sousou no Frieren".into(),
        alternate_titles: Vec::new(),
        year: Some(2023),
        poster_path: None,
        status,
        seasons,
    }
}

fn regular_air_dates(series: &Series) -> impl Iterator<Item = Date> {
    series
        .seasons
        .iter()
        .filter(|season| season.number != 0)
        .flat_map(|season| &season.episodes)
        .filter_map(|e| e.air_date)
}

fn refs(pairs: &[(u16, u16)]) -> Vec<EpisodeRef> {
    pairs.iter().map(|&(season, episode)| EpisodeRef { season, episode }).collect()
}

fn monitored(series: &Series) -> Vec<EpisodeRef> {
    series.monitored_episodes().map(|(reference, _)| reference).collect()
}

fn find(series: &Series, season: u16, episode: u16) -> &yokoku_domain::Episode {
    let season = series.seasons.iter().find(|s| s.number == season).expect("season exists");
    season.episodes.iter().find(|e| e.number == episode).expect("episode exists")
}

#[rstest]
#[case::ended_even_with_future_episodes(SourceStatus::Ended, Some(TOMORROW), SeriesStatus::Ended)]
#[case::canceled(SourceStatus::Canceled, Some(TOMORROW), SeriesStatus::Ended)]
#[case::episode_scheduled(SourceStatus::Returning, Some(TOMORROW), SeriesStatus::Continuing)]
#[case::episode_airs_today(SourceStatus::Returning, Some(TODAY), SeriesStatus::Continuing)]
#[case::all_episodes_aired(SourceStatus::Returning, Some(YESTERDAY), SeriesStatus::OnBreak)]
#[case::no_dates_yet(SourceStatus::Planned, None, SeriesStatus::OnBreak)]
fn status_follows_source_and_schedule(
    #[case] source_status: SourceStatus,
    #[case] latest_air_date: Option<Date>,
    #[case] expected: SeriesStatus,
) {
    let series = Series::add(
        metadata(source_status, &[(1, &[Some(date(2020, 1, 1)), latest_air_date])]),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );

    assert_eq!(series.status(TODAY), expected);
}

#[rstest]
#[case::refreshed_recently(1, SourceStatus::Returning, 1, -100, "Pilot", false)]
#[case::running(7, SourceStatus::Returning, 1, -100, "Pilot", true)]
#[case::ended_long_ago(7, SourceStatus::Ended, 1, -100, "Pilot", false)]
#[case::canceled_long_ago(7, SourceStatus::Canceled, 1, -100, "Pilot", false)]
#[case::ended_this_month(7, SourceStatus::Ended, 1, -10, "Pilot", true)]
#[case::stale(31 * 24, SourceStatus::Ended, 1, -100, "Pilot", true)]
#[case::aired_but_untitled(1, SourceStatus::Ended, 1, -100, "TBA", true)]
#[case::aired_without_title(1, SourceStatus::Ended, 1, -100, "", true)]
#[case::untitled_but_not_aired(1, SourceStatus::Returning, 1, 10, "TBA", false)]
#[case::untitled_special(7, SourceStatus::Ended, 0, -100, "TBA", false)]
fn series_follow_sonarrs_refresh_rules(
    #[case] hours_since_refresh: i64,
    #[case] source_status: SourceStatus,
    #[case] season: u16,
    #[case] aired_days_from_today: i64,
    #[case] title: &str,
    #[case] expected: bool,
) {
    let mut series = Series::add(
        metadata(source_status, &[(season, &[Some(TODAY + aired_days_from_today.days())])]),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );
    title.clone_into(&mut series.seasons[0].episodes[0].title);

    assert_eq!(series.needs_refresh(now() + hours_since_refresh.hours(), TODAY), expected);
}

#[rstest]
#[case::aired_yesterday(Some(YESTERDAY), false, FileStatus::Missing)]
#[case::airs_today(Some(TODAY), false, FileStatus::Upcoming)]
#[case::no_date(None, false, FileStatus::Upcoming)]
#[case::downloaded(Some(YESTERDAY), true, FileStatus::Downloaded)]
fn episode_file_status(#[case] air_date: Option<Date>, #[case] has_file: bool, #[case] expected: FileStatus) {
    let mut series = Series::add(
        metadata(SourceStatus::Returning, &[(1, &[air_date])]),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );
    series.seasons[0].episodes[0].file = has_file.then(MediaFileId::generate);

    assert_eq!(series.seasons[0].episodes[0].file_status(TODAY), expected);
}

#[rstest]
#[case::all(MonitorPreset::All, true, &[(1, 1), (1, 2), (2, 1), (2, 2), (2, 3)])]
#[case::future(MonitorPreset::Future, true, &[(2, 2), (2, 3)])]
#[case::latest_season(MonitorPreset::LatestSeason, true, &[(2, 1), (2, 2), (2, 3)])]
#[case::none(MonitorPreset::None, false, &[])]
fn presets_choose_monitored_episodes(
    #[case] preset: MonitorPreset,
    #[case] series_monitored: bool,
    #[case] expected: &[(u16, u16)],
) {
    let past = Some(YESTERDAY);
    let series = Series::add(
        metadata(SourceStatus::Returning, &[(0, &[past]), (1, &[past, past]), (2, &[past, Some(TOMORROW), None])]),
        ItemFolder::default(),
        preset,
        TODAY,
        now(),
    );

    assert_eq!(series.monitored, series_monitored);
    assert_eq!(monitored(&series), refs(expected));
}

#[test]
fn seasons_and_episodes_are_ordered_by_number() {
    let mut unordered = metadata(SourceStatus::Returning, &[(2, &[None, None]), (1, &[None])]);
    unordered.seasons[0].episodes.reverse();

    let series = Series::add(unordered, ItemFolder::default(), MonitorPreset::All, TODAY, now());

    let order: Vec<_> =
        series.seasons.iter().flat_map(|s| s.episodes.iter().map(move |e| (s.number, e.number))).collect();
    assert_eq!(order, [(1, 1), (2, 1), (2, 2)]);
}

#[test]
fn refresh_keeps_identity_flags_and_files_of_renumbered_episodes() {
    let original = metadata(SourceStatus::Returning, &[(1, &[None, None])]);
    let mut series = Series::add(original.clone(), ItemFolder::default(), MonitorPreset::All, TODAY, now());
    let moved = find(&series, 1, 2).clone();
    let file = MediaFileId::generate();
    series.episode_mut(EpisodeRef { season: 1, episode: 2 }).unwrap().file = Some(file);
    series.episode_mut(EpisodeRef { season: 1, episode: 2 }).unwrap().monitored = false;

    let mut renumbered = original;
    let episode = renumbered.seasons[0].episodes.pop().unwrap();
    renumbered.seasons.push(SeasonMetadata {
        number: 2,
        episodes: vec![EpisodeMetadata { number: 1, title: "Renamed".into(), ..episode }],
    });
    let files = series.refresh(renumbered, now() + 1.hour());

    let episode = find(&series, 2, 1);
    assert_eq!(episode.id, moved.id);
    assert_eq!(episode.title, "Renamed");
    assert_eq!(episode.file, Some(file));
    assert!(!episode.monitored);
    assert_eq!(series.refreshed_at, now() + 1.hour());
    assert_eq!(files, [RenumberedFile { file, span: EpisodeSpan::new(2, 1, 1) }]);
}

#[test]
fn refresh_unlinks_a_file_whose_episodes_are_no_longer_consecutive() {
    let original = metadata(SourceStatus::Returning, &[(1, &[None, None])]);
    let mut series = Series::add(original.clone(), ItemFolder::default(), MonitorPreset::All, TODAY, now());
    let file = MediaFileId::generate();
    for episode in [1, 2] {
        series.episode_mut(EpisodeRef { season: 1, episode }).unwrap().file = Some(file);
    }

    let mut renumbered = original;
    let episode = renumbered.seasons[0].episodes.pop().unwrap();
    renumbered.seasons.push(SeasonMetadata { number: 2, episodes: vec![EpisodeMetadata { number: 1, ..episode }] });
    let files = series.refresh(renumbered, now());

    assert_eq!(files, [RenumberedFile { file, span: None }]);
    assert_eq!((find(&series, 1, 1).file, find(&series, 2, 1).file), (None, None));
}

#[test]
fn refresh_without_renumbering_reports_no_files() {
    let original = metadata(SourceStatus::Returning, &[(1, &[None, None])]);
    let mut series = Series::add(original.clone(), ItemFolder::default(), MonitorPreset::All, TODAY, now());
    series.episode_mut(EpisodeRef { season: 1, episode: 2 }).unwrap().file = Some(MediaFileId::generate());

    assert!(series.refresh(original, now()).is_empty());
}

#[rstest]
#[case::monitored_season(true, true)]
#[case::unmonitored_season(false, false)]
fn refresh_monitors_new_episodes_like_their_season(#[case] season_monitored: bool, #[case] expected: bool) {
    let mut series = Series::add(
        metadata(SourceStatus::Returning, &[(1, &[None])]),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );
    series.season_mut(1).unwrap().monitored = season_monitored;

    series.refresh(metadata(SourceStatus::Returning, &[(1, &[None, None])]), now());

    assert_eq!(find(&series, 1, 2).monitored, expected);
    assert_eq!(series.season_mut(1).unwrap().monitored, season_monitored);
}

#[rstest]
#[case::monitored_series(MonitorPreset::All, true)]
#[case::unmonitored_series(MonitorPreset::None, false)]
fn refresh_monitors_new_seasons_like_the_series(#[case] preset: MonitorPreset, #[case] expected: bool) {
    let mut series =
        Series::add(metadata(SourceStatus::Returning, &[(1, &[None])]), ItemFolder::default(), preset, TODAY, now());

    series.refresh(metadata(SourceStatus::Returning, &[(0, &[None]), (1, &[None]), (2, &[None])]), now());

    assert_eq!(series.season_mut(2).unwrap().monitored, expected);
    assert!(!series.season_mut(0).unwrap().monitored);
}

#[test]
fn refresh_drops_episodes_gone_from_the_source() {
    let mut series = Series::add(
        metadata(SourceStatus::Returning, &[(1, &[None, None])]),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );

    series.refresh(metadata(SourceStatus::Ended, &[(1, &[None])]), now());

    assert_eq!(series.episodes().count(), 1);
    assert_eq!(series.status(TODAY), SeriesStatus::Ended);
}

#[rstest]
#[case::first(1, Some((1, 1)))]
#[case::crosses_into_season_two(3, Some((2, 1)))]
#[case::last(4, Some((2, 2)))]
#[case::past_the_end(5, None)]
#[case::zero(0, None)]
fn absolute_numbers_skip_specials(#[case] absolute: u32, #[case] expected: Option<(u16, u16)>) {
    let series = Series::add(
        metadata(SourceStatus::Returning, &[(0, &[None]), (1, &[None, None]), (2, &[None, None])]),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );

    let expected = expected.map(|(season, episode)| EpisodeRef { season, episode });
    assert_eq!(series.absolute_to_ref(absolute), expected);
}

fn any_date() -> impl Strategy<Value = Date> {
    (0..3_650i64).prop_map(|days| date(2020, 1, 1) + days.days())
}

fn any_metadata() -> impl Strategy<Value = SeriesMetadata> {
    prop::collection::btree_map(0..6u16, prop::collection::vec(prop::option::of(any_date()), 0..6), 0..5).prop_map(
        |seasons: BTreeMap<u16, Vec<Option<Date>>>| {
            let seasons: Vec<(u16, &[Option<Date>])> =
                seasons.iter().map(|(&number, dates)| (number, dates.as_slice())).collect();
            metadata(SourceStatus::Returning, &seasons)
        },
    )
}

fn any_preset() -> impl Strategy<Value = MonitorPreset> {
    prop_oneof![
        Just(MonitorPreset::All),
        Just(MonitorPreset::Future),
        Just(MonitorPreset::LatestSeason),
        Just(MonitorPreset::None),
    ]
}

proptest! {
    #[test]
    fn presets_never_monitor_specials(metadata in any_metadata(), preset in any_preset(), today in any_date()) {
        let series = Series::add(metadata, ItemFolder::default(), preset, today, now());

        prop_assert!(monitored(&series).iter().all(|reference| reference.season != 0));
    }

    #[test]
    fn future_preset_monitors_exactly_the_unaired_episodes(metadata in any_metadata(), today in any_date()) {
        let series = Series::add(metadata, ItemFolder::default(), MonitorPreset::Future, today, now());

        for season in series.seasons.iter().filter(|season| season.number != 0) {
            for episode in &season.episodes {
                let unaired = episode.air_date.is_none_or(|date| date >= today);
                prop_assert_eq!(episode.monitored, unaired);
            }
        }
    }

    #[test]
    fn refreshing_with_unchanged_metadata_changes_nothing(
        metadata in any_metadata(),
        preset in any_preset(),
        today in any_date(),
    ) {
        let original = Series::add(metadata.clone(), ItemFolder::default(), preset, today, now());
        let mut refreshed = original.clone();

        refreshed.refresh(metadata, now());

        prop_assert_eq!(refreshed, original);
    }

    #[test]
    fn absolute_numbers_cover_every_regular_episode_once(metadata in any_metadata()) {
        let series = Series::add(metadata, ItemFolder::default(), MonitorPreset::All, date(2026, 1, 1), now());
        let regular = series.seasons.iter().filter(|s| s.number != 0).map(|s| s.episodes.len()).sum::<usize>();
        let count = u32::try_from(regular).unwrap();

        let mapped: Vec<_> = (1..=count).map(|n| series.absolute_to_ref(n).unwrap()).collect();

        prop_assert!(mapped.windows(2).all(|pair| pair[0] < pair[1]));
        prop_assert_eq!(series.absolute_to_ref(count + 1), None);
    }
}

#[rstest]
#[case::airing_today_is_next(&[Some(YESTERDAY), Some(TODAY), Some(TOMORROW)], Some((1, 2)), Some((1, 1)))]
#[case::nothing_scheduled(&[Some(YESTERDAY), None], None, Some((1, 1)))]
#[case::nothing_aired(&[None, Some(TOMORROW)], Some((1, 2)), None)]
#[case::no_dates(&[None, None], None, None)]
fn next_and_last_aired_episodes(
    #[case] dates: &[Option<Date>],
    #[case] next: Option<(u16, u16)>,
    #[case] last: Option<(u16, u16)>,
) {
    let series = Series::add(
        metadata(SourceStatus::Returning, &[(1, dates)]),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );
    let reference = |pair: Option<(u16, u16)>| pair.map(|(season, episode)| EpisodeRef { season, episode });

    assert_eq!(series.next_episode(TODAY).map(|(reference, _)| reference), reference(next));
    assert_eq!(series.last_aired(TODAY).map(|(reference, _)| reference), reference(last));
}

#[test]
fn same_day_episodes_are_ordered_by_number() {
    let series = Series::add(
        metadata(
            SourceStatus::Returning,
            &[(1, &[Some(TOMORROW), Some(TOMORROW)]), (2, &[Some(YESTERDAY), Some(YESTERDAY)])],
        ),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );

    assert_eq!(series.next_episode(TODAY).map(|(reference, _)| reference), Some(EpisodeRef { season: 1, episode: 1 }));
    assert_eq!(series.last_aired(TODAY).map(|(reference, _)| reference), Some(EpisodeRef { season: 2, episode: 2 }));
}

fn follow_specials(series: &mut Series) {
    let specials = series.season_mut(0).unwrap();
    specials.monitored = true;
    for episode in &mut specials.episodes {
        episode.monitored = true;
    }
}

#[rstest]
#[case::unmonitored_specials_are_ignored(false, (1, 2), (1, 1))]
#[case::monitored_specials_count(true, (0, 2), (0, 1))]
fn next_and_last_aired_include_only_followed_specials(
    #[case] follow: bool,
    #[case] next: (u16, u16),
    #[case] last: (u16, u16),
) {
    let seasons: &[(u16, &[Option<Date>])] =
        &[(0, &[Some(YESTERDAY), Some(TOMORROW)]), (1, &[Some(date(2026, 9, 1)), Some(date(2026, 10, 10))])];
    let mut series = Series::add(
        metadata(SourceStatus::Returning, seasons),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );
    if follow {
        follow_specials(&mut series);
    }

    assert_eq!(series.next_episode(TODAY).map(|(reference, _)| reference), refs(&[next]).pop());
    assert_eq!(series.last_aired(TODAY).map(|(reference, _)| reference), refs(&[last]).pop());
}

#[rstest]
#[case::unmonitored_special(false, SeriesStatus::OnBreak)]
#[case::monitored_special(true, SeriesStatus::Continuing)]
fn only_followed_specials_keep_a_series_continuing(#[case] follow: bool, #[case] expected: SeriesStatus) {
    let seasons: &[(u16, &[Option<Date>])] = &[(0, &[Some(TOMORROW)]), (1, &[Some(YESTERDAY)])];
    let mut series = Series::add(
        metadata(SourceStatus::Returning, seasons),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );
    if follow {
        follow_specials(&mut series);
    }

    assert_eq!(series.status(TODAY), expected);
}

proptest! {
    #[test]
    fn next_episode_is_the_earliest_on_or_after_today(metadata in any_metadata(), today in any_date()) {
        let series = Series::add(metadata, ItemFolder::default(), MonitorPreset::All, today, now());
        let upcoming: Vec<Date> = regular_air_dates(&series).filter(|&d| d >= today).collect();

        let next = series.next_episode(today).and_then(|(_, episode)| episode.air_date);

        prop_assert_eq!(next, upcoming.iter().copied().min());
    }

    #[test]
    fn last_aired_is_the_latest_before_today(metadata in any_metadata(), today in any_date()) {
        let series = Series::add(metadata, ItemFolder::default(), MonitorPreset::All, today, now());
        let aired: Vec<Date> = regular_air_dates(&series).filter(|&d| d < today).collect();

        let last = series.last_aired(today).and_then(|(_, episode)| episode.air_date);

        prop_assert_eq!(last, aired.iter().copied().max());
    }
}
