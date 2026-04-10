//! Authenticated market (trading) endpoints.
//!
//! These endpoints require API key and secret credentials. They cover wallet
//! balances, order placement, cancellation, and order history.

use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::client::BitkubClient;
use crate::error::Result;
use crate::models::trading::{
    Balance, CancelOrderRequest, OpenOrder, OrderHistory, OrderInfo, PlaceOrderRequest,
    PlaceOrderResponse,
};

impl BitkubClient {
    // ------------------------------------------------------------------
    // Wallet & balances
    // ------------------------------------------------------------------

    /// Retrieve the user's wallet balances (available amounts only).
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/market/wallet` (secure)
    ///
    /// Returns `{"error": 0, "result": {"THB": 1000.0, "BTC": 0.5, ...}}`.
    pub async fn get_wallet(&self) -> Result<HashMap<String, Decimal>> {
        self.post_secure("/api/v3/market/wallet", &serde_json::Value::Object(Default::default()))
            .await
    }

    /// Retrieve detailed balances including available, reserved, and total.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/market/balances` (secure)
    ///
    /// Returns `{"error": 0, "result": {"THB": {"available": ..., "reserved": ...}, ...}}`.
    pub async fn get_balances(&self) -> Result<HashMap<String, Balance>> {
        self.post_secure(
            "/api/v3/market/balances",
            &serde_json::Value::Object(Default::default()),
        )
        .await
    }

    // ------------------------------------------------------------------
    // Order placement
    // ------------------------------------------------------------------

    /// Place a buy (bid) order.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/market/place-bid` (secure)
    ///
    /// # Arguments
    ///
    /// * `req` -- Order parameters including symbol, amount, rate, and order type.
    pub async fn place_bid(&self, req: &PlaceOrderRequest) -> Result<PlaceOrderResponse> {
        self.post_secure("/api/v3/market/place-bid", req).await
    }

    /// Place a sell (ask) order.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/market/place-ask` (secure)
    ///
    /// # Arguments
    ///
    /// * `req` -- Order parameters including symbol, amount, rate, and order type.
    pub async fn place_ask(&self, req: &PlaceOrderRequest) -> Result<PlaceOrderResponse> {
        self.post_secure("/api/v3/market/place-ask", req).await
    }

    // ------------------------------------------------------------------
    // Order cancellation
    // ------------------------------------------------------------------

    /// Cancel an existing open order.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/market/cancel-order` (secure)
    ///
    /// # Arguments
    ///
    /// * `req` -- Cancellation request containing the symbol, order ID, and side.
    pub async fn cancel_order(&self, req: &CancelOrderRequest) -> Result<()> {
        // The API returns `{"error": 0}` with no meaningful result payload.
        let _: serde_json::Value = self
            .post_secure("/api/v3/market/cancel-order", req)
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // WebSocket token
    // ------------------------------------------------------------------

    /// Obtain a short-lived WebSocket authentication token.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/market/wstoken` (secure)
    ///
    /// Returns `{"error": 0, "result": "<token>"}`.
    pub async fn get_ws_token(&self) -> Result<String> {
        self.post_secure(
            "/api/v3/market/wstoken",
            &serde_json::Value::Object(Default::default()),
        )
        .await
    }

    // ------------------------------------------------------------------
    // Order queries
    // ------------------------------------------------------------------

    /// List the user's open orders for a given symbol.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/market/my-open-orders` (secure)
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Trading pair (e.g. `"THB_BTC"`).
    pub async fn get_my_open_orders(&self, symbol: &str) -> Result<Vec<OpenOrder>> {
        let params: Vec<(&str, &str)> = vec![("sym", symbol)];
        self.get_secure("/api/v3/market/my-open-orders", &params)
            .await
    }

    /// Fetch the user's historical (filled/cancelled) orders for a symbol.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/market/my-order-history` (secure)
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Trading pair.
    /// * `page` -- Page number (1-based). Server default when `None`.
    /// * `limit` -- Number of records per page.
    pub async fn get_my_order_history(
        &self,
        symbol: &str,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<Vec<OrderHistory>> {
        let page_str = page.map(|p| p.to_string());
        let lmt_str = limit.map(|l| l.to_string());
        let mut params: Vec<(&str, &str)> = vec![("sym", symbol)];
        if let Some(ref p) = page_str {
            params.push(("p", p));
        }
        if let Some(ref lmt) = lmt_str {
            params.push(("lmt", lmt));
        }
        self.get_secure("/api/v3/market/my-order-history", &params)
            .await
    }

    /// Retrieve detailed information about a specific order.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/market/order-info` (secure)
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Trading pair.
    /// * `order_id` -- The unique order identifier.
    /// * `side` -- `"buy"` or `"sell"`.
    pub async fn get_order_info(
        &self,
        symbol: &str,
        order_id: &str,
        side: &str,
    ) -> Result<OrderInfo> {
        let params: Vec<(&str, &str)> = vec![
            ("sym", symbol),
            ("id", order_id),
            ("sd", side),
        ];
        self.get_secure("/api/v3/market/order-info", &params).await
    }
}
