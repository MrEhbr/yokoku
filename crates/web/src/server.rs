use std::{io, path::Path};

use tokio::net::TcpListener;
use topcoat::{asset::AssetBundle, router::Router};

/// The web interface with its asset bundle loaded, ready to serve.
pub struct Server {
    router: Router,
}

impl Server {
    /// Loads the asset bundle from `assets`, or from `assets/` beside the executable.
    ///
    /// # Errors
    ///
    /// Fails when the bundle's `manifest.toml` is missing or unreadable.
    pub fn new(assets: Option<&Path>) -> io::Result<Self> {
        let bundle = match assets {
            Some(dir) => AssetBundle::load_dir(dir)?,
            None => AssetBundle::load()?,
        };
        Ok(Self { router: crate::router(bundle) })
    }

    /// Serves on `listener` until `shutdown` completes.
    ///
    /// # Errors
    ///
    /// Fails when accepting a connection fails.
    pub async fn run(self, listener: TcpListener, shutdown: impl Future<Output = ()>) -> io::Result<()> {
        topcoat::serve_until(listener, self.router, shutdown).await
    }
}
