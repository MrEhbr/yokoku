use std::{
    future::Future,
    io::{self, ErrorKind},
    net::SocketAddr,
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
    /// Binds `address`, or the address `dx` assigns when it runs the service. Fails when the web
    /// assets or the address are unavailable.
    pub async fn bind(address: SocketAddr, state: AppState) -> io::Result<Self> {
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
        let address = if dioxus::cli_config::is_cli_enabled() {
            dioxus::cli_config::fullstack_address_or_localhost()
        } else {
            address
        };
        let listener = TcpListener::bind(address).await?;
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
