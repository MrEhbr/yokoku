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

    use super::*;

    #[rstest]
    #[case::series_added(
        Event::SeriesAdded { series: SeriesId(7), title: "Frieren".into() },
        json!({ "type": "SeriesAdded", "series": 7, "title": "Frieren" }),
    )]
    #[case::movie_added(
        Event::MovieAdded { movie: MovieId(3), title: "Dune".into() },
        json!({ "type": "MovieAdded", "movie": 3, "title": "Dune" }),
    )]
    fn stored_format_is_stable(#[case] event: Event, #[case] stored: serde_json::Value) {
        assert_eq!(serde_json::to_value(&event).unwrap(), stored);
        assert_eq!(serde_json::from_value::<Event>(stored).unwrap(), event);
    }

    fn any_event() -> impl Strategy<Value = Event> {
        prop_oneof![
            (any::<i64>(), any::<String>()).prop_map(|(id, title)| Event::SeriesAdded { series: SeriesId(id), title }),
            (any::<i64>(), any::<String>()).prop_map(|(id, title)| Event::MovieAdded { movie: MovieId(id), title }),
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
