use rust_decimal::Decimal;
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::fmt;

use super::serde_helpers::DecimalAny;

/// Symbol metadata from `GET /api/v3/market/symbols`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub base_asset: String,
    pub base_asset_scale: i32,
    pub buy_price_gap_as_percent: i32,
    pub created_at: String,
    pub description: String,
    pub freeze_buy: bool,
    pub freeze_cancel: bool,
    pub freeze_sell: bool,
    pub market_segment: String,
    pub min_quote_size: Decimal,
    pub modified_at: String,
    pub name: String,
    pub pairing_id: i32,
    pub price_scale: i32,
    pub price_step: String,
    pub quantity_scale: i32,
    pub quantity_step: String,
    pub quote_asset: String,
    pub quote_asset_scale: i32,
    pub sell_price_gap_as_percent: i32,
    pub status: String,
    pub symbol: String,
    pub source: String,
}

/// 24-hour ticker data from `GET /api/v3/market/ticker`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker {
    pub symbol: String,
    pub base_volume: Decimal,
    #[serde(rename = "high24hr")]
    pub high_24_hr: Decimal,
    pub highest_bid: Decimal,
    pub last: Decimal,
    #[serde(rename = "low24hr")]
    pub low_24_hr: Decimal,
    pub lowest_ask: Decimal,
    pub percent_change: Decimal,
    pub quote_volume: Decimal,
}

/// A single order from `GET /api/v3/market/bids` or `/asks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookEntry {
    pub order_id: String,
    pub price: Decimal,
    pub side: String,
    pub size: Decimal,
    pub timestamp: i64,
    pub volume: Decimal,
}

/// Aggregated order book depth from `GET /api/v3/market/depth`.
///
/// The raw JSON returns bids and asks as arrays of `[price, size]` pairs.
/// This struct deserializes them into `Vec<(Decimal, Decimal)>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Depth {
    #[serde(deserialize_with = "deserialize_depth_levels")]
    pub bids: Vec<(Decimal, Decimal)>,
    #[serde(deserialize_with = "deserialize_depth_levels")]
    pub asks: Vec<(Decimal, Decimal)>,
}

/// Deserialize depth levels from `[[price, size], ...]` where each inner
/// element can be either a number or a string representation of a number.
fn deserialize_depth_levels<'de, D>(deserializer: D) -> Result<Vec<(Decimal, Decimal)>, D::Error>
where
    D: Deserializer<'de>,
{
    struct DepthLevelsVisitor;

    impl<'de> Visitor<'de> for DepthLevelsVisitor {
        type Value = Vec<(Decimal, Decimal)>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an array of [price, size] pairs")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut levels = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(pair) = seq.next_element::<Vec<DecimalAny>>()? {
                if pair.len() != 2 {
                    return Err(de::Error::custom(format!(
                        "expected [price, size] pair, got {} elements",
                        pair.len()
                    )));
                }
                levels.push((pair[0].0, pair[1].0));
            }
            Ok(levels)
        }
    }

    deserializer.deserialize_seq(DepthLevelsVisitor)
}

/// A recent trade from `GET /api/v3/market/trades`.
///
/// The raw JSON returns each trade as `[timestamp, rate, amount, "BUY"|"SELL"]`.
#[derive(Debug, Clone, Serialize)]
pub struct RecentTrade {
    pub timestamp: i64,
    pub rate: Decimal,
    pub amount: Decimal,
    pub side: String,
}

impl<'de> Deserialize<'de> for RecentTrade {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RecentTradeVisitor;

        impl<'de> Visitor<'de> for RecentTradeVisitor {
            type Value = RecentTrade;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a [timestamp, rate, amount, side] array")
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

                Ok(RecentTrade {
                    timestamp,
                    rate: rate.0,
                    amount: amount.0,
                    side,
                })
            }
        }

        deserializer.deserialize_seq(RecentTradeVisitor)
    }
}

/// TradingView OHLCV history from `GET /tradingview/history`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingViewHistory {
    pub c: Vec<f64>,
    pub h: Vec<f64>,
    pub l: Vec<f64>,
    pub o: Vec<f64>,
    pub s: String,
    pub t: Vec<i64>,
    pub v: Vec<f64>,
}

/// Server status for an endpoint from `GET /api/status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStatus {
    pub name: String,
    pub status: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_depth() {
        let json = r#"{"bids":[[100.5,1.2],[99.0,3.5]],"asks":[["101.0","0.8"]]}"#;
        let depth: Depth = serde_json::from_str(json).unwrap();
        assert_eq!(depth.bids.len(), 2);
        assert_eq!(depth.asks.len(), 1);
        assert_eq!(depth.bids[0].0, Decimal::new(1005, 1));
        assert_eq!(depth.asks[0].0, Decimal::new(1010, 1));
    }

    #[test]
    fn deserialize_recent_trade() {
        let json = r#"[1609459200, "100.50", "0.5", "BUY"]"#;
        let trade: RecentTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.timestamp, 1609459200);
        assert_eq!(trade.side, "BUY");
        assert_eq!(trade.rate, Decimal::new(1005, 1));
    }

    #[test]
    fn deserialize_recent_trade_numeric_values() {
        let json = r#"[1609459200, 100.50, 0.5, "SELL"]"#;
        let trade: RecentTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.timestamp, 1609459200);
        assert_eq!(trade.side, "SELL");
    }
}
