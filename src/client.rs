//! REST client for the Bitkub exchange API.
//!
//! [`BitkubClient`] is the main entry point for interacting with both public
//! and authenticated (private) endpoints. Use [`BitkubClientBuilder`] to
//! configure the client before construction.
//!
//! # Examples
//!
//! ```no_run
//! use bitkub::BitkubClient;
//! use std::time::Duration;
//!
//! # async fn example() -> bitkub::Result<()> {
//! // Public-only client (no credentials).
//! let client = BitkubClient::new();
//!
//! // Fully configured client via the builder.
//! let client = BitkubClient::builder()
//!     .with_credentials("my-api-key", "my-api-secret")
//!     .with_timeout(Duration::from_secs(5))
//!     .build()?;
//! # Ok(())
//! # }
//! ```

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::{debug, warn};

use crate::auth::Credentials;
use crate::error::{map_api_error, BitkubError, Result};

/// Default Bitkub REST API base URL.
const DEFAULT_BASE_URL: &str = "https://api.bitkub.com";

/// Default HTTP request timeout.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Server time endpoint used to obtain the timestamp for request signing.
const SERVER_TIME_PATH: &str = "/api/v3/servertime";

// ---------------------------------------------------------------------------
// V3 response envelope
// ---------------------------------------------------------------------------

/// The standard V3 API response envelope.
///
/// ```json
/// {"error": 0, "result": { ... }}
/// ```
#[derive(serde::Deserialize)]
struct V3Response {
    error: i32,
    result: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// V4 response envelope
// ---------------------------------------------------------------------------

/// The standard V4 API response envelope.
///
/// ```json
/// {"code": "0", "message": "success", "data": { ... }}
/// ```
#[derive(serde::Deserialize)]
struct V4Response {
    code: String,
    message: String,
    data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// BitkubClient
// ---------------------------------------------------------------------------

/// Asynchronous REST client for the Bitkub cryptocurrency exchange.
///
/// Obtain an instance via [`BitkubClient::new`] for public-only access or
/// [`BitkubClient::builder`] for full configuration including credentials.
#[derive(Debug, Clone)]
pub struct BitkubClient {
    http: reqwest::Client,
    base_url: String,
    credentials: Option<Credentials>,
}

impl BitkubClient {
    /// Create a public-only client with default settings and no credentials.
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .build()
                .expect("default reqwest client should build"),
            base_url: DEFAULT_BASE_URL.to_owned(),
            credentials: None,
        }
    }

    /// Start building a client with custom configuration.
    pub fn builder() -> BitkubClientBuilder {
        BitkubClientBuilder::default()
    }

    /// Return the configured base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Return a reference to the credentials, if configured.
    pub fn credentials(&self) -> Option<&Credentials> {
        self.credentials.as_ref()
    }

    // -----------------------------------------------------------------------
    // Public (unsigned) helpers
    // -----------------------------------------------------------------------

    /// Send an unsigned GET request and deserialize the JSON response body
    /// directly (no envelope unwrapping).
    pub(crate) async fn get_raw<T: DeserializeOwned, Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!(method = "GET", path, "public request");

        let resp = self
            .http
            .get(&url)
            .query(query)
            .header(ACCEPT, "application/json")
            .send()
            .await?;

        Self::check_status(&resp)?;

