/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use url::Url;

/// Hashes a URL in a canonical form so that semantically equivalent URLs
/// produce identical hashes regardless of query-parameter ordering.
///
/// Operates on structured URL components — scheme, host, port, path, and
/// sorted (key, value) query pairs — instead of the raw string. This avoids
/// percent-encoding round-trips and removes the order-sensitivity introduced
/// by `Url::as_str()`. The fragment is intentionally excluded as it is
/// client-side state and never sent to the server.
pub fn hash_url<H: Hasher>(url: &Url, state: &mut H) {
    url.scheme().hash(state);
    url.host_str().hash(state);
    url.port_or_known_default().hash(state);
    url.path().hash(state);

    let mut pairs: Vec<_> = url.query_pairs().collect();
    pairs.sort();
    for (key, value) in &pairs {
        key.hash(state);
        value.hash(state);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestHash(String);

impl RequestHash {
    pub fn new(value: &impl Hash) -> Self {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        RequestHash(format!("{:x}", hasher.finish()))
    }
}

impl From<&str> for RequestHash {
    fn from(s: &str) -> Self {
        RequestHash(s.to_string())
    }
}

impl From<String> for RequestHash {
    fn from(s: String) -> Self {
        RequestHash(s)
    }
}

impl std::fmt::Display for RequestHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_value_produces_same_hash() {
        let hash1 = RequestHash::new(&("GET", "https://example.com/api"));
        let hash2 = RequestHash::new(&("GET", "https://example.com/api"));
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_values_produce_different_hashes() {
        let hash1 = RequestHash::new(&("GET", "https://example.com/api1"));
        let hash2 = RequestHash::new(&("GET", "https://example.com/api2"));
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_request_hash_from_string() {
        let hash_str = "abc123def456";
        let hash = RequestHash::from(hash_str);
        assert_eq!(hash.to_string(), hash_str);

        let hash_string = String::from("xyz789");
        let hash2 = RequestHash::from(hash_string);
        assert_eq!(hash2.to_string(), "xyz789");
    }

    fn hash_one(url: &Url) -> u64 {
        let mut hasher = DefaultHasher::new();
        hash_url(url, &mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_hash_url_invariant_to_query_order() {
        let url_a: Url = "https://example.com/path?b=2&a=1".parse().unwrap();
        let url_b: Url = "https://example.com/path?a=1&b=2".parse().unwrap();

        assert_eq!(
            hash_one(&url_a),
            hash_one(&url_b),
            "URLs with the same query params in different order must hash equally",
        );
    }

    #[test]
    fn test_hash_url_invariant_to_query_order_with_repeats() {
        // Repeated keys preserve their pairing ((k, v) is the unit).
        let url_a: Url = "https://example.com/p?tag=red&tag=blue&id=7"
            .parse()
            .unwrap();
        let url_b: Url = "https://example.com/p?id=7&tag=blue&tag=red"
            .parse()
            .unwrap();

        assert_eq!(hash_one(&url_a), hash_one(&url_b));
    }

    #[test]
    fn test_hash_url_distinguishes_query_values() {
        let url_a: Url = "https://example.com/path?key=value1".parse().unwrap();
        let url_b: Url = "https://example.com/path?key=value2".parse().unwrap();

        assert_ne!(hash_one(&url_a), hash_one(&url_b));
    }

    #[test]
    fn test_hash_url_distinguishes_paths() {
        let url_a: Url = "https://example.com/path1".parse().unwrap();
        let url_b: Url = "https://example.com/path2".parse().unwrap();

        assert_ne!(hash_one(&url_a), hash_one(&url_b));
    }

    #[test]
    fn test_hash_url_distinguishes_hosts() {
        let url_a: Url = "https://a.example.com/path".parse().unwrap();
        let url_b: Url = "https://b.example.com/path".parse().unwrap();

        assert_ne!(hash_one(&url_a), hash_one(&url_b));
    }
}
