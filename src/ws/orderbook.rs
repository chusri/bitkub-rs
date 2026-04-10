//! Live order book WebSocket client.
//!
//! Connects to `wss://api.bitkub.com/websocket-api/orderbook/<symbol_id>` and
//! receives real-time order book events including bid/ask changes, trades, and
//! ticker updates.
//!
//! # Example
//!
//! ```no_run
//! use bitkub::ws::orderbook::{OrderBookClient, OrderBookMessage};
//!
//! #[tokio::main]
//! async fn main() -> bitkub::error::Result<()> {
//!     // symbol_id 1 = THB_BTC
//!     let mut client = OrderBookClient::new(1);
//!     let (mut rx, mut err_rx) = client.connect().await?;
//!
//!     while let Some(msg) = rx.recv().await {
//!         match msg {
//!             OrderBookMessage::BidsChanged(e) => {
//!                 println!("bids updated: {} levels", e.data.len());
//!             }
//!             OrderBookMessage::AsksChanged(e) => {
//!                 println!("asks updated: {} levels", e.data.len());
//!             }
//!             OrderBookMessage::TradesChanged(e) => {
//!                 println!("trades: {} new", e.trades.len());
//!             }
//!             OrderBookMessage::Ticker(e) => {
//!                 println!("ticker: last={}", e.data.last);
//!             }
//!             OrderBookMessage::GlobalTicker(e) => {
//!                 println!("global ticker: last={}", e.data.last);
//!             }
//!         }
//!     }
//!     Ok(())
//! }
//! ```

use futures_util::stream::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;

use crate::error::{BitkubError, Result};
use crate::models::websocket::{
    AsksChangedEvent, BidsChangedEvent, GlobalTickerEvent, OrderBookOrder, TickerData, TickerEvent,
    TradeEntry, TradesChangedEvent,
};

/// Default WebSocket base URL for the order book stream.
const ORDERBOOK_WS_BASE: &str = "wss://api.bitkub.com/websocket-api/orderbook/";

/// Messages emitted by [`OrderBookClient`].
#[derive(Debug, Clone)]
pub enum OrderBookMessage {
    /// Bid levels have been added, updated, or removed.
    BidsChanged(BidsChangedEvent),
    /// Ask levels have been added, updated, or removed.
    AsksChanged(AsksChangedEvent),
    /// New trades occurred, with updated bid/ask snapshots.
    TradesChanged(TradesChangedEvent),
    /// Per-symbol ticker update.
    Ticker(TickerEvent),
    /// Global ticker update.
    GlobalTicker(GlobalTickerEvent),
}

/// Client for subscribing to a live order book WebSocket stream.
pub struct OrderBookClient {
    symbol_id: i32,
    url: String,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
    read_handle: Option<JoinHandle<()>>,
}

impl OrderBookClient {
    /// Create a new order book client for the given symbol ID.
    ///
    /// The symbol ID corresponds to `pairing_id` in the Bitkub API
    /// (e.g. `1` for THB_BTC).
    pub fn new(symbol_id: i32) -> Self {
        let url = format!("{ORDERBOOK_WS_BASE}{symbol_id}");
        Self {
            symbol_id,
            url,
            shutdown_tx: None,
            read_handle: None,
        }
    }

    /// Connect to the order book WebSocket and begin receiving events.
    ///
    /// Returns a pair of unbounded receivers: one for parsed order book
    /// messages and one for errors encountered during the read loop.
    pub async fn connect(
        &mut self,
    ) -> Result<(
        mpsc::UnboundedReceiver<OrderBookMessage>,
        mpsc::UnboundedReceiver<BitkubError>,
    )> {
        if self.shutdown_tx.is_some() {
            return Err(BitkubError::Internal("already connected".to_owned()));
        }

        tracing::debug!(
            symbol_id = self.symbol_id,
            url = %self.url,
            "connecting to orderbook websocket"
        );

        let (ws_stream, _response) = connect_async(&self.url).await?;
        let (_write, read) = ws_stream.split();

        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<OrderBookMessage>();
        let (err_tx, err_rx) = mpsc::unbounded_channel::<BitkubError>();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        self.shutdown_tx = Some(shutdown_tx);

        let symbol_id = self.symbol_id;
        let handle = tokio::spawn(read_loop(read, msg_tx, err_tx, shutdown_rx, symbol_id));
        self.read_handle = Some(handle);

        tracing::debug!(symbol_id = self.symbol_id, "orderbook websocket connected");
        Ok((msg_rx, err_rx))
    }

    /// Disconnect from the order book WebSocket.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(handle) = self.read_handle.take() {
            let _ = handle.await;
        }
        tracing::debug!(symbol_id = self.symbol_id, "orderbook websocket disconnected");
        Ok(())
    }
}