        let body = resp.json::<T>().await?;
        Ok(body)
    }

    /// Send an unsigned GET request expecting a V3 envelope.
    /// Returns the deserialized `result` field.
    pub(crate) async fn get<T: DeserializeOwned, Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!(method = "GET", path, "public v3 request");

        let resp = self
            .http
            .get(&url)
            .query(query)
            .header(ACCEPT, "application/json")
            .send()
            .await?;

        Self::check_status(&resp)?;

        let envelope: V3Response = resp.json().await?;
        Self::unwrap_v3(envelope)
    }

    /// Send an unsigned POST request expecting a V3 envelope.
    pub(crate) async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!(method = "POST", path, "public v3 request");

        let resp = self
            .http
            .post(&url)
            .json(body)
            .header(ACCEPT, "application/json")
            .send()
            .await?;

        Self::check_status(&resp)?;

        let envelope: V3Response = resp.json().await?;
        Self::unwrap_v3(envelope)
    }

    // -----------------------------------------------------------------------
    // Signed (private) helpers -- V3
    // -----------------------------------------------------------------------

    /// Send a signed GET request expecting a V3 envelope.
    ///
    /// The `query` parameter accepts any type that implements `Serialize`,
    /// such as `&[(&str, &str)]` or `&[(&str, String)]`.
    pub(crate) async fn get_secure<T: DeserializeOwned, Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T> {
        let creds = self.require_credentials()?;
        let timestamp = self.server_time().await?;

        // Serialize the query to a URL-encoded string for signing.
        let query_string = serde_urlencoded::to_string(query).unwrap_or_default();
        let sign_payload = if query_string.is_empty() {
            String::new()
        } else {
            format!("?{query_string}")
        };

        let signature = creds.sign(timestamp, "GET", path, &sign_payload);
        let headers = self.auth_headers(creds, timestamp, &signature);

        let url = format!("{}{}", self.base_url, path);
        debug!(method = "GET", path, "secure v3 request");

        let resp = self
            .http
            .get(&url)
            .query(query)
            .headers(headers)
            .send()
            .await?;

        Self::check_status(&resp)?;

        let envelope: V3Response = resp.json().await?;
        Self::unwrap_v3(envelope)
    }

    /// Send a signed POST request expecting a V3 envelope.
    pub(crate) async fn post_secure<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let creds = self.require_credentials()?;
        let timestamp = self.server_time().await?;

        let body_json = serde_json::to_string(body).map_err(BitkubError::Json)?;
        let signature = creds.sign(timestamp, "POST", path, &body_json);
        let headers = self.auth_headers(creds, timestamp, &signature);

        let url = format!("{}{}", self.base_url, path);
        debug!(method = "POST", path, "secure v3 request");

        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .header(CONTENT_TYPE, "application/json")
            .body(body_json)
            .send()
            .await?;

        Self::check_status(&resp)?;

        let envelope: V3Response = resp.json().await?;
        Self::unwrap_v3(envelope)
    }

    // -----------------------------------------------------------------------
    // Signed (private) helpers -- V4
    // -----------------------------------------------------------------------

    /// Send a signed GET request expecting a V4 envelope.
    pub(crate) async fn get_v4<T: DeserializeOwned, Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T> {
        let creds = self.require_credentials()?;
        let timestamp = self.server_time().await?;

        let query_string = serde_urlencoded::to_string(query).unwrap_or_default();
        let sign_payload = if query_string.is_empty() {
            String::new()
        } else {
            format!("?{query_string}")
        };

        let signature = creds.sign(timestamp, "GET", path, &sign_payload);
        let headers = self.auth_headers(creds, timestamp, &signature);

        let url = format!("{}{}", self.base_url, path);
        debug!(method = "GET", path, "secure v4 request");

        let resp = self
            .http
            .get(&url)
            .query(query)
            .headers(headers)
            .send()
            .await?;

        Self::check_status(&resp)?;

        let envelope: V4Response = resp.json().await?;
        Self::unwrap_v4(envelope)
    }

    /// Send a signed POST request expecting a V4 envelope.
    pub(crate) async fn post_v4<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let creds = self.require_credentials()?;
        let timestamp = self.server_time().await?;

        let body_json = serde_json::to_string(body).map_err(BitkubError::Json)?;
        let signature = creds.sign(timestamp, "POST", path, &body_json);
        let headers = self.auth_headers(creds, timestamp, &signature);

        let url = format!("{}{}", self.base_url, path);
        debug!(method = "POST", path, "secure v4 request");

        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .header(CONTENT_TYPE, "application/json")
            .body(body_json)
            .send()
            .await?;

        Self::check_status(&resp)?;

        let envelope: V4Response = resp.json().await?;
        Self::unwrap_v4(envelope)
    }

    // -----------------------------------------------------------------------
    // Internal utilities
    // -----------------------------------------------------------------------

    /// Fetch the server timestamp (milliseconds since epoch) from the Bitkub
    /// API. This is used as the timestamp component of request signatures.
    async fn server_time(&self) -> Result<u64> {
        let url = format!("{}{}", self.base_url, SERVER_TIME_PATH);
        let resp = self
            .http
            .get(&url)
            .header(ACCEPT, "application/json")
            .send()
            .await?;

        Self::check_status(&resp)?;

        let text = resp.text().await?;
        let ts: u64 = text.trim().parse().map_err(|_| {
            BitkubError::Internal(format!("failed to parse server time: {text}"))
        })?;

        debug!(server_time = ts, "obtained server time");
        Ok(ts)
    }

    /// Check the HTTP status code and return an appropriate error for
    /// non-success responses.
    fn check_status(resp: &reqwest::Response) -> Result<()> {
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("rate limited by Bitkub API");
            return Err(BitkubError::RateLimited);
        }
        // We do not eagerly fail on other non-2xx codes here because the
        // Bitkub API sometimes returns error details in a 200 body. The
        // envelope unwrap methods handle those cases.
        Ok(())
    }

    /// Require that credentials are configured, returning an error if not.
    fn require_credentials(&self) -> Result<&Credentials> {
        self.credentials
            .as_ref()
            .ok_or_else(|| BitkubError::Auth("credentials are required for this endpoint".into()))
    }

    /// Build the authentication headers for a signed request.
    fn auth_headers(
        &self,
        creds: &Credentials,
        timestamp: u64,
        signature: &str,
    ) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-BTK-APIKEY",
            HeaderValue::from_str(&creds.api_key).expect("API key should be valid header value"),
        );
        headers.insert(
            "X-BTK-TIMESTAMP",
            HeaderValue::from_str(&timestamp.to_string())
                .expect("timestamp should be valid header value"),
        );
        headers.insert(
            "X-BTK-SIGN",
            HeaderValue::from_str(signature).expect("signature should be valid header value"),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers
    }

    /// Unwrap a V3 response envelope, returning the `result` field or an
    /// appropriate error.
    fn unwrap_v3<T: DeserializeOwned>(envelope: V3Response) -> Result<T> {
        if envelope.error != 0 {
            return Err(map_api_error(envelope.error));
        }
        match envelope.result {
            Some(value) => serde_json::from_value(value).map_err(BitkubError::Json),
            None => Err(BitkubError::Internal(
                "V3 response has error=0 but no result field".into(),
            )),
        }
    }

    /// Unwrap a V4 response envelope, returning the `data` field or an
    /// appropriate error.
    fn unwrap_v4<T: DeserializeOwned>(envelope: V4Response) -> Result<T> {
        if envelope.code != "0" {
            return Err(BitkubError::ApiV4 {
                code: envelope.code,
                message: envelope.message,
            });
        }
        match envelope.data {
            Some(value) => serde_json::from_value(value).map_err(BitkubError::Json),
            None => Err(BitkubError::Internal(
                "V4 response has code=\"0\" but no data field".into(),
            )),
        }
    }
}

