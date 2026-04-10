//! User account endpoints.
//!
//! Authenticated endpoints for retrieving user-level information such as
//! trading credits, account limits, and coin conversion history.

use rust_decimal::Decimal;

use crate::client::BitkubClient;
use crate::error::Result;
use crate::models::user::{CoinConvertHistory, UserLimits};

impl BitkubClient {
    // ------------------------------------------------------------------
    // Trading credits
    // ------------------------------------------------------------------

    /// Retrieve the user's available trading credits.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/user/trading-credits` (secure)
    ///
    /// Returns `{"error": 0, "result": <decimal>}`.
    pub async fn get_trading_credits(&self) -> Result<Decimal> {
        self.post_secure(
            "/api/v3/user/trading-credits",
            &serde_json::Value::Object(Default::default()),
        )
        .await
    }

    // ------------------------------------------------------------------
    // Account limits
    // ------------------------------------------------------------------

    /// Retrieve the user's deposit/withdrawal limits and usage.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/user/limits` (secure)
    ///
    /// Returns `{"error": 0, "result": {...}}`.
    pub async fn get_user_limits(&self) -> Result<UserLimits> {
        self.post_secure(
            "/api/v3/user/limits",
            &serde_json::Value::Object(Default::default()),
        )
        .await
    }

    // ------------------------------------------------------------------
    // Coin conversion history
    // ------------------------------------------------------------------

    /// Fetch the user's coin conversion (dust-to-KUB) history.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v3/user/coin-convert-history` (secure)
    ///
    /// # Arguments
    ///
    /// * `page` -- Page number (1-based). Server default when `None`.
    /// * `limit` -- Number of records per page.
    pub async fn get_coin_convert_history(
        &self,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<Vec<CoinConvertHistory>> {
        let page_str = page.map(|p| p.to_string());
        let lmt_str = limit.map(|l| l.to_string());
        let mut params: Vec<(&str, &str)> = Vec::new();
        if let Some(ref p) = page_str {
            params.push(("p", p));
        }
        if let Some(ref lmt) = lmt_str {
            params.push(("lmt", lmt));
        }
        self.get_secure("/api/v3/user/coin-convert-history", &params)
            .await
    }
}
