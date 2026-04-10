//! HMAC-SHA256 request signing for the Bitkub API.
//!
//! Both V3 and V4 endpoints use the same signing scheme:
//!
//! ```text
//! signature = HMAC-SHA256(secret, "{timestamp}{METHOD}{path}{body_or_query}")
//! ```
//!
//! For **GET** requests the query string (including the leading `?`) is appended:
//! ```text
//! 1699381086593GET/api/v3/market/my-order-history?sym=BTC_THB
//! ```
//!
//! For **POST** requests the JSON body is appended:
//! ```text
//! 1699376552354POST/api/v3/market/place-bid{"sym":"thb_btc","amt":1000,"rat":10,"typ":"limit"}
//! ```
//!
//! Private WebSocket authentication signs only the timestamp:
//! ```text
//! signature = HMAC-SHA256(secret, "{timestamp}")
//! ```

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// API credentials for authenticating with the Bitkub exchange.
///
/// # Examples
///
/// ```
/// use bitkub::auth::Credentials;
///
/// let creds = Credentials::new("my-api-key", "my-api-secret");
///
/// // Sign a REST request
/// let sig = creds.sign(1699376552354, "POST", "/api/v3/market/place-bid", r#"{"sym":"thb_btc"}"#);
/// assert!(!sig.is_empty());
///
/// // Sign a WebSocket auth message
/// let ws_sig = creds.sign_ws(1699376552354);
/// assert!(!ws_sig.is_empty());
/// ```
#[derive(Clone)]
pub struct Credentials {
    pub api_key: String,
    pub api_secret: String,
}

impl Credentials {
    /// Create a new set of credentials.
    ///
    /// Both the API key and secret are accepted as anything that converts
    /// into `String`, so `&str`, `String`, and similar types all work.
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
        }
    }

    /// Produce an HMAC-SHA256 hex signature for a REST API request.
    ///
    /// The signing string is constructed as:
    /// ```text
    /// {timestamp}{method}{path}{body}
    /// ```
    ///
    /// - For GET requests, `body` should be the query string including the
    ///   leading `?` (e.g., `"?sym=BTC_THB"`). Pass an empty string when
    ///   there are no query parameters.
    /// - For POST requests, `body` should be the JSON request body.
    pub fn sign(&self, timestamp: u64, method: &str, path: &str, body: &str) -> String {
        let data = format!("{timestamp}{method}{path}{body}");
        self.hmac_hex(&data)
    }

    /// Produce an HMAC-SHA256 hex signature for private WebSocket authentication.
    ///
    /// The payload is simply the timestamp as a string.
    pub fn sign_ws(&self, timestamp: u64) -> String {
        let data = timestamp.to_string();
        self.hmac_hex(&data)
    }

    /// Compute HMAC-SHA256 over `data` using the API secret and return the
    /// lowercase hex-encoded result.
    fn hmac_hex(&self, data: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(self.api_secret.as_bytes()).expect("HMAC accepts any key size");
        mac.update(data.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("api_key", &mask_string(&self.api_key))
            .field("api_secret", &"***")
            .finish()
    }
}

/// Mask a string for safe logging, showing at most the first 4 and last 4
/// characters.
fn mask_string(s: &str) -> String {
    if s.len() <= 8 {
        return "***".to_owned();
    }
    format!("{}***{}", &s[..4], &s[s.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_post_request() {
        let creds = Credentials::new("test-key", "test-secret");
        let sig = creds.sign(
            1699376552354,
            "POST",
            "/api/v3/market/place-bid",
            r#"{"sym":"thb_btc","amt":1000,"rat":10,"typ":"limit"}"#,
        );

        // The signature should be a 64-character hex string (256 bits).
        assert_eq!(sig.len(), 64);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sign_get_request_with_query() {
        let creds = Credentials::new("test-key", "test-secret");
        let sig = creds.sign(
            1699381086593,
            "GET",
            "/api/v3/market/my-order-history",
            "?sym=BTC_THB",
        );

        assert_eq!(sig.len(), 64);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sign_get_request_without_query() {
        let creds = Credentials::new("test-key", "test-secret");
        let sig = creds.sign(1699381086593, "GET", "/api/v3/servertime", "");

        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn sign_ws_produces_valid_hex() {
        let creds = Credentials::new("test-key", "test-secret");
        let sig = creds.sign_ws(1699376552354);

        assert_eq!(sig.len(), 64);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sign_ws_differs_from_rest_sign() {
        let creds = Credentials::new("test-key", "test-secret");
        let ts = 1699376552354u64;

        let ws_sig = creds.sign_ws(ts);
        let rest_sig = creds.sign(ts, "GET", "/api/v3/servertime", "");

        // The WS signature signs only the timestamp, so it must differ from
        // a REST signature that includes method + path.
        assert_ne!(ws_sig, rest_sig);
    }

    #[test]
    fn deterministic_signatures() {
        let creds = Credentials::new("key", "secret");
        let sig1 = creds.sign(100, "GET", "/path", "");
        let sig2 = creds.sign(100, "GET", "/path", "");
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn known_hmac_vector() {
        // Verify against a known HMAC-SHA256 computation.
        // HMAC-SHA256("secret", "hello") is a well-known value.
        let creds = Credentials::new("key", "secret");
        let result = creds.hmac_hex("hello");
        // Computed externally: HMAC-SHA256(key=b"secret", msg=b"hello")
        assert_eq!(
            result,
            "88aab3ede8d3adf94d26ab90d3bafd4a2083070c3bcce9c014ee04a443847c0b"
        );
    }

    #[test]
    fn debug_masks_credentials() {
        let creds = Credentials::new("abcdefghijklmnop", "super-secret-key-12345");
        let debug = format!("{creds:?}");
        assert!(!debug.contains("abcdefghijklmnop"));
        assert!(!debug.contains("super-secret-key-12345"));
        assert!(debug.contains("***"));
    }

    #[test]
    fn mask_short_string() {
        assert_eq!(mask_string("short"), "***");
        assert_eq!(mask_string(""), "***");
    }

    #[test]
    fn mask_long_string() {
        assert_eq!(mask_string("abcdefghijkl"), "abcd***ijkl");
    }

    #[test]
    fn v4_signing_uses_same_scheme() {
        let creds = Credentials::new("test-key", "test-secret");

        // V4 GET with query
        let sig = creds.sign(
            1699381086593,
            "GET",
            "/api/v4/crypto/addresses",
            "?symbol=ATOM",
        );
        assert_eq!(sig.len(), 64);

        // V4 POST with body
        let sig = creds.sign(
            1699376552354,
            "POST",
            "/api/v4/crypto/addresses",
            r#"{"symbol":"ATOM","network":"ATOM"}"#,
        );
        assert_eq!(sig.len(), 64);
    }
}