impl Default for BitkubClient {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for constructing a [`BitkubClient`] with custom configuration.
///
/// # Examples
///
/// ```no_run
/// use bitkub::BitkubClient;
/// use std::time::Duration;
///
/// # fn example() -> bitkub::Result<()> {
/// let client = BitkubClient::builder()
///     .with_credentials("api-key", "api-secret")
///     .with_base_url("https://api.bitkub.com")
///     .with_timeout(Duration::from_secs(30))
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct BitkubClientBuilder {
    credentials: Option<Credentials>,
    base_url: Option<String>,
    timeout: Option<Duration>,
}

impl BitkubClientBuilder {
    /// Set the API key and secret for authenticated endpoints.
    pub fn with_credentials(
        mut self,
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
    ) -> Self {
        self.credentials = Some(Credentials::new(api_key, api_secret));
        self
    }

    /// Override the default base URL.
    ///
    /// The URL should not have a trailing slash.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the HTTP request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Consume the builder and construct a [`BitkubClient`].
    ///
    /// Returns an error if the HTTP client cannot be built.
    pub fn build(self) -> Result<BitkubClient> {
        let timeout = self.timeout.unwrap_or(DEFAULT_TIMEOUT);

        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(BitkubError::Http)?;

        let base_url = self
            .base_url
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());