impl Drop for OrderBookClient {
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

type WsReadHalf = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
>;

async fn read_loop(
    mut read: WsReadHalf,
    msg_tx: mpsc::UnboundedSender<OrderBookMessage>,
    err_tx: mpsc::UnboundedSender<BitkubError>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    symbol_id: i32,
) {
    loop {
        tokio::select! {
            biased;

            _ = shutdown_rx.changed() => {
                tracing::debug!(symbol_id, "orderbook ws read loop received shutdown signal");
                break;
            }

            frame = read.next() => {
                match frame {
                    Some(Ok(msg)) => {
                        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                            parse_and_route(&text, symbol_id, &msg_tx, &err_tx);
                        }
                    }
                    Some(Err(e)) => {
                        tracing::warn!(symbol_id, error = %e, "orderbook ws read error");
                        let _ = err_tx.send(BitkubError::WebSocket(e));
                        break;
                    }
                    None => {
                        tracing::debug!(symbol_id, "orderbook ws stream ended");
                        break;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Raw JSON structure for peeking at the event field
// ---------------------------------------------------------------------------

/// Intermediate structure used to peek at the `event` field before fully
/// parsing the payload.
#[derive(Deserialize)]
struct RawOrderBookEvent {
    event: String,
    #[serde(default)]
    pairing_id: Option<i32>,
    data: serde_json::Value,
}

fn parse_and_route(
    text: &str,
    symbol_id: i32,
    msg_tx: &mpsc::UnboundedSender<OrderBookMessage>,
    err_tx: &mpsc::UnboundedSender<BitkubError>,
) {
    let raw: RawOrderBookEvent = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                symbol_id,
                error = %e,
                raw = %text,
                "failed to parse orderbook ws message"
            );
            let _ = err_tx.send(BitkubError::Json(e));
            return;
        }
    };

    let pairing_id = raw.pairing_id.unwrap_or(symbol_id);

    match raw.event.as_str() {
        "bidschanged" => {
            match serde_json::from_value::<Vec<OrderBookOrder>>(raw.data) {
                Ok(orders) => {
                    let _ = msg_tx.send(OrderBookMessage::BidsChanged(BidsChangedEvent {
                        pairing_id,
                        data: orders,
                    }));
                }
                Err(e) => {
                    tracing::warn!(symbol_id, error = %e, "failed to parse bidschanged data");
                    let _ = err_tx.send(BitkubError::Json(e));
                }
            }
        }

        "askschanged" => {
            match serde_json::from_value::<Vec<OrderBookOrder>>(raw.data) {
                Ok(orders) => {
                    let _ = msg_tx.send(OrderBookMessage::AsksChanged(AsksChangedEvent {
                        pairing_id,
                        data: orders,
                    }));
                }
                Err(e) => {
                    tracing::warn!(symbol_id, error = %e, "failed to parse askschanged data");
                    let _ = err_tx.send(BitkubError::Json(e));
                }
            }
        }

        "tradeschanged" => {
            match parse_trades_changed(raw.data, pairing_id) {
                Ok(event) => {
                    let _ = msg_tx.send(OrderBookMessage::TradesChanged(event));
                }
                Err(e) => {
                    tracing::warn!(symbol_id, error = %e, "failed to parse tradeschanged data");
                    let _ = err_tx.send(BitkubError::Json(e));
                }
            }
        }

        "ticker" => {
            match serde_json::from_value::<TickerData>(raw.data) {
                Ok(data) => {
                    let _ = msg_tx.send(OrderBookMessage::Ticker(TickerEvent {
                        pairing_id,
                        data,
                    }));
                }
                Err(e) => {
                    tracing::warn!(symbol_id, error = %e, "failed to parse ticker data");
                    let _ = err_tx.send(BitkubError::Json(e));
                }
            }
        }

        "global.ticker" => {
            match serde_json::from_value::<TickerData>(raw.data) {
                Ok(data) => {
                    let _ = msg_tx.send(OrderBookMessage::GlobalTicker(GlobalTickerEvent {
                        data,
                    }));
                }
                Err(e) => {
                    tracing::warn!(symbol_id, error = %e, "failed to parse global.ticker data");
                    let _ = err_tx.send(BitkubError::Json(e));
                }
            }
        }

        other => {
            tracing::debug!(symbol_id, event = %other, "unknown orderbook event, ignoring");
        }
    }
}

/// Parse `tradeschanged` data which is `[trades_array, bids_array, asks_array]`.
fn parse_trades_changed(
    data: serde_json::Value,
    pairing_id: i32,
) -> std::result::Result<TradesChangedEvent, serde_json::Error> {
    let outer = match data.as_array() {
        Some(a) if a.len() == 3 => a,
        Some(a) => {
            return Err(serde::de::Error::custom(format!(
                "expected 3 arrays in tradeschanged, got {}",
                a.len()
            )));
        }
        None => {
            return Err(serde::de::Error::custom(
                "expected array for tradeschanged data",
            ));
        }
    };

    let trades: Vec<TradeEntry> = serde_json::from_value(outer[0].clone())?;
    let bids: Vec<OrderBookOrder> = serde_json::from_value(outer[1].clone())?;
    let asks: Vec<OrderBookOrder> = serde_json::from_value(outer[2].clone())?;

    Ok(TradesChangedEvent {
        pairing_id,
        trades,
        bids,
        asks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_construction() {
        let client = OrderBookClient::new(1);
        assert_eq!(
            client.url,
            "wss://api.bitkub.com/websocket-api/orderbook/1"
        );
    }

    #[test]
    fn url_construction_different_symbol() {
        let client = OrderBookClient::new(26);
        assert_eq!(
            client.url,
            "wss://api.bitkub.com/websocket-api/orderbook/26"
        );
    }

    #[test]
    fn parse_bidschanged_event() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"bidschanged","pairing_id":1,"data":[[1500.5, 1000000, 0.0015, 0, 1, 0]]}"#;
        parse_and_route(json, 1, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            OrderBookMessage::BidsChanged(e) => {
                assert_eq!(e.pairing_id, 1);
                assert_eq!(e.data.len(), 1);
                assert!(e.data[0].is_new);
                assert!(!e.data[0].user_owner);
            }
            _ => panic!("expected BidsChanged variant"),
        }
    }

    #[test]
    fn parse_askschanged_event() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"askschanged","pairing_id":1,"data":[[200.0, 1000001, 0.002, 0, 0, 1]]}"#;
        parse_and_route(json, 1, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            OrderBookMessage::AsksChanged(e) => {
                assert_eq!(e.pairing_id, 1);
                assert_eq!(e.data.len(), 1);
                assert!(!e.data[0].is_new);
                assert!(e.data[0].user_owner);
            }
            _ => panic!("expected AsksChanged variant"),
        }
    }

    #[test]
    fn parse_tradeschanged_event() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        // trades: [timestamp, rate, amount, side, 0, 0, is_new, user_buyer, user_seller]
        // bids/asks: [volume, rate, amount, reserved, is_new, user_owner]
        let json = r#"{"event":"tradeschanged","pairing_id":1,"data":[[[1609459200,"1000000","0.001","BUY",0,0,true,false,false]],[[100.5,1000000,0.001,0,1,0]],[[200.0,1000001,0.002,0,0,0]]]}"#;
        parse_and_route(json, 1, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            OrderBookMessage::TradesChanged(e) => {
                assert_eq!(e.pairing_id, 1);
                assert_eq!(e.trades.len(), 1);
                assert_eq!(e.trades[0].side, "BUY");
                assert_eq!(e.bids.len(), 1);
                assert_eq!(e.asks.len(), 1);
            }
            _ => panic!("expected TradesChanged variant"),
        }
    }

