//! Fiat deposit and withdrawal endpoints.
//!
//! Authenticated endpoints for managing Thai Baht (THB) bank accounts,
//! withdrawals, and deposit/withdrawal history.

use rust_decimal::Decimal;
use serde::Serialize;

use crate::client::BitkubClient;
use crate::error::Result;
use crate::models::fiat::{BankAccount, FiatDeposit, FiatWithdrawResponse, FiatWithdrawal};

/// Request body for fiat withdrawal.
#[derive(Debug, Clone, Serialize)]
struct FiatWithdrawBody {
    id: String,
    amt: Decimal,
}

impl BitkubClient {
    // ------------------------------------------------------------------
    // Bank accounts
    // ------------------------------------------------------------------

    /// List the user's linked bank accounts for fiat operations.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/fiat/accounts` (secure)
    ///
    /// # Arguments
    ///
    /// * `page` -- Page number (1-based).
    /// * `limit` -- Number of records per page.
    pub async fn get_fiat_accounts(
        &self,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<Vec<BankAccount>> {
        let mut body = serde_json::Map::new();
        if let Some(p) = page {
            body.insert("p".to_owned(), serde_json::Value::Number(p.into()));
        }
        if let Some(lim) = limit {
            body.insert("lmt".to_owned(), serde_json::Value::Number(lim.into()));
        }
        self.post_secure("/api/v3/fiat/accounts", &serde_json::Value::Object(body))
            .await
    }

    // ------------------------------------------------------------------
    // Fiat withdrawal
    // ------------------------------------------------------------------

    /// Initiate a fiat (THB) withdrawal to a linked bank account.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/fiat/withdraw` (secure)
    ///
    /// # Arguments
    ///
    /// * `account_id` -- The bank account identifier from [`get_fiat_accounts`](Self::get_fiat_accounts).
    /// * `amount` -- The amount in THB to withdraw.
    pub async fn fiat_withdraw(
        &self,
        account_id: &str,
        amount: Decimal,
    ) -> Result<FiatWithdrawResponse> {
        let body = FiatWithdrawBody {
            id: account_id.to_owned(),
            amt: amount,
        };
        self.post_secure("/api/v3/fiat/withdraw", &body).await
    }

    // ------------------------------------------------------------------
    // Fiat history
    // ------------------------------------------------------------------

    /// Fetch the user's fiat deposit history.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/fiat/deposit-history` (secure)
    ///
    /// # Arguments
    ///
    /// * `page` -- Page number (1-based).
    /// * `limit` -- Number of records per page.
    pub async fn get_fiat_deposit_history(
        &self,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<Vec<FiatDeposit>> {
        let mut body = serde_json::Map::new();
        if let Some(p) = page {
            body.insert("p".to_owned(), serde_json::Value::Number(p.into()));
        }
        if let Some(lim) = limit {
            body.insert("lmt".to_owned(), serde_json::Value::Number(lim.into()));
        }
        self.post_secure(
            "/api/v3/fiat/deposit-history",
            &serde_json::Value::Object(body),
        )
        .await
    }

    /// Fetch the user's fiat withdrawal history.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v3/fiat/withdraw-history` (secure)
    ///
    /// # Arguments
    ///
    /// * `page` -- Page number (1-based).
    /// * `limit` -- Number of records per page.
    pub async fn get_fiat_withdraw_history(
        &self,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<Vec<FiatWithdrawal>> {
        let mut body = serde_json::Map::new();
        if let Some(p) = page {
            body.insert("p".to_owned(), serde_json::Value::Number(p.into()));
        }
        if let Some(lim) = limit {
            body.insert("lmt".to_owned(), serde_json::Value::Number(lim.into()));
        }
        self.post_secure(
            "/api/v3/fiat/withdraw-history",
            &serde_json::Value::Object(body),
        )
        .await
    }
}
