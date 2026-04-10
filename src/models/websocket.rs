use rust_decimal::Decimal;
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::fmt;

use super::serde_helpers::{
    flexible_decimal, flexible_option_decimal, BoolAny, DecimalAny,
};

// ---------------------------------------------------------------------------
// Public WebSocket - Trade stream
// ---------------------------------------------------------------------------

/// A trade event from the public WebSocket trade stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeStreamMessage {
    pub stream: String,
    #[serde(rename = "sym")]
    pub symbol: String,
    pub txn: String,
    #[serde(rename = "rat", with = "flexible_decimal")]
    pub rate: Decimal,
    #[serde(rename = "amt", with = "flexible_decimal")]
    pub amount: Decimal,
    pub bid: String,
    pub sid: String,
    pub ts: i64,
}

// ---------------------------------------------------------------------------
// Public WebSocket - Ticker stream
// ---------------------------------------------------------------------------

/// A ticker snapshot from the public WebSocket ticker stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerStreamMessage {
    pub stream: String,
    pub id: i32,
    #[serde(with = "flexible_decimal")]
    pub last: Decimal,
    #[serde(rename = "lowestAsk", with = "flexible_decimal")]
    pub lowest_ask: Decimal,
    #[serde(rename = "lowestAskSize", with = "flexible_decimal")]
    pub lowest_ask_size: Decimal,
    #[serde(rename = "highestBid", with = "flexible_decimal")]
    pub highest_bid: Decimal,
    #[serde(rename = "highestBidSize", with = "flexible_decimal")]
    pub highest_bid_size: Decimal,
    #[serde(with = "flexible_decimal")]
    pub change: Decimal,
    #[serde(rename = "percentChange", with = "flexible_decimal")]
    pub percent_change: Decimal,
    #[serde(rename = "baseVolume", with = "flexible_decimal")]
    pub base_volume: Decimal,
    #[serde(rename = "quoteVolume", with = "flexible_decimal")]
    pub quote_volume: Decimal,
    #[serde(rename = "isFrozen")]
    pub is_frozen: i32,
    #[serde(rename = "high24hr", with = "flexible_decimal")]
    pub high_24hr: Decimal,
    #[serde(rename = "low24hr", with = "flexible_decimal")]
    pub low_24hr: Decimal,
    #[serde(with = "flexible_decimal")]
    pub open: Decimal,
    #[serde(with = "flexible_decimal")]
    pub close: Decimal,
}

// ---------------------------------------------------------------------------
// Live Order Book events
// ---------------------------------------------------------------------------

/// A tagged union of all order book WebSocket event types.
#[derive(Debug, Clone)]
pub enum OrderBookEvent {
    BidsChanged(BidsChangedEvent),
    AsksChanged(AsksChangedEvent),
    TradesChanged(TradesChangedEvent),
    Ticker(TickerEvent),
    GlobalTicker(GlobalTickerEvent),
}

/// A single order in the live order book.
///
/// Arrives as a JSON array: `[volume, rate, amount, reserved, is_new, user_owner]`.
#[derive(Debug, Clone, Serialize)]
pub struct OrderBookOrder {
    pub volume: Decimal,
    pub rate: Decimal,
    pub amount: Decimal,
    pub reserved: i32,
    pub is_new: bool,
    pub user_owner: bool,
}

impl<'de> Deserialize<'de> for OrderBookOrder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OrderBookOrderVisitor;

        impl<'de> Visitor<'de> for OrderBookOrderVisitor {
            type Value = OrderBookOrder;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("[volume, rate, amount, reserved, is_new, user_owner]")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let volume: DecimalAny = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let rate: DecimalAny = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let amount: DecimalAny = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let reserved: i32 = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let is_new: BoolAny = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let user_owner: BoolAny = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(5, &self))?;

                Ok(OrderBookOrder {
                    volume: volume.0,
                    rate: rate.0,
                    amount: amount.0,
                    reserved,
                    is_new: is_new.0,
                    user_owner: user_owner.0,
                })
            }
        }

        deserializer.deserialize_seq(OrderBookOrderVisitor)
    }
}

/// Bids changed event from the live order book WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidsChangedEvent {
    pub pairing_id: i32,
    pub data: Vec<OrderBookOrder>,
}

/// Asks changed event from the live order book WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsksChangedEvent {
    pub pairing_id: i32,
    pub data: Vec<OrderBookOrder>,
}