    #[test]
    fn parse_ticker_event() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"ticker","pairing_id":1,"data":{"baseVolume":100.5,"change":1.2,"close":100,"high24hr":105,"highestBid":99,"highestBidSize":10,"id":1,"isFrozen":0,"last":100,"low24hr":95,"lowestAsk":101,"lowestAskSize":5,"open":98,"percentChange":1.5,"quoteVolume":10000}}"#;
        parse_and_route(json, 1, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            OrderBookMessage::Ticker(e) => {
                assert_eq!(e.pairing_id, 1);
                assert_eq!(e.data.id, 1);
            }
            _ => panic!("expected Ticker variant"),
        }
    }

    #[test]
    fn parse_global_ticker_event() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, _err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"global.ticker","data":{"baseVolume":100.5,"change":1.2,"close":100,"high24hr":105,"highestBid":99,"highestBidSize":10,"id":1,"isFrozen":0,"last":100,"low24hr":95,"lowestAsk":101,"lowestAskSize":5,"open":98,"percentChange":1.5,"quoteVolume":10000}}"#;
        parse_and_route(json, 1, &msg_tx, &err_tx);

        let msg = msg_rx.try_recv().expect("should receive a message");
        match msg {
            OrderBookMessage::GlobalTicker(e) => {
                assert_eq!(e.data.id, 1);
            }
            _ => panic!("expected GlobalTicker variant"),
        }
    }

    #[test]
    fn parse_unknown_event_is_ignored() {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let (err_tx, mut err_rx) = mpsc::unbounded_channel();

        let json = r#"{"event":"unknown","pairing_id":1,"data":[]}"#;
        parse_and_route(json, 1, &msg_tx, &err_tx);

        assert!(msg_rx.try_recv().is_err());
        assert!(err_rx.try_recv().is_err());
    }

    #[test]
    fn parse_invalid_json_sends_error() {
        let (msg_tx, _msg_rx) = mpsc::unbounded_channel();
        let (err_tx, mut err_rx) = mpsc::unbounded_channel();

        parse_and_route("{{bad", 1, &msg_tx, &err_tx);

        let err = err_rx.try_recv().expect("should receive an error");
        assert!(matches!(err, BitkubError::Json(_)));
    }

    #[test]
    fn parse_tradeschanged_wrong_array_length() {
        let data = serde_json::json!([[], []]);
        let result = parse_trades_changed(data, 1);
        assert!(result.is_err());
    }

    #[test]
    fn parse_tradeschanged_not_array() {
        let data = serde_json::json!({"not": "array"});
        let result = parse_trades_changed(data, 1);
        assert!(result.is_err());
    }
}
