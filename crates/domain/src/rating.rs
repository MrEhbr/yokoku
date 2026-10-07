/// Where a rating comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RatingSource {
    /// Users' average from 1 to 10.
    Imdb,
}

crate::string_enum!(RatingSource, "rating source" {
    Imdb => "imdb",
});

/// An item's rating at one source, on that source's scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rating {
    pub source: RatingSource,
    pub value: f32,
    /// `None` when the source gives no count.
    pub votes: Option<u32>,
}