/// A single trade within a `TradesChangedEvent`.
///
/// Arrives as a JSON array:
/// `[timestamp, rate, amount, side, 0, 0, is_new, user_buyer, user_seller]`.
#[derive(Debug, Clone, Serialize)]
pub struct TradeEntry {
    pub timestamp: i64,
    pub rate: Decimal,
    pub amount: Decimal,
    pub side: String,
    pub is_new: bool,
    pub user_buyer: bool,
    pub user_seller: bool,
}

impl<'de> Deserialize<'de> for TradeEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TradeEntryVisitor;

        impl<'de> Visitor<'de> for TradeEntryVisitor {
            type Value = TradeEntry;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(
                    "[timestamp, rate, amount, side, 0, 0, is_new, user_buyer, user_seller]",
                )
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let timestamp: i64 = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let rate: DecimalAny = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let amount: DecimalAny = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let side: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;
                // Skip two reserved fields (indices 4 and 5).
                let _reserved_0: serde_json::Value = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let _reserved_1: serde_json::Value = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(5, &self))?;
                let is_new: BoolAny = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(6, &self))?;
                let user_buyer: BoolAny = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(7, &self))?;
                let user_seller: BoolAny = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(8, &self))?;

                Ok(TradeEntry {
                    timestamp,
                    rate: rate.0,
                    amount: amount.0,
                    side,
                    is_new: is_new.0,
                    user_buyer: user_buyer.0,
                    user_seller: user_seller.0,
                })
            }
        }

        deserializer.deserialize_seq(TradeEntryVisitor)
    }
}

/// Trades changed event including updated bids and asks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradesChangedEvent {
    pub pairing_id: i32,
    pub trades: Vec<TradeEntry>,
    pub bids: Vec<OrderBookOrder>,
    pub asks: Vec<OrderBookOrder>,
}

/// Ticker event for a specific pairing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerEvent {
    pub pairing_id: i32,
    pub data: TickerData,
}

/// Global ticker event (not pairing-specific).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTickerEvent {
    pub data: TickerData,
}

/// Shared ticker data used by both `TickerEvent` and `GlobalTickerEvent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerData {
    #[serde(rename = "baseVolume", with = "flexible_decimal")]
    pub base_volume: Decimal,
    #[serde(with = "flexible_decimal")]
    pub change: Decimal,
    #[serde(with = "flexible_decimal")]
    pub close: Decimal,
    #[serde(rename = "high24hr", with = "flexible_decimal")]
    pub high_24hr: Decimal,
    #[serde(rename = "highestBid", with = "flexible_decimal")]
    pub highest_bid: Decimal,
    #[serde(rename = "highestBidSize", with = "flexible_decimal")]
    pub highest_bid_size: Decimal,
    pub id: i32,
    #[serde(rename = "isFrozen")]
    pub is_frozen: i32,
    #[serde(with = "flexible_decimal")]
    pub last: Decimal,
    #[serde(rename = "low24hr", with = "flexible_decimal")]
    pub low_24hr: Decimal,
    #[serde(rename = "lowestAsk", with = "flexible_decimal")]
    pub lowest_ask: Decimal,
    #[serde(rename = "lowestAskSize", with = "flexible_decimal")]
    pub lowest_ask_size: Decimal,
    #[serde(with = "flexible_decimal")]
    pub open: Decimal,
    #[serde(rename = "percentChange", with = "flexible_decimal")]
    pub percent_change: Decimal,
    #[serde(rename = "quoteVolume", with = "flexible_decimal")]
    pub quote_volume: Decimal,
    pub stream: Option<String>,
}

// ---------------------------------------------------------------------------
// Private WebSocket types
// ---------------------------------------------------------------------------

/// A raw private WebSocket message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateWsMessage {
    pub event: String,
    pub code: String,
    pub message: String,
    pub data: serde_json::Value,
    pub connection_id: Option<String>,
    pub timestamp: Option<String>,
}

/// An order update event from the private WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpdate {
    pub user_id: String,
    pub order_id: String,
    pub client_id: Option<String>,
    pub symbol: String,
    /// `"buy"` or `"sell"`.
    pub side: String,
    /// `"limit"`, `"stoplimit"`, or `"market"`.
    #[serde(rename = "type")]
    pub order_type: String,
    /// One of: `"new"`, `"open"`, `"rejected"`, `"partial_filled"`, `"filled"`,
    /// `"partial_filled_canceled"`, `"canceled"`, `"untriggered"`.
    pub status: String,
    #[serde(with = "flexible_option_decimal")]
    pub price: Option<Decimal>,
    #[serde(with = "flexible_option_decimal")]
    pub stop_price: Option<Decimal>,
    pub order_currency: String,
    #[serde(with = "flexible_decimal")]
    pub order_amount: Decimal,
    pub executed_currency: String,
    #[serde(with = "flexible_decimal")]
    pub executed_amount: Decimal,
    pub received_currency: String,
    #[serde(with = "flexible_decimal")]
    pub received_amount: Decimal,
    #[serde(with = "flexible_decimal")]
    pub total_fee: Decimal,
    #[serde(with = "flexible_decimal")]
    pub credit_used: Decimal,
    #[serde(with = "flexible_decimal")]
    pub net_fee_paid: Decimal,
    #[serde(with = "flexible_option_decimal")]
    pub avg_filled_price: Option<Decimal>,
    pub post_only: bool,
    pub canceled_by: Option<String>,
    pub order_created_at: i64,
    pub order_triggered_at: Option<i64>,
    pub order_updated_at: Option<i64>,
}

