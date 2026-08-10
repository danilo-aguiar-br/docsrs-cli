//! SHA-256 hex helpers for cache keys and digests (DRY).

use sha2::{Digest, Sha256};

/// Bytes in a SHA-256 digest.
const SHA256_DIGEST_BYTES: usize = 32;

/// Characters in the lowercase hex rendering of a SHA-256 digest.
///
/// Derived from the digest width rather than written as `64`, so the constant
/// states *why* the length is what it is: two hex characters per byte.
pub(super) const SHA256_HEX_LEN: usize = SHA256_DIGEST_BYTES * 2;

/// True when `s` is exactly [`SHA256_HEX_LEN`] lowercase hex digits.
pub(super) fn is_sha256_hex(s: &str) -> bool {
    s.len() == SHA256_HEX_LEN && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Alias kept for call-site clarity (cache key material hash).
#[inline]
pub(super) fn is_cache_key_hex(key: &str) -> bool {
    is_sha256_hex(key)
}

pub(super) fn input_checksum(url: &str, parser_version: &str, accept: &str) -> String {
    let mut h = Sha256::new();
    h.update(url.as_bytes());
    h.update([0u8]);
    h.update(parser_version.as_bytes());
    h.update([0u8]);
    h.update(accept.as_bytes());
    hex_encode(h.finalize())
}

pub(super) fn key_hex(url: &str, parser_version: &str, accept: &str) -> String {
    input_checksum(url, parser_version, accept)
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex_encode(h.finalize())
}

pub(super) fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_sha256_hex_accepts_only_64_lowercase() {
        assert!(is_sha256_hex(&"a".repeat(64)));
        assert!(is_sha256_hex(&"0".repeat(64)));
        assert!(!is_sha256_hex(".."));
        assert!(!is_sha256_hex(""));
        assert!(!is_sha256_hex(&"g".repeat(64)));
        assert!(!is_sha256_hex(&"A".repeat(64))); // uppercase rejected
        assert!(!is_sha256_hex(&"a".repeat(63)));
    }
}
