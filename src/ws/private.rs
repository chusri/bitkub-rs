//! Private WebSocket client for authenticated order and match updates.
//!
//! Connects to `wss://stream.bitkub.com/v3/private`, authenticates using
//! HMAC-SHA256 signed credentials, and subscribes to `order_update` and/or
//! `match_update` channels.
//!
//! # Constraints
//!
//! - A ping must be sent every 4 minutes to keep the connection alive.
//! - Maximum connection duration is 2 hours.
//! - Maximum 5 concurrent connections per API key.
//! - The `User-Agent` header is required.
//!
//! # Example
//!
//! ```no_run
//! use bitkub::auth::Credentials;
//! use bitkub::ws::private::{PrivateWsClient, PrivateWsMessage};
//!
//! #[tokio::main]
//! async fn main() -> bitkub::error::Result<()> {
//!     let creds = Credentials::new("my-api-key", "my-api-secret");
//!     let mut client = PrivateWsClient::new(creds);
//!     let (mut rx, mut err_rx) = client
//!         .connect(&["order_update", "match_update"])
//!         .await?;
//!
//!     while let Some(msg) = rx.recv().await {
//!         match msg {
//!             PrivateWsMessage::OrderUpdate(o) => {
//!                 println!("order {}: {}", o.order_id, o.status);
//!             }
//!             PrivateWsMessage::MatchUpdate(m) => {
//!                 println!("match {}: {} @ {}", m.txn_id, m.executed_amount, m.price);
//!             }
//!             PrivateWsMessage::Authenticated => println!("authenticated"),
//!             PrivateWsMessage::Subscribed(ch) => println!("subscribed to {ch}"),
//!             PrivateWsMessage::Pong => {}
//!         }
//!     }
//!     Ok(())
//! }
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::http::Request;
use tokio_tungstenite::tungstenite::Message;

use crate::auth::Credentials;
use crate::error::{BitkubError, Result};
use crate::models::websocket::{MatchUpdate, OrderUpdate};

/// Private WebSocket endpoint.
const PRIVATE_WS_URL: &str = "wss://stream.bitkub.com/v3/private";

/// Ping interval in seconds (4 minutes).
const PING_INTERVAL_SECS: u64 = 240;

/// User-Agent string sent in the WebSocket handshake.
const USER_AGENT: &str = "bitkub-rs/0.1.0";

/// Messages emitted by [`PrivateWsClient`].
#[derive(Debug, Clone)]
pub enum PrivateWsMessage {
    /// An order status update.
    OrderUpdate(OrderUpdate),
    /// A trade fill (match) update.
    MatchUpdate(MatchUpdate),
    /// Authentication was successful.
    Authenticated,
    /// Successfully subscribed to the given channel.
    Subscribed(String),
    /// Server acknowledged our ping.
    Pong,
}

/// Client for the Bitkub private WebSocket API.
pub struct PrivateWsClient {
    credentials: Credentials,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
    read_handle: Option<JoinHandle<()>>,
    ping_handle: Option<JoinHandle<()>>,
    writer_handle: Option<JoinHandle<()>>,
}

impl PrivateWsClient {
    /// Create a new private WebSocket client with the given credentials.
    pub fn new(credentials: Credentials) -> Self {
        Self {
            credentials,
            shutdown_tx: None,
            read_handle: None,
            ping_handle: None,
            writer_handle: None,
        }
    }

