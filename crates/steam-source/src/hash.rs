use sha2::{Digest, Sha256};

/// SHA-256 hex digest of raw bytes. Used for response de-duplication and
/// review query parameter fingerprints.
pub fn content_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_encode(&digest)
}

/// Stable parameter hash: keys sorted, joined as `k=v&...`, then hashed.
pub fn parameter_hash(params: &[(&str, &str)]) -> String {
    let mut pairs: Vec<(&str, &str)> = params.to_vec();
    pairs.sort_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(b.1)));
    let canonical = pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    content_hash(canonical.as_bytes())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{content_hash, parameter_hash};

    #[test]
    fn content_hash_is_stable() {
        assert_eq!(
            content_hash(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn parameter_hash_sorts_keys() {
        let a = parameter_hash(&[("b", "2"), ("a", "1")]);
        let b = parameter_hash(&[("a", "1"), ("b", "2")]);
        assert_eq!(a, b);
        assert_ne!(a, parameter_hash(&[("a", "1"), ("b", "3")]));
    }
}
