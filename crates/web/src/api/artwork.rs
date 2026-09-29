//! `GET /artwork/{series|movie}/{id}/{kind}/{name}`: an item's poster, backdrop or logo. `name`
//! changes with the image, so a URL's image never changes and browsers keep it.
//!
//! `GET /artwork/preview/{source}/{kind}?path=`: an image of an item not in the library, like a
//! search result's poster, from its source's image server.
//!
//! Plain routes, not server functions: they answer with the image itself.

use dioxus::{
    logger::tracing::error,
    server::axum::{
        extract::{Path, Query},
        http::{
            StatusCode,
            header::{CACHE_CONTROL, CONTENT_TYPE},
        },
        response::{IntoResponse, Response},
    },
};
use serde::Deserialize;
use yokoku_domain::{ArtworkKind, ExternalId, ItemId};
use yokoku_library::{LibraryError, ports::MetadataError};

use super::{Artworks, Dep};

pub(crate) const ROUTE: &str = "/artwork/{item}/{id}/{kind}/{name}";
pub(crate) const PREVIEW_ROUTE: &str = "/artwork/preview/{source}/{kind}";

/// The URL of the item's `kind` image `name`.
pub(crate) fn url(item: ItemId, kind: ArtworkKind, name: &str) -> String {
    match item {
        ItemId::Series(id) => format!("/artwork/series/{id}/{kind}/{name}"),
        ItemId::Movie(id) => format!("/artwork/movie/{id}/{kind}/{name}"),
    }
}

pub(crate) async fn image(
    Path((item, id, kind, _name)): Path<(String, String, String, String)>,
    artworks: Dep<Artworks>,
) -> Response {
    let item = match item.as_str() {
        "series" => id.parse().map(ItemId::Series).ok(),
        "movie" => id.parse().map(ItemId::Movie).ok(),
        _ => None,
    };
    let (Some(item), Ok(kind)) = (item, kind.parse::<ArtworkKind>()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match artworks.image(item, kind).await {
        Ok(Some(image)) => {
            ([(CONTENT_TYPE, image.content_type), (CACHE_CONTROL, "public, max-age=31536000, immutable")], image.bytes)
                .into_response()
        },
        Ok(None)
        | Err(
            LibraryError::SeriesNotFound(_)
            | LibraryError::MovieNotFound(_)
            | LibraryError::Metadata(MetadataError::NotFound(_)),
        ) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            error!(%error, ?item, %kind, "loading artwork failed");
            StatusCode::BAD_GATEWAY.into_response()
        },
    }
}

/// The URL of the `kind` image at `path` of `source`, a TMDB path or a TVDB URL.
pub(crate) fn preview_url(source: ExternalId, kind: ArtworkKind, path: &str) -> String {
    format!("/artwork/preview/{source}/{kind}?path={}", encode(path))
}

#[derive(Deserialize)]
pub(crate) struct PreviewQuery {
    path: String,
}

pub(crate) async fn preview(
    Path((source, kind)): Path<(String, String)>,
    Query(PreviewQuery { path }): Query<PreviewQuery>,
    artworks: Dep<Artworks>,
) -> Response {
    let (Ok(source), Ok(kind)) = (source.parse::<ExternalId>(), kind.parse::<ArtworkKind>()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match artworks.preview(source, kind, &path).await {
        Ok(Some(image)) => {
            ([(CONTENT_TYPE, image.content_type), (CACHE_CONTROL, "public, max-age=86400")], image.bytes)
                .into_response()
        },
        Ok(None) | Err(LibraryError::Metadata(MetadataError::NotFound(_) | MetadataError::Invalid(_))) => {
            StatusCode::NOT_FOUND.into_response()
        },
        Err(error) => {
            error!(%error, %source, %kind, "loading a preview image failed");
            StatusCode::BAD_GATEWAY.into_response()
        },
    }
}

/// Percent-encodes every byte but unreserved characters and `/`.
fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