        // Strip trailing slash to avoid double-slash when joining paths.
        let base_url = base_url.trim_end_matches('/').to_owned();

        Ok(BitkubClient {
            http,
            base_url,
            credentials: self.credentials,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_client_has_correct_base_url() {
        let client = BitkubClient::new();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn default_client_has_no_credentials() {
        let client = BitkubClient::new();
        assert!(client.credentials().is_none());
    }

    #[test]
    fn builder_sets_credentials() {
        let client = BitkubClient::builder()
            .with_credentials("key", "secret")
            .build()
            .unwrap();

        let creds = client.credentials().expect("should have credentials");
        assert_eq!(creds.api_key, "key");
        assert_eq!(creds.api_secret, "secret");
    }

    #[test]
    fn builder_sets_base_url() {
        let client = BitkubClient::builder()
            .with_base_url("https://custom.example.com")
            .build()
            .unwrap();

        assert_eq!(client.base_url(), "https://custom.example.com");
    }

    #[test]
    fn builder_strips_trailing_slash() {
        let client = BitkubClient::builder()
            .with_base_url("https://custom.example.com/")
            .build()
            .unwrap();

        assert_eq!(client.base_url(), "https://custom.example.com");
    }

    #[test]
    fn require_credentials_fails_without_creds() {
        let client = BitkubClient::new();
        let result = client.require_credentials();
        assert!(result.is_err());
        match result.unwrap_err() {
            BitkubError::Auth(msg) => {
                assert!(msg.contains("credentials are required"));
            }
            other => panic!("unexpected error variant: {other}"),
        }
    }

    #[test]
    fn unwrap_v3_success() {
        let envelope = V3Response {
            error: 0,
            result: Some(serde_json::json!(42)),
        };
        let value: i32 = BitkubClient::unwrap_v3(envelope).unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn unwrap_v3_api_error() {
        let envelope = V3Response {
            error: 18,
            result: None,
        };
        let err = BitkubClient::unwrap_v3::<i32>(envelope).unwrap_err();
        match err {
            BitkubError::Api { code, message } => {
                assert_eq!(code, 18);
                assert_eq!(message, "insufficient balance");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn unwrap_v3_missing_result() {
        let envelope = V3Response {
            error: 0,
            result: None,
        };
        let err = BitkubClient::unwrap_v3::<i32>(envelope).unwrap_err();
        assert!(matches!(err, BitkubError::Internal(_)));
    }

    #[test]
    fn unwrap_v4_success() {
        let envelope = V4Response {
            code: "0".to_owned(),
            message: "success".to_owned(),
            data: Some(serde_json::json!({"key": "value"})),
        };
        let value: serde_json::Value = BitkubClient::unwrap_v4(envelope).unwrap();
        assert_eq!(value["key"], "value");
    }

    #[test]
    fn unwrap_v4_api_error() {
        let envelope = V4Response {
            code: "B1003-CW".to_owned(),
            message: "insufficient balance".to_owned(),
            data: None,
        };
        let err = BitkubClient::unwrap_v4::<serde_json::Value>(envelope).unwrap_err();
        match err {
            BitkubError::ApiV4 { code, message } => {
                assert_eq!(code, "B1003-CW");
                assert_eq!(message, "insufficient balance");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn unwrap_v4_missing_data() {
        let envelope = V4Response {
            code: "0".to_owned(),
            message: "success".to_owned(),
            data: None,
        };
        let err = BitkubClient::unwrap_v4::<serde_json::Value>(envelope).unwrap_err();
        assert!(matches!(err, BitkubError::Internal(_)));
    }
}