/// A match (fill) update event from the private WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchUpdate {
    pub order_id: String,
    pub txn_id: String,
    pub client_id: Option<String>,
    pub symbol: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub status: String,
    pub side: String,
    pub is_maker: bool,
    #[serde(with = "flexible_decimal")]
    pub price: Decimal,
    pub executed_currency: String,
    #[serde(with = "flexible_decimal")]
    pub executed_amount: Decimal,
    pub received_currency: String,
    #[serde(with = "flexible_decimal")]
    pub received_amount: Decimal,
    #[serde(with = "flexible_decimal")]
    pub fee_rate: Decimal,
    #[serde(with = "flexible_decimal")]
    pub total_fee: Decimal,
    #[serde(with = "flexible_decimal")]
    pub credit_used: Decimal,
    #[serde(with = "flexible_decimal")]
    pub net_fee_paid: Decimal,
    pub txn_ts: i64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_order_book_order() {
        let json = r#"[1500.5, 100000, 0.015, 0, true, false]"#;
        let order: OrderBookOrder = serde_json::from_str(json).unwrap();
        assert_eq!(order.rate, Decimal::from(100000));
        assert!(order.is_new);
        assert!(!order.user_owner);
    }

    #[test]
    fn deserialize_order_book_order_int_bools() {
        let json = r#"[1500.5, 100000, 0.015, 0, 1, 0]"#;
        let order: OrderBookOrder = serde_json::from_str(json).unwrap();
        assert!(order.is_new);
        assert!(!order.user_owner);
    }

    #[test]
    fn deserialize_trade_entry() {
        let json = r#"[1609459200, "100.50", "0.5", "BUY", 0, 0, true, false, false]"#;
        let trade: TradeEntry = serde_json::from_str(json).unwrap();
        assert_eq!(trade.timestamp, 1609459200);
        assert_eq!(trade.side, "BUY");
        assert!(trade.is_new);
        assert!(!trade.user_buyer);
        assert!(!trade.user_seller);
    }

    #[test]
    fn deserialize_ticker_data_from_numbers() {
        let json = r#"{
            "baseVolume": 100.5,
            "change": 1.2,
            "close": 100,
            "high24hr": 105,
            "highestBid": 99,
            "highestBidSize": 10,
            "id": 1,
            "isFrozen": 0,
            "last": 100,
            "low24hr": 95,
            "lowestAsk": 101,
            "lowestAskSize": 5,
            "open": 98,
            "percentChange": 1.5,
            "quoteVolume": 10000
        }"#;
        let data: TickerData = serde_json::from_str(json).unwrap();
        assert_eq!(data.id, 1);
        assert_eq!(data.is_frozen, 0);
        assert!(data.stream.is_none());
    }

    #[test]
    fn deserialize_ticker_data_from_strings() {
        let json = r#"{
            "baseVolume": "100.5",
            "change": "1.2",
            "close": "100",
            "high24hr": "105",
            "highestBid": "99",
            "highestBidSize": "10",
            "id": 1,
            "isFrozen": 0,
            "last": "100",
            "low24hr": "95",
            "lowestAsk": "101",
            "lowestAskSize": "5",
            "open": "98",
            "percentChange": "1.5",
            "quoteVolume": "10000"
        }"#;
        let data: TickerData = serde_json::from_str(json).unwrap();
        assert_eq!(data.id, 1);
        assert_eq!(data.last, Decimal::from(100));
    }

    #[test]
    fn deserialize_private_ws_message() {
        let json = r#"{
            "event": "order.update",
            "code": "0",
            "message": "success",
            "data": {},
            "connection_id": "abc123",
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;
        let msg: PrivateWsMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.event, "order.update");
        assert_eq!(msg.connection_id.as_deref(), Some("abc123"));
    }
}
