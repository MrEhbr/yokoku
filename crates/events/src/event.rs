use serde::{Deserialize, Serialize};
use yokoku_domain::{MovieId, SeriesId};

/// Stored events never change meaning; a breaking change adds a new variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    SeriesAdded { series: SeriesId, title: String },
    MovieAdded { movie: MovieId, title: String },
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