    /// Connect, authenticate, and subscribe to the given channels.
    ///
    /// Valid channel names are `"order_update"` and `"match_update"`.
    ///
    /// Returns a pair of unbounded receivers: one for parsed messages and one
    /// for errors. The client spawns background tasks for reading, writing
    /// (forwarding outbound messages), and pinging (every 240 seconds).
    pub async fn connect(
        &mut self,
        channels: &[&str],
    ) -> Result<(
        mpsc::UnboundedReceiver<PrivateWsMessage>,
        mpsc::UnboundedReceiver<BitkubError>,
    )> {
        if self.shutdown_tx.is_some() {
            return Err(BitkubError::Internal("already connected".to_owned()));
        }

        tracing::debug!("connecting to private websocket");

        // Build the HTTP request with the User-Agent header.
        let request = Request::builder()
            .uri(PRIVATE_WS_URL)
            .header("User-Agent", USER_AGENT)
            .header("Host", "stream.bitkub.com")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .body(())
            .map_err(|e| BitkubError::Internal(format!("failed to build request: {e}")))?;

        let (ws_stream, _response) =
            tokio_tungstenite::connect_async(request).await?;
        let (mut write, read) = ws_stream.split();

        // --- Authenticate ---
        let timestamp = current_timestamp_ms();
        let signature = self.credentials.sign_ws(timestamp);

        let auth_msg = serde_json::json!({
            "event": "auth",
            "data": {
                "X-BTK-APIKEY": self.credentials.api_key,
                "X-BTK-SIGN": signature,
                "X-BTK-TIMESTAMP": timestamp.to_string(),
            }
        });

        tracing::debug!("sending auth message");
        write
            .send(Message::Text(auth_msg.to_string().into()))
            .await
            .map_err(BitkubError::WebSocket)?;

        // --- Subscribe to channels ---
        for channel in channels {
            let sub_msg = serde_json::json!({
                "event": "subscribe",
                "channel": channel,
            });
            tracing::debug!(channel = %channel, "subscribing to channel");
            write
                .send(Message::Text(sub_msg.to_string().into()))
                .await
                .map_err(BitkubError::WebSocket)?;
        }

        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<PrivateWsMessage>();
        let (err_tx, err_rx) = mpsc::unbounded_channel::<BitkubError>();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        self.shutdown_tx = Some(shutdown_tx);

        // --- Spawn writer task ---
        // The ping task (and any future outbound messages) send through this
        // channel, and the writer task forwards them to the WS write half.
        // This avoids contention over the write half.
        let (write_tx, write_rx) = mpsc::unbounded_channel::<Message>();
        let writer_shutdown_rx = self
            .shutdown_tx
            .as_ref()
            .expect("shutdown_tx was just set")
            .subscribe();
        let writer_handle = tokio::spawn(write_loop(write, write_rx, writer_shutdown_rx));
        self.writer_handle = Some(writer_handle);

        // --- Spawn ping task ---
        let ping_shutdown_rx = self
            .shutdown_tx
            .as_ref()
            .expect("shutdown_tx was just set")
            .subscribe();
        let ping_handle = tokio::spawn(ping_loop(write_tx, ping_shutdown_rx));
        self.ping_handle = Some(ping_handle);

        // --- Spawn read loop ---
        let handle = tokio::spawn(read_loop(read, msg_tx, err_tx, shutdown_rx));
        self.read_handle = Some(handle);

        tracing::debug!("private websocket connected and authenticated");
        Ok((msg_rx, err_rx))
    }

    /// Disconnect from the private WebSocket.
    ///
    /// Signals all background tasks to stop and awaits their completion.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(handle) = self.read_handle.take() {
            let _ = handle.await;
        }
        if let Some(handle) = self.ping_handle.take() {
            let _ = handle.await;
        }
        if let Some(handle) = self.writer_handle.take() {
            let _ = handle.await;
        }
        tracing::debug!("private websocket disconnected");
        Ok(())
    }
}

impl Drop for PrivateWsClient {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(handle) = self.read_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.ping_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.writer_handle.take() {
            handle.abort();
        }
    }
}

// ---------------------------------------------------------------------------
// Background tasks
// ---------------------------------------------------------------------------

type WsReadHalf = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
>;

type WsWriteHalf = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    Message,
>;

/// Reads messages from the WebSocket and routes them to the appropriate
/// channel based on the `event` field.
async fn read_loop(
    mut read: WsReadHalf,
    msg_tx: mpsc::UnboundedSender<PrivateWsMessage>,
    err_tx: mpsc::UnboundedSender<BitkubError>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            biased;

            _ = shutdown_rx.changed() => {
                tracing::debug!("private ws read loop received shutdown signal");
                break;
            }

            frame = read.next() => {
                match frame {
                    Some(Ok(msg)) => {
                        if let Message::Text(text) = msg {
                            parse_and_route(&text, &msg_tx, &err_tx);
                        }
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "private ws read error");
                        let _ = err_tx.send(BitkubError::WebSocket(e));
                        break;
                    }
                    None => {
                        tracing::debug!("private ws stream ended");
                        break;
                    }
                }
            }
        }
    }
}

/// Forwards messages from the internal channel to the WebSocket write half.
async fn write_loop(
    mut write: WsWriteHalf,
    mut write_rx: mpsc::UnboundedReceiver<Message>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            biased;

            _ = shutdown_rx.changed() => {
                tracing::debug!("private ws write loop received shutdown signal");
                // Attempt a graceful close.
                let _ = write.send(Message::Close(None)).await;
                break;
            }

            msg = write_rx.recv() => {
                match msg {
                    Some(m) => {
                        if let Err(e) = write.send(m).await {
                            tracing::warn!(error = %e, "private ws write error");
                            break;
                        }
                    }
                    None => {
                        tracing::debug!("private ws write channel closed");
                        break;
                    }
                }
            }
        }
    }
}

