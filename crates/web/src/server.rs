use std::{
    future::Future,
    io::{self, ErrorKind},
    path::PathBuf,
};

use dioxus::server::axum::{self, Extension, Router};
use tokio::net::TcpListener;

/// What server functions read from the composition root.
#[derive(Clone)]
pub struct AppState {
    pub version: &'static str,
}

/// The web UI, bound and ready to serve pages, assets and server functions.
pub struct Server {
    listener: TcpListener,
    router: Router,
}

impl Server {
    /// Binds `IP`/`PORT` (default 127.0.0.1:8080). Fails when the web assets or the address are unavailable.
    pub async fn bind(state: AppState) -> io::Result<Self> {
        let assets = public_dir()?;
        if !assets.is_dir() {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!(
                    "web assets not found at {}; run `just web serve`, bundle them with `dx bundle`, or set DIOXUS_PUBLIC_PATH",
                    assets.display()
                ),
            ));
        }
        let router = dioxus::server::router(crate::App).layer(Extension(state));
        let listener = TcpListener::bind(dioxus::cli_config::fullstack_address_or_localhost()).await?;
        Ok(Self { listener, router })
    }

    pub async fn serve(self, shutdown: impl Future<Output = ()> + Send + 'static) -> io::Result<()> {
        axum::serve(self.listener, self.router).with_graceful_shutdown(shutdown).await
    }
}

/// `DIOXUS_PUBLIC_PATH`, or `public` next to the executable, as `dioxus::server::router` resolves it.
fn public_dir() -> io::Result<PathBuf> {
    if let Some(path) = std::env::var_os("DIOXUS_PUBLIC_PATH") {
        return Ok(path.into());
    }
    let exe = std::env::current_exe()?;
    Ok(exe.parent().unwrap_or(&exe).join("public"))
}
