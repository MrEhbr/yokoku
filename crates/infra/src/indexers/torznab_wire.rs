use serde::Deserialize;
use yokoku_core::downloads::ports::Release;

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
    #[serde(default, rename = "enclosure")]
    pub enclosures: Vec<Enclosure>,
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

    pub fn download_link(&self) -> Option<String> {
        self.enclosures
            .iter()
            .find(|enclosure| {
                enclosure.kind.as_deref().is_some_and(|kind| kind.starts_with("application/x-bittorrent"))
            })
            .map(|enclosure| enclosure.url.clone())
            .or_else(|| self.attr("magneturl").map(str::to_owned))
            .or_else(|| self.link.clone())
    }
}

/// A Torznab item without a link to its torrent.
#[derive(Debug)]
pub(crate) struct NoLink;

impl TryFrom<Item> for Release {
    type Error = NoLink;

    fn try_from(item: Item) -> Result<Self, NoLink> {
        let link = item.download_link().ok_or(NoLink)?;
        let number = |name: &str| item.attr(name).and_then(|value| value.parse::<u32>().ok());
        let seeders = number("seeders");
        let size = item.size.or_else(|| item.attr("size").and_then(|size| size.parse().ok())).unwrap_or(0);
        let published = item.pub_date.as_deref().and_then(|date| jiff::fmt::rfc2822::parse(date).ok());
        Ok(Release {
            tracker: item.jackettindexer.as_ref().map(|indexer| indexer.text.clone()).unwrap_or_default(),
            leechers: number("peers").map(|peers| peers.saturating_sub(seeders.unwrap_or(0))),
            grabs: item.grabs.or_else(|| number("grabs")),
            seeders,
            size,
            published: published.map(|date| date.timestamp()),
            link,
            details: item.comments,
            title: item.title,
        })
    }
}

#[derive(Deserialize)]
pub(crate) struct Enclosure {
    #[serde(rename = "@url")]
    pub url: String,
    #[serde(rename = "@type")]
    pub kind: Option<String>,
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
    pub searching: Option<Searching>,
}

#[derive(Deserialize)]
pub(crate) struct Searching {
    pub search: Option<SearchMode>,
    #[serde(rename = "tv-search")]
    pub tv: Option<SearchMode>,
    #[serde(rename = "movie-search")]
    pub movie: Option<SearchMode>,
}

#[derive(Deserialize)]
pub(crate) struct SearchMode {
    #[serde(rename = "@available")]
    pub available: String,
    #[serde(rename = "@supportedParams", default)]
    pub supported: String,
}

impl SearchMode {
    pub fn supports(&self, parameter: &str) -> bool {
        self.available == "yes" && self.supported.split(',').any(|part| part.trim() == parameter)
    }
}

#[derive(Deserialize)]
pub(crate) struct Server {
    #[serde(rename = "@title")]
    pub title: Option<String>,
    #[serde(rename = "@version")]
    pub version: Option<String>,
}
