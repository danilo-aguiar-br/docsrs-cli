//! Trust anchors for the TLS client (ADR 0007 section 4).
//!
//! Why this is a module rather than three lines inside [`super::client`]: the
//! root set is the one part of the TLS posture that changed underneath the
//! product without anyone noticing. reqwest 0.12 offered a
//! `rustls-tls-webpki-roots` feature; reqwest 0.13 removed every webpki-roots
//! feature and made `rustls-no-provider` pull `rustls-platform-verifier`
//! unconditionally. The 0.12 to 0.13 upgrade therefore moved trust from a
//! compiled-in Mozilla set to the operating system store, silently, while ADR
//! 0007 went on stating the opposite and explaining why the opposite was better.
//!
//! Owning the config here, from a direct `webpki-roots` dependency, turns the
//! anchor set into a decision this crate makes instead of a side effect of a
//! dependency's feature wiring — and gives the invariant somewhere to be tested.

use std::sync::Arc;

use rustls::ClientConfig;

use crate::error::{AppError, AppResult, ErrorDetail};

/// Build the rustls client config: Mozilla roots, TLS 1.2 floor, no client auth.
///
/// `rustls-platform-verifier` stays in the dependency graph because reqwest's
/// only public gateway to its rustls path (`rustls-no-provider`) pulls it
/// unconditionally. It is no longer consulted: a preconfigured config replaces
/// the verifier reqwest would otherwise install.
pub(super) fn client_config() -> AppResult<ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    // `builder_with_provider`, not `builder`: the latter reads the process-wide
    // default provider, which library code must not assume was installed.
    ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        // The floor is declared here because a preconfigured config makes
        // reqwest's `min_tls_version` inert — it would look applied and do
        // nothing, which is the failure mode this whole module exists to end.
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
        .map_err(|e| AppError::of_with_source(ErrorDetail::HttpClientBuild, e))
        .map(|builder| builder.with_root_certificates(roots).with_no_client_auth())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mozilla_anchor_reaches_the_store() {
        // An empty or truncated store rejects every certificate, and the failure
        // surfaces as a network error rather than as a configuration one. This
        // is also the assertion that would have caught the 0.12 to 0.13 swap.
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        assert_eq!(roots.len(), webpki_roots::TLS_SERVER_ROOTS.len());
        assert!(
            roots.len() > 100,
            "the Mozilla set holds hundreds of anchors; got {}",
            roots.len()
        );
    }

    #[test]
    fn the_client_config_builds_without_a_process_default_provider() {
        assert!(client_config().is_ok());
    }
}
