use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Wallet balances from `POST /api/v3/market/wallet`.
///
/// The response is `{"error": 0, "result": {"THB": 188379.27, "BTC": 8.903}}`.
/// Use `ApiResponse<WalletBalances>` to deserialize the full response.
pub type WalletBalances = HashMap<String, Decimal>;

/// Detailed balance for a single asset from `POST /api/v3/market/balances`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub available: Decimal,
    pub reserved: Decimal,
}

/// Detailed balances keyed by asset symbol.
///
/// Use `ApiResponse<DetailedBalances>` to deserialize the full response.
pub type DetailedBalances = HashMap<String, Balance>;

/// Request body for placing a bid or ask order.
#[derive(Debug, Clone, Serialize)]
pub struct PlaceOrderRequest {
    pub sym: String,
    pub amt: Decimal,
    pub rat: Decimal,
    /// Order type: `"limit"` or `"market"`.
    pub typ: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_only: Option<bool>,
}

/// Response after successfully placing a bid or ask order.
#[derive(Debug, Clone, Deserialize)]
pub struct PlaceOrderResponse {
    pub id: String,
    pub typ: String,
    pub amt: Decimal,
    pub rat: Decimal,
    pub fee: Decimal,
    pub cre: Decimal,
    pub rec: Decimal,
    pub ts: String,
    pub ci: Option<String>,
}

/// Request body for cancelling an order.
#[derive(Debug, Clone, Serialize)]
pub struct CancelOrderRequest {
    pub sym: String,
    pub id: String,
    /// Side of the order: `"buy"` or `"sell"`.
    pub sd: String,
}

/// An open order from `GET /api/v3/market/my-open-orders`.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenOrder {
    pub id: String,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub rate: Decimal,
    pub fee: Decimal,
    pub credit: Decimal,
    pub amount: Decimal,
    pub receive: Decimal,
    pub parent_id: String,
    pub super_id: String,
    pub client_id: Option<String>,
    pub ts: i64,
}

/// A filled order from `GET /api/v3/market/my-order-history`.
#[derive(Debug, Clone, Deserialize)]
pub struct OrderHistory {
    pub txn_id: String,
    pub order_id: String,
    pub parent_order_id: String,
    pub super_order_id: String,
    pub client_id: Option<String>,
    pub taken_by_me: bool,
    pub is_maker: bool,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub rate: Decimal,
    pub fee: Decimal,
    pub credit: Decimal,
    pub amount: Decimal,
    pub ts: i64,
    pub order_closed_at: Option<i64>,
}

/// Detailed order information from `GET /api/v3/market/order-info`.
#[derive(Debug, Clone, Deserialize)]
pub struct OrderInfo {
    pub id: String,
    pub first: String,
    pub parent: String,
    pub last: String,
    pub client_id: Option<String>,
    pub post_only: bool,
    pub amount: Decimal,
    pub rate: Decimal,
    pub fee: Decimal,
    pub credit: Decimal,
    pub filled: Decimal,
    pub total: Decimal,
    pub status: String,
    pub partial_filled: bool,
    pub remaining: Decimal,
    pub history: Vec<OrderHistoryEntry>,
}

/// A single fill within an order's history.
#[derive(Debug, Clone, Deserialize)]
pub struct OrderHistoryEntry {
    pub amount: Decimal,
    pub credit: Decimal,
    pub fee: Decimal,
    pub id: String,
    pub rate: Decimal,
    pub timestamp: i64,
    pub txn_id: String,
}
