//! Public (unauthenticated) endpoints.
//!
//! These endpoints do not require API credentials and return market-wide data
//! such as server status, tickers, orderbook depth, and recent trades.

use crate::client::BitkubClient;
use crate::error::Result;
use crate::models::market::{
    Depth, EndpointStatus, OrderBookEntry, RecentTrade, SymbolInfo, Ticker, TradingViewHistory,
};

impl BitkubClient {
    // ------------------------------------------------------------------
    // Status & server time
    // ------------------------------------------------------------------

    /// Retrieve the current status of all API endpoints.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/status`
    ///
    /// The response is a bare JSON array (no `{"error":0, "result":...}` wrapper).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> bitkub::error::Result<()> {
    /// let client = bitkub::client::BitkubClient::new();
    /// let statuses = client.get_status().await?;
    /// for s in &statuses {
    ///     println!("{}: {}", s.name, s.status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_status(&self) -> Result<Vec<EndpointStatus>> {
        self.get_raw("/api/status", &[] as &[(&str, &str)]).await
    }

    /// Return the server's current UNIX timestamp in milliseconds.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/servertime`
    ///
    /// The response is a bare integer (no wrapper).
    pub async fn get_server_time(&self) -> Result<u64> {
        self.get_raw("/api/v3/servertime", &[] as &[(&str, &str)])
            .await
    }

    // ------------------------------------------------------------------
    // Market data
    // ------------------------------------------------------------------

    /// List all available trading symbols and their configuration.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/market/symbols`
    ///
    /// Returns `{"error": 0, "result": [...]}`.
    pub async fn get_symbols(&self) -> Result<Vec<SymbolInfo>> {
        self.get("/api/v3/market/symbols", &[] as &[(&str, &str)])
            .await
    }

    /// Fetch ticker data for one or all trading pairs.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/market/ticker`
    ///
    /// The response is a bare JSON object keyed by symbol name (no V3 wrapper).
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Optional symbol filter (e.g. `"THB_BTC"`). When `None`,
    ///   tickers for all pairs are returned.
    pub async fn get_ticker(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<Ticker>> {
        let params: Vec<(&str, &str)> = match symbol {
            Some(sym) => vec![("sym", sym)],
            None => vec![],
        };
        self.get_raw("/api/v3/market/ticker", &params).await
    }

    /// Fetch open bid (buy) orders from the orderbook.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/market/bids`
    ///
    /// Returns `{"error": 0, "result": [...]}`.
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Trading pair (e.g. `"THB_BTC"`).
    /// * `limit` -- Maximum number of entries to return. Defaults to the
    ///   server-side limit when `None`.
    pub async fn get_bids(
        &self,
        symbol: &str,
        limit: Option<i32>,
    ) -> Result<Vec<OrderBookEntry>> {
        let lmt_str = limit.map(|l| l.to_string());
        let mut params: Vec<(&str, &str)> = vec![("sym", symbol)];
        if let Some(ref lmt) = lmt_str {
            params.push(("lmt", lmt));
        }
        self.get("/api/v3/market/bids", &params).await
    }

    /// Fetch open ask (sell) orders from the orderbook.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/market/asks`
    ///
    /// Returns `{"error": 0, "result": [...]}`.
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Trading pair (e.g. `"THB_BTC"`).
    /// * `limit` -- Maximum number of entries to return.
    pub async fn get_asks(
        &self,
        symbol: &str,
        limit: Option<i32>,
    ) -> Result<Vec<OrderBookEntry>> {
        let lmt_str = limit.map(|l| l.to_string());
        let mut params: Vec<(&str, &str)> = vec![("sym", symbol)];
        if let Some(ref lmt) = lmt_str {
            params.push(("lmt", lmt));
        }
        self.get("/api/v3/market/asks", &params).await
    }

    /// Fetch aggregated orderbook depth for a symbol.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/market/depth`
    ///
    /// Returns `{"error": 0, "result": {...}}`.
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Trading pair.
    /// * `limit` -- Maximum number of price levels per side.
    pub async fn get_depth(&self, symbol: &str, limit: Option<i32>) -> Result<Depth> {
        let lmt_str = limit.map(|l| l.to_string());
        let mut params: Vec<(&str, &str)> = vec![("sym", symbol)];
        if let Some(ref lmt) = lmt_str {
            params.push(("lmt", lmt));
        }
        self.get("/api/v3/market/depth", &params).await
    }

    /// Fetch recent trades for a symbol.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/market/trades`
    ///
    /// Returns `{"error": 0, "result": [...]}`.
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Trading pair.
    /// * `limit` -- Maximum number of trades to return.
    pub async fn get_trades(
        &self,
        symbol: &str,
        limit: Option<i32>,
    ) -> Result<Vec<RecentTrade>> {
        let lmt_str = limit.map(|l| l.to_string());
        let mut params: Vec<(&str, &str)> = vec![("sym", symbol)];
        if let Some(ref lmt) = lmt_str {
            params.push(("lmt", lmt));
        }
        self.get("/api/v3/market/trades", &params).await
    }

    // ------------------------------------------------------------------
    // TradingView
    // ------------------------------------------------------------------

    /// Fetch OHLCV candle data in TradingView-compatible format.
    ///
    /// # Bitkub API
    ///
    /// `GET /tradingview/history`
    ///
    /// The response is a TradingView-format JSON object (no wrapper).
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Trading pair (e.g. `"THB_BTC"`).
    /// * `resolution` -- Candle interval (`"1"`, `"5"`, `"15"`, `"60"`, `"240"`, `"1D"`).
    /// * `from` -- Start UNIX timestamp (seconds).
    /// * `to` -- End UNIX timestamp (seconds).
    pub async fn get_tradingview_history(
        &self,
        symbol: &str,
        resolution: &str,
        from: i64,
        to: i64,
    ) -> Result<TradingViewHistory> {
        let from_str = from.to_string();
        let to_str = to.to_string();
        let params: Vec<(&str, &str)> = vec![
            ("symbol", symbol),
            ("resolution", resolution),
            ("from", &from_str),
            ("to", &to_str),
        ];
        self.get_raw("/tradingview/history", &params).await
    }
}
