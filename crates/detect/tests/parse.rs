use std::path::Path;

use jiff::civil::date;
use rstest::rstest;
use yokoku_detect::{Numbers, parse};

fn episodes(season: u16, episodes: &[u16]) -> Numbers {
    Numbers::Episodes { season, episodes: episodes.to_vec() }
}

fn seasonless(episodes: &[u16], folder_season: Option<u16>) -> Numbers {
    Numbers::Seasonless { episodes: episodes.to_vec(), folder_season }
}

fn aired(year: i16, month: i8, day: i8) -> Numbers {
    Numbers::Date(date(year, month, day))
}

/// `title: None` means the title is left to library matching and not asserted.
#[rstest]
// scene naming
#[case("The.Walking.Dead.S05E03.720p.BluRay.x264-DEMAND.mkv", Some("The Walking Dead"), None, episodes(5, &[3]))]
#[case("the.expanse.s1e2.720p.mkv", Some("the expanse"), None, episodes(1, &[2]))]
#[case("Doctor.Who.2005.1x02.HDTV.mkv", Some("Doctor Who"), Some(2005), episodes(1, &[2]))]
#[case("Breaking Bad Season 1 Episode 2.mkv", Some("Breaking Bad"), None, episodes(1, &[2]))]
#[case("Game.of.Thrones.S08E06.The.Iron.Throne.1080p.AMZN.WEB-DL.DDP5.1.H.264-GoT.mkv", Some("Game of Thrones"), None, episodes(8, &[6]))]
#[case("Severance.S02E01.Hello.Ms.Cobel.2160p.ATVP.WEB-DL.DDP5.1.Atmos.DV.HDR.H.265-FLUX.mkv", Some("Severance"), None, episodes(2, &[1]))]
#[case("The.Office.US.S02E01.720p.mkv", None, None, episodes(2, &[1]))]
#[case("Shogun.2024.S01E04.1080p.WEB.h264-ETHEL.mkv", Some("Shogun"), Some(2024), episodes(1, &[4]))]
// multi-episode
#[case("Friends.S01E01-E03.720p.mkv", Some("Friends"), None, episodes(1, &[1, 2, 3]))]
#[case("Friends.S01E01E02.720p.mkv", Some("Friends"), None, episodes(1, &[1, 2]))]
#[case("Stargate.SG-1.S01E01-02.DVDRip.mkv", Some("Stargate SG-1"), None, episodes(1, &[1, 2]))]
#[case("Lost.S01E01.E02.720p.mkv", Some("Lost"), None, episodes(1, &[1, 2]))]
// fansub, absolute numbering
#[case("[SubsPlease] Sousou no Frieren - 12 (1080p) [ABCDEF12].mkv", Some("Sousou no Frieren"), None, seasonless(&[12], None))]
#[case("[Erai-raws] One Piece - 1085 [1080p][Multiple Subtitle].mkv", Some("One Piece"), None, seasonless(&[1085], None))]
#[case("[HorribleSubs] Kimetsu no Yaiba - 26 [720p].mkv", Some("Kimetsu no Yaiba"), None, seasonless(&[26], None))]
#[case("[Group] Vinland Saga - 12v2 [1080p].mkv", Some("Vinland Saga"), None, seasonless(&[12], None))]
#[case("[ASW] Dungeon Meshi - 01 [1080p HEVC][A1B2C3D4].mkv", Some("Dungeon Meshi"), None, seasonless(&[1], None))]
#[case("[Group] 86 - Eighty Six - 05 [1080p].mkv", Some("86 - Eighty Six"), None, seasonless(&[5], None))]
#[case("[Judas] Shingeki no Kyojin - S04E28 [1080p][HEVC x265 10bit].mkv", Some("Shingeki no Kyojin"), None, episodes(4, &[28]))]
#[case("[SubsPlease] Mushoku Tensei S2 - 13 (1080p) [8E1A2B3C].mkv", Some("Mushoku Tensei"), None, episodes(2, &[13]))]
// daily shows
#[case(
    "The.Daily.Show.2026.09.26.Guest.Name.720p.WEB.h264-EDITH.mkv",
    Some("The Daily Show"),
    None,
    aired(2026, 9, 26)
)]
#[case("Jeopardy.2026-09-25.720p.mkv", Some("Jeopardy"), None, aired(2026, 9, 25))]
#[case(
    "Last.Week.Tonight.with.John.Oliver.2026.09.20.1080p.WEB.h264-GGEZ.mkv",
    Some("Last Week Tonight with John Oliver"),
    None,
    aired(2026, 9, 20)
)]
// specials
#[case("Doctor.Who.2005.S00E150.The.Day.of.the.Doctor.mkv", Some("Doctor Who"), Some(2005), episodes(0, &[150]))]
#[case("Sherlock.S00E01.Unaired.Pilot.720p.mkv", Some("Sherlock"), None, episodes(0, &[1]))]
#[case("Doctor Who/Specials/Doctor.Who.E03.mkv", Some("Doctor Who"), None, seasonless(&[3], Some(0)))]
// season and series packs
#[case("Breaking.Bad.S02.1080p.BluRay/Breaking.Bad.E05.1080p.mkv", Some("Breaking Bad"), None, seasonless(&[5], Some(2)))]
#[case("Friends Season 3/Friends - 07.mkv", Some("Friends"), None, seasonless(&[7], Some(3)))]
#[case("Breaking.Bad.S02.1080p.BluRay/05.mkv", Some("Breaking Bad"), None, seasonless(&[5], Some(2)))]
#[case("The.Wire.Complete.Series/Season 1/The.Wire.S01E01.mkv", Some("The Wire"), None, episodes(1, &[1]))]
#[case("The.Wire.Complete.Series/Season 4/03.mkv", Some("The Wire Complete Series"), None, seasonless(&[3], Some(4)))]
// russian
#[case("Метод.S02E03.WEB-DL.1080p.mkv", Some("Метод"), None, episodes(2, &[3]))]
#[case("The.Witcher.S01E03.1080p.rus.LostFilm.TV.mkv", Some("The Witcher"), None, episodes(1, &[3]))]
#[case("Ведьмак.S01E03.Предательская.луна.1080p.mkv", Some("Ведьмак"), None, episodes(1, &[3]))]
#[case("Ведьмак - 1 сезон 3 серия.mkv", Some("Ведьмак"), None, episodes(1, &[3]))]
#[case("Ведьмак.Сезон.1.Серия.3.mkv", Some("Ведьмак"), None, episodes(1, &[3]))]
#[case("Сериал/2 сезон/01.mkv", Some("Сериал"), None, seasonless(&[1], Some(2)))]
#[case("Сериал/Сезон 2/Сериал - 04.mkv", Some("Сериал"), None, seasonless(&[4], Some(2)))]
// jellyfin naming, as written by yokoku
#[case(
    "Frieren - Beyond Journey's End (2023)/Season 01/Frieren - Beyond Journey's End (2023) - S01E01 - Episode 1.mkv",
    Some("Frieren - Beyond Journey's End"),
    Some(2023),
    episodes(1, &[1])
)]
#[case(
    "Frieren - Beyond Journey's End (2023)/Season 01/Frieren - Beyond Journey's End (2023) - S01E01-E02.mkv",
    Some("Frieren - Beyond Journey's End"),
    Some(2023),
    episodes(1, &[1, 2])
)]
#[case("Frieren - Beyond Journey's End (2023)/Season 01/03.mkv", Some("Frieren - Beyond Journey's End"), None, seasonless(&[3], Some(1)))]
#[case("Dune - Part Two (2024)/Dune - Part Two (2024).mkv", Some("Dune - Part Two"), Some(2024), Numbers::None)]
#[case("300 (2006)/300 (2006).mkv", Some("300"), Some(2006), Numbers::None)]
// movies
#[case("The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv", Some("The Matrix"), Some(1999), Numbers::None)]
#[case("2001.A.Space.Odyssey.1968.1080p.BluRay.mkv", Some("2001 A Space Odyssey"), Some(1968), Numbers::None)]
#[case("Амели.2001.BDRip.1080p.mkv", Some("Амели"), Some(2001), Numbers::None)]
#[case("1917.2019.1080p.BluRay.x264.mkv", Some("1917"), Some(2019), Numbers::None)]
fn parses_release_names(
    #[case] path: &str,
    #[case] title: Option<&str>,
    #[case] year: Option<i16>,
    #[case] numbers: Numbers,
) {
    let parsed = parse(Path::new(path));

    if let Some(title) = title {
        assert_eq!(parsed.title.as_deref(), Some(title), "{path}");
    }
    assert_eq!(parsed.year, year, "{path}");
    assert_eq!(parsed.numbers, numbers, "{path}");
}

