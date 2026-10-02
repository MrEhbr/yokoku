/// Installs ring as the process-wide rustls crypto provider, which reqwest needs before it builds
/// a client; calls after the first do nothing.
pub(crate) fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
