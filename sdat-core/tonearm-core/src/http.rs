use reqwest::blocking::ClientBuilder;

/// Every HTTP client of the core starts here.
///
/// On Android, reqwest's default verifier (rustls-platform-verifier) panics unless
/// it was initialized over JNI with the app's Context, so the client trusts the
/// bundled Mozilla roots instead.
pub(crate) fn client_builder() -> ClientBuilder {
    let builder = ClientBuilder::new();
    #[cfg(target_os = "android")]
    let builder = builder.tls_certs_only(
        webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter().filter_map(|c| reqwest::Certificate::from_der(c).ok()),
    );
    builder
}
