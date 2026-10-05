use serde::Deserialize;

/// A search answer: an RSS feed, or an `<error>` root with `code` and `description`.
#[derive(Deserialize)]
pub(crate) struct Feed {
    #[serde(rename = "@code")]
    pub code: Option<u16>,
    #[serde(rename = "@description")]
    pub error: Option<String>,
    pub channel: Option<Channel>,
}

#[derive(Deserialize)]
pub(crate) struct Channel {
    #[serde(default, rename = "item")]
    pub items: Vec<Item>,
}

#[derive(Deserialize)]
pub(crate) struct Item {
    pub title: String,
    pub link: Option<String>,
    /// The release's page.
    pub comments: Option<String>,
    /// RFC 2822.
    #[serde(rename = "pubDate")]
    pub pub_date: Option<String>,
    pub size: Option<u64>,
    pub grabs: Option<u32>,
    pub jackettindexer: Option<Text>,
    /// `torznab:attr` elements; names match without their prefix.
    #[serde(default, rename = "attr")]
    pub attrs: Vec<Attr>,
}

impl Item {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|attr| attr.name == name).map(|attr| attr.value.as_str())
    }
}

#[derive(Deserialize)]
pub(crate) struct Text {
    #[serde(rename = "$text")]
    pub text: String,
}

#[derive(Deserialize)]
pub(crate) struct Attr {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value")]
    pub value: String,
}

/// An indexers answer, or an `<error>` root.
#[derive(Deserialize)]
pub(crate) struct Indexers {
    #[serde(rename = "@description")]
    pub error: Option<String>,
    #[serde(default, rename = "indexer")]
    pub indexers: Vec<Indexer>,
}

#[derive(Deserialize)]
pub(crate) struct Indexer {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@configured")]
    pub configured: bool,
    pub title: String,
}

/// A caps answer, or an `<error>` root.
#[derive(Deserialize)]
pub(crate) struct Caps {
    #[serde(rename = "@description")]
    pub error: Option<String>,
    pub server: Option<Server>,
}

#[derive(Deserialize)]
pub(crate) struct Server {
    #[serde(rename = "@title")]
    pub title: Option<String>,
    #[serde(rename = "@version")]
    pub version: Option<String>,
}
