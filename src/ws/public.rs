//! Public WebSocket client for trade and ticker streams.
//!
//! Connects to `wss://api.bitkub.com/websocket-api/<streams>` where `<streams>`
//! is a comma-separated list of stream names such as `market.trade.thb_btc` or
//! `market.ticker.thb_btc`.
//!
//! # Example
//!
//! ```no_run
//! use bitkub::ws::public::{PublicWsClient, PublicWsMessage};
//!
//! #[tokio::main]
//! async fn main() -> bitkub::error::Result<()> {
//!     let mut client = PublicWsClient::new(&["market.trade.thb_btc", "market.ticker.thb_btc"]);
//!     let (mut rx, mut err_rx) = client.connect().await?;
//!
//!     while let Some(msg) = rx.recv().await {
//!         match msg {
//!             PublicWsMessage::Trade(t) => println!("trade: {} @ {}", t.amount, t.rate),
//!             PublicWsMessage::Ticker(t) => println!("ticker: last={}", t.last),
//!         }
//!     }
//!     Ok(())
//! }
//! ```

use futures_util::stream::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;

use crate::error::{BitkubError, Result};
use crate::models::websocket::{TickerStreamMessage, TradeStreamMessage};

/// Default WebSocket base URL for public streams.
const PUBLIC_WS_BASE: &str = "wss://api.bitkub.com/websocket-api/";

/// Messages emitted by [`PublicWsClient`].
#[derive(Debug, Clone)]
pub enum PublicWsMessage {
    /// A trade event from a `market.trade.*` stream.
    Trade(TradeStreamMessage),
    /// A ticker event from a `market.ticker.*` stream.
    Ticker(TickerStreamMessage),
}

/// Client for subscribing to public Bitkub trade and ticker WebSocket streams.
pub struct PublicWsClient {
    url: String,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
    read_handle: Option<JoinHandle<()>>,
}

impl PublicWsClient {
    /// Create a new client for the given stream names.
    ///
    /// Stream names follow the pattern `market.trade.<symbol>` or
    /// `market.ticker.<symbol>`, e.g. `"market.trade.thb_btc"`.
    ///
    /// Multiple streams are joined by commas in the URL path.
    pub fn new(streams: &[&str]) -> Self {
        let joined = streams.join(",");
        let url = format!("{PUBLIC_WS_BASE}{joined}");
        Self {
            url,
            shutdown_tx: None,
            read_handle: None,
        }
    }

    /// Connect to the WebSocket and begin receiving messages.
    ///
    /// Returns a pair of unbounded receivers: one for parsed messages and one
    /// for errors encountered during the read loop. The read loop runs in a
    /// background task and terminates when [`disconnect`](Self::disconnect) is
    /// called or the server closes the connection.
    pub async fn connect(
        &mut self,
    ) -> Result<(
        mpsc::UnboundedReceiver<PublicWsMessage>,
        mpsc::UnboundedReceiver<BitkubError>,
    )> {
        if self.shutdown_tx.is_some() {
            return Err(BitkubError::Internal("already connected".to_owned()));
        }

        tracing::debug!(url = %self.url, "connecting to public websocket");

        let (ws_stream, _response) = connect_async(&self.url).await?;
        let (_write, read) = ws_stream.split();

        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<PublicWsMessage>();
        let (err_tx, err_rx) = mpsc::unbounded_channel::<BitkubError>();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        self.shutdown_tx = Some(shutdown_tx);

        let handle = tokio::spawn(read_loop(read, msg_tx, err_tx, shutdown_rx));
        self.read_handle = Some(handle);

        tracing::debug!("public websocket connected");
        Ok((msg_rx, err_rx))
    }

    /// Disconnect from the WebSocket.
    ///
    /// Signals the background read loop to stop and awaits its completion.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(handle) = self.read_handle.take() {
            let _ = handle.await;
        }
        tracing::debug!("public websocket disconnected");
        Ok(())
    }
}

impl Drop for PublicWsClient {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(handle) = self.read_handle.take() {
            handle.abort();
        }
    }
}

// ---------------------------------------------------------------------------
// Background read loop
// ---------------------------------------------------------------------------

/// Type alias for the read half of the WebSocket stream.
type WsReadHalf = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
>;

