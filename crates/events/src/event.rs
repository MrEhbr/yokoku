use serde::{Deserialize, Serialize};
use yokoku_domain::{MovieId, SeriesId};

/// Stored events never change meaning; a breaking change adds a new variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    SeriesAdded { series: SeriesId, title: String },
    MovieAdded { movie: MovieId, title: String },
    SeriesRemoved { series: SeriesId, title: String, delete_files: bool },
    MovieRemoved { movie: MovieId, title: String, delete_files: bool },
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;

    #[rstest]
    #[case::series_added(
        Event::SeriesAdded { series: SeriesId(Uuid::from_u128(7)), title: "Frieren".into() },
        json!({ "type": "SeriesAdded", "series": "00000000-0000-0000-0000-000000000007", "title": "Frieren" }),
    )]
    #[case::movie_added(
        Event::MovieAdded { movie: MovieId(Uuid::from_u128(3)), title: "Dune".into() },
        json!({ "type": "MovieAdded", "movie": "00000000-0000-0000-0000-000000000003", "title": "Dune" }),
    )]
    #[case::series_removed(
        Event::SeriesRemoved { series: SeriesId(Uuid::from_u128(7)), title: "Frieren".into(), delete_files: true },
        json!({ "type": "SeriesRemoved", "series": "00000000-0000-0000-0000-000000000007", "title": "Frieren", "delete_files": true }),
    )]
    #[case::movie_removed(
        Event::MovieRemoved { movie: MovieId(Uuid::from_u128(3)), title: "Dune".into(), delete_files: false },
        json!({ "type": "MovieRemoved", "movie": "00000000-0000-0000-0000-000000000003", "title": "Dune", "delete_files": false }),
    )]
    fn stored_format_is_stable(#[case] event: Event, #[case] stored: serde_json::Value) {
        assert_eq!(serde_json::to_value(&event).unwrap(), stored);
        assert_eq!(serde_json::from_value::<Event>(stored).unwrap(), event);
    }

    fn any_event() -> impl Strategy<Value = Event> {
        prop_oneof![
            (any::<u128>(), any::<String>())
                .prop_map(|(id, title)| Event::SeriesAdded { series: SeriesId(Uuid::from_u128(id)), title }),
            (any::<u128>(), any::<String>())
                .prop_map(|(id, title)| Event::MovieAdded { movie: MovieId(Uuid::from_u128(id)), title }),
            (any::<u128>(), any::<String>(), any::<bool>()).prop_map(|(id, title, delete_files)| {
                Event::SeriesRemoved { series: SeriesId(Uuid::from_u128(id)), title, delete_files }
            }),
            (any::<u128>(), any::<String>(), any::<bool>()).prop_map(|(id, title, delete_files)| {
                Event::MovieRemoved { movie: MovieId(Uuid::from_u128(id)), title, delete_files }
            }),
        ]
    }

    proptest! {
        #[test]
        fn round_trips_through_json(event in any_event()) {
            let stored = serde_json::to_string(&event).unwrap();
            prop_assert_eq!(serde_json::from_str::<Event>(&stored).unwrap(), event);
        }
    }
}
