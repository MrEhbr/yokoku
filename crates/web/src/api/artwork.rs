//! `GET /artwork/{series|movie}/{id}/{kind}/{name}`: an item's poster, backdrop or logo. A plain
//! route, not a server function: it answers with the image itself. `name` changes with the image,
//! so a URL's image never changes and browsers keep it.

use dioxus::{
    logger::tracing::error,
    server::axum::{
        extract::Path,
        http::{
            StatusCode,
            header::{CACHE_CONTROL, CONTENT_TYPE},
        },
        response::{IntoResponse, Response},
    },
};
use yokoku_domain::{ArtworkKind, ItemId};
use yokoku_library::{LibraryError, ports::MetadataError};

use super::{Artworks, Dep};

pub(crate) const ROUTE: &str = "/artwork/{item}/{id}/{kind}/{name}";

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