#[rstest]
#[case("Blade.Runner.2049.2017.2160p.UHD.BluRay.x265-TERMiNAL.mkv", 2017)]
#[case("Dune.Part.Two.2024.1080p.WEB-DL.DDP5.1.Atmos.H.264-FLUX.mkv", 2024)]
fn movie_years_are_reliable_when_titles_are_not(#[case] path: &str, #[case] year: i16) {
    assert_eq!(parse(Path::new(path)).year, Some(year));
}

#[rstest]
#[case("Game.of.Thrones.S08E06.The.Iron.Throne.1080p.mkv", Some("The Iron Throne"))]
#[case("Breaking Bad/Season 2/03 - Grilled.mkv", Some("Grilled"))]
#[case("Breaking Bad/Season 2/03.Grilled.mkv", Some("Grilled"))]
#[case("Breaking Bad/Season 2/03.mkv", None)]
#[case("Frieren (2023)/Season 01/Frieren (2023) - S01E01 - The Journey's End.mkv", Some("The Journey's End"))]
fn keeps_episode_titles(#[case] path: &str, #[case] episode_title: Option<&str>) {
    assert_eq!(parse(Path::new(path)).episode_title.as_deref(), episode_title);
}

proptest::proptest! {
    #[test]
    fn any_path_parses_to_well_formed_numbers(path in "[\\PC/]{0,80}") {
        let numbers = parse(Path::new(&path)).numbers;

        match numbers {
            Numbers::Episodes { episodes, .. } | Numbers::Seasonless { episodes, .. } => {
                proptest::prop_assert!(!episodes.is_empty());
                proptest::prop_assert!(episodes.windows(2).all(|pair| pair[0] < pair[1]));
            },
            Numbers::None | Numbers::Date(_) => {},
        }
    }
}