/// Reads messages from the WebSocket, parses them, and routes to the
/// appropriate channel.
async fn read_loop(
    mut read: WsReadHalf,
    msg_tx: mpsc::UnboundedSender<PublicWsMessage>,
    err_tx: mpsc::UnboundedSender<BitkubError>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            biased;

            _ = shutdown_rx.changed() => {
                tracing::debug!("public ws read loop received shutdown signal");
                break;
            }

            frame = read.next() => {
                match frame {
                    Some(Ok(msg)) => {
                        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                            parse_and_route(&text, &msg_tx, &err_tx);
                        }
                        // Ping/Pong/Binary/Close frames are handled by tungstenite
                        // automatically or are not relevant for public streams.
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "public ws read error");
                        let _ = err_tx.send(BitkubError::WebSocket(e));
                        break;
                    }
                    None => {
                        tracing::debug!("public ws stream ended");
                        break;
                    }
                }
            }
        }
    }
}

/// Parse a raw JSON text frame and send it to the message channel.
fn parse_and_route(
    text: &str,
    msg_tx: &mpsc::UnboundedSender<PublicWsMessage>,
    err_tx: &mpsc::UnboundedSender<BitkubError>,
) {
    // Peek at the `stream` field to decide the message type.
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, raw = %text, "failed to parse public ws message as JSON");
            let _ = err_tx.send(BitkubError::Json(e));
            return;
        }
    };

    let stream = match value.get("stream").and_then(|s| s.as_str()) {
        Some(s) => s,
        None => {
            tracing::debug!(raw = %text, "public ws message missing 'stream' field, ignoring");
            return;
        }
    };

    if stream.starts_with("market.trade.") {
        match serde_json::from_value::<TradeStreamMessage>(value) {
            Ok(trade) => {
                let _ = msg_tx.send(PublicWsMessage::Trade(trade));
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to deserialize trade stream message");
                let _ = err_tx.send(BitkubError::Json(e));
            }
        }
    } else if stream.starts_with("market.ticker.") {
        match serde_json::from_value::<TickerStreamMessage>(value) {
            Ok(ticker) => {
                let _ = msg_tx.send(PublicWsMessage::Ticker(ticker));
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to deserialize ticker stream message");
                let _ = err_tx.send(BitkubError::Json(e));
            }
        }
    } else {
        tracing::debug!(stream = %stream, "unknown public ws stream type, ignoring");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_construction_single_stream() {
        let client = PublicWsClient::new(&["market.trade.thb_btc"]);
        assert_eq!(
            client.url,
            "wss://api.bitkub.com/websocket-api/market.trade.thb_btc"
        );
    }

    #[test]
    fn url_construction_multiple_streams() {
        let client = PublicWsClient::new(&[
            "market.trade.thb_btc",
            "market.ticker.thb_btc",
            "market.trade.thb_eth",
        ]);
        assert_eq!(
            client.url,
            "wss://api.bitkub.com/websocket-api/market.trade.thb_btc,market.ticker.thb_btc,market.trade.thb_eth"
        );
    }

    #[test]
    fn parse_trade_message() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"stream":"market.trade.thb_btc","sym":"THB_BTC","txn":"SELL","rat":"1000000","amt":"0.001","bid":"123","sid":"456","ts":1609459200}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            PublicWsMessage::Trade(t) => {
                assert_eq!(t.symbol, "THB_BTC");
                assert_eq!(t.txn, "SELL");
            }
            _ => panic!("expected Trade variant"),
        }
    }

    #[test]
    fn parse_ticker_message() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"stream":"market.ticker.thb_btc","id":1,"last":"1000000","lowestAsk":"1000001","lowestAskSize":"5","highestBid":"999999","highestBidSize":"10","change":"15000","percentChange":"1.5","baseVolume":"100.5","quoteVolume":"100500000","isFrozen":0,"high24hr":"1050000","low24hr":"950000","open":"985000","close":"1000000"}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            PublicWsMessage::Ticker(t) => {
                assert_eq!(t.id, 1);
            }
            _ => panic!("expected Ticker variant"),
        }
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
    fn parse_unknown_stream_is_ignored() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, mut err_rx) = mpsc::unbounded_channel();

        let json = r#"{"stream":"market.unknown.thb_btc","data":{}}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        assert!(msg_rx.try_recv().is_err(), "no message should be emitted");
        assert!(err_rx.try_recv().is_err(), "no error should be emitted");
    }

    #[test]
    fn parse_missing_stream_field_is_ignored() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, mut err_rx) = mpsc::unbounded_channel();

        let json = r#"{"data":"something"}"#;
        parse_and_route(json, &msg_tx, &err_tx);

        assert!(msg_rx.try_recv().is_err());
        assert!(err_rx.try_recv().is_err());
    }
}