/// Sends a ping message every [`PING_INTERVAL_SECS`] seconds.
async fn ping_loop(
    write_tx: mpsc::UnboundedSender<Message>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    let interval = tokio::time::Duration::from_secs(PING_INTERVAL_SECS);

    loop {
        tokio::select! {
            biased;

            _ = shutdown_rx.changed() => {
                tracing::debug!("private ws ping loop received shutdown signal");
                break;
            }

            _ = tokio::time::sleep(interval) => {
                let ping_msg = serde_json::json!({"event": "ping"});
                tracing::debug!("sending ping to private ws");
                if write_tx.send(Message::Text(ping_msg.to_string().into())).is_err() {
                    tracing::debug!("private ws write channel closed, stopping ping loop");
                    break;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Message parsing
// ---------------------------------------------------------------------------

/// Intermediate structure for peeking at the event type.
#[derive(Deserialize)]
struct RawPrivateEvent {
    event: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    data: serde_json::Value,
}

fn parse_and_route(
    text: &str,
    msg_tx: &mpsc::UnboundedSender<PrivateWsMessage>,
    err_tx: &mpsc::UnboundedSender<BitkubError>,
) {
    let raw: RawPrivateEvent = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, raw = %text, "failed to parse private ws message");
            let _ = err_tx.send(BitkubError::Json(e));
            return;
        }
    };

    match raw.event.as_str() {
        "auth" => {
            // Auth response: check for success (code "0" or "200").
            let code = raw.code.as_deref().unwrap_or("");
            if code == "0" || code == "200" {
                tracing::debug!("private ws authentication successful");
                let _ = msg_tx.send(PrivateWsMessage::Authenticated);
            } else {
                let message = raw.message.unwrap_or_else(|| "auth failed".to_owned());
                tracing::warn!(code = %code, message = %message, "private ws auth failed");
                let _ = err_tx.send(BitkubError::Auth(format!("code {code}: {message}")));
            }
        }

        "subscribe" => {
            let code = raw.code.as_deref().unwrap_or("");
            let channel = raw.channel.unwrap_or_default();
            if code == "0" || code == "200" {
                tracing::debug!(channel = %channel, "subscribed to private ws channel");
                let _ = msg_tx.send(PrivateWsMessage::Subscribed(channel));
            } else {
                let message = raw.message.unwrap_or_else(|| "subscribe failed".to_owned());
                tracing::warn!(
                    code = %code,
                    channel = %channel,
                    message = %message,
                    "private ws subscribe failed"
                );
                let _ = err_tx.send(BitkubError::Internal(format!(
                    "subscribe to '{channel}' failed: code {code}: {message}"
                )));
            }
        }

        "order_update" | "order.update" => {
            match serde_json::from_value::<OrderUpdate>(raw.data) {
                Ok(update) => {
                    let _ = msg_tx.send(PrivateWsMessage::OrderUpdate(update));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to parse order_update data");
                    let _ = err_tx.send(BitkubError::Json(e));
                }
            }
        }

        "match_update" | "match.update" => {
            match serde_json::from_value::<MatchUpdate>(raw.data) {
                Ok(update) => {
                    let _ = msg_tx.send(PrivateWsMessage::MatchUpdate(update));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to parse match_update data");
                    let _ = err_tx.send(BitkubError::Json(e));
                }
            }
        }

        "pong" => {
            tracing::debug!("private ws pong received");
            let _ = msg_tx.send(PrivateWsMessage::Pong);
        }

        other => {
            tracing::debug!(event = %other, "unknown private ws event, ignoring");
        }
    }
}

/// Get the current Unix timestamp in milliseconds.
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before unix epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_auth_success() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"auth","code":"0","message":"success","data":{}}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        assert!(matches!(msg, PrivateWsMessage::Authenticated));
    }

    #[test]
    fn parse_auth_success_code_200() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"auth","code":"200","message":"success","data":{}}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        assert!(matches!(msg, PrivateWsMessage::Authenticated));
    }

    #[test]
    fn parse_auth_failure() {
        let (msg_tx, _msg_rx) = mpsc::unbounded_channel();
        let (err_tx, mut err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"auth","code":"401","message":"invalid api key","data":{}}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let err = err_rx.try_recv().expect("should receive an error");
        assert!(matches!(err, BitkubError::Auth(_)));
    }

    #[test]
    fn parse_subscribe_success() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"subscribe","code":"0","message":"success","channel":"order_update","data":{}}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            PrivateWsMessage::Subscribed(ch) => assert_eq!(ch, "order_update"),
            _ => panic!("expected Subscribed variant"),
        }
    }

    #[test]
    fn parse_subscribe_failure() {
        let (msg_tx, _msg_rx) = mpsc::unbounded_channel();
        let (err_tx, mut err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"subscribe","code":"400","message":"bad channel","channel":"invalid","data":{}}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let err = err_rx.try_recv().expect("should receive an error");
        assert!(matches!(err, BitkubError::Internal(_)));
    }

    #[test]
    fn parse_pong() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"pong"}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        assert!(matches!(msg, PrivateWsMessage::Pong));
    }

    #[test]
    fn parse_order_update() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"order_update","code":"0","message":"","data":{"user_id":"u1","order_id":"o1","client_id":null,"symbol":"THB_BTC","side":"buy","type":"limit","status":"open","price":"1000000","stop_price":null,"order_currency":"THB","order_amount":"1000","executed_currency":"THB","executed_amount":"0","received_currency":"BTC","received_amount":"0","total_fee":"0","credit_used":"0","net_fee_paid":"0","avg_filled_price":null,"post_only":false,"canceled_by":null,"order_created_at":1609459200,"order_triggered_at":null,"order_updated_at":null}}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            PrivateWsMessage::OrderUpdate(o) => {
                assert_eq!(o.order_id, "o1");
                assert_eq!(o.status, "open");
                assert_eq!(o.side, "buy");
            }
            _ => panic!("expected OrderUpdate variant"),
        }
    }

    #[test]
    fn parse_match_update() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"match_update","code":"0","message":"","data":{"order_id":"o1","txn_id":"t1","client_id":null,"symbol":"THB_BTC","type":"limit","status":"filled","side":"buy","is_maker":true,"price":"1000000","executed_currency":"THB","executed_amount":"1000","received_currency":"BTC","received_amount":"0.001","fee_rate":"0.0025","total_fee":"2.5","credit_used":"0","net_fee_paid":"2.5","txn_ts":1609459200}}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            PrivateWsMessage::MatchUpdate(m) => {
                assert_eq!(m.txn_id, "t1");
                assert!(m.is_maker);
            }
            _ => panic!("expected MatchUpdate variant"),
        }
    }

    #[test]
    fn parse_order_update_dot_notation() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"order.update","code":"0","message":"","data":{"user_id":"u1","order_id":"o2","client_id":"c1","symbol":"THB_ETH","side":"sell","type":"market","status":"filled","price":null,"stop_price":null,"order_currency":"ETH","order_amount":"1","executed_currency":"ETH","executed_amount":"1","received_currency":"THB","received_amount":"50000","total_fee":"125","credit_used":"0","net_fee_paid":"125","avg_filled_price":"50000","post_only":false,"canceled_by":null,"order_created_at":1609459300,"order_triggered_at":null,"order_updated_at":1609459301}}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            PrivateWsMessage::OrderUpdate(o) => {
                assert_eq!(o.order_id, "o2");
                assert_eq!(o.client_id.as_deref(), Some("c1"));
            }
            _ => panic!("expected OrderUpdate variant"),
        }
    }

    #[test]
    fn parse_unknown_event_is_ignored() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, mut err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"heartbeat"}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        assert!(msg_rx.try_recv().is_err());
        assert!(err_rx.try_recv().is_err());
    }

    #[test]
    fn parse_invalid_json_sends_error() {
        let (msg_tx, _msg_rx) = mpsc::unbounded_channel();
        let (err_tx, mut err_rx) = mpsc::unbounded_channel();

        parse_and_route("not-json", &msg_tx, &err_tx);

        let err = err_rx.try_recv().expect("should receive an error");
        assert!(matches!(err, BitkubError::Json(_)));
    }

    #[test]
    fn timestamp_is_reasonable() {
        let ts = current_timestamp_ms();
        // Should be after 2020-01-01 in milliseconds.
        assert!(ts > 1_577_836_800_000);
    }

    #[test]
    fn client_creation() {
        let creds = Credentials::new("test-key", "test-secret");
        let client = PrivateWsClient::new(creds);
        assert!(client.shutdown_tx.is_none());
        assert!(client.read_handle.is_none());
        assert!(client.ping_handle.is_none());
        assert!(client.writer_handle.is_none());
    }
}
