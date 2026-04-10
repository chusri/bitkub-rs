//! V4 crypto deposit, withdrawal, and address management endpoints.
//!
//! These endpoints use the V4 API format with bearer-token authentication
//! and return paginated responses via [`PaginatedItems`].

use crate::client::BitkubClient;
use crate::error::Result;
use crate::models::common::PaginatedItems;
use crate::models::crypto::{
    CoinInfo, CryptoAddress, CryptoCompensation, CryptoDeposit, CryptoWithdrawRequest,
    CryptoWithdrawal, GenerateAddressRequest,
};

impl BitkubClient {
    // ------------------------------------------------------------------
    // Crypto addresses
    // ------------------------------------------------------------------

    /// List the user's crypto deposit addresses.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v4/crypto/addresses` (secure, V4)
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Filter by coin symbol (e.g. `"BTC"`).
    /// * `network` -- Filter by network (e.g. `"BTC"`, `"ERC20"`).
    /// * `page` -- Page number (1-based).
    /// * `limit` -- Number of items per page.
    pub async fn get_crypto_addresses(
        &self,
        symbol: Option<&str>,
        network: Option<&str>,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<PaginatedItems<CryptoAddress>> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(sym) = symbol {
            params.push(("symbol", sym.to_owned()));
        }
        if let Some(net) = network {
            params.push(("network", net.to_owned()));
        }
        if let Some(p) = page {
            params.push(("page", p.to_string()));
        }
        if let Some(lim) = limit {
            params.push(("limit", lim.to_string()));
        }
        self.get_v4("/api/v4/crypto/addresses", &params).await
    }

    /// Generate a new crypto deposit address for a given coin and network.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v4/crypto/addresses` (secure, V4)
    ///
    /// # Arguments
    ///
    /// * `req` -- Request specifying the symbol and network.
    pub async fn generate_crypto_address(
        &self,
        req: &GenerateAddressRequest,
    ) -> Result<Vec<CryptoAddress>> {
        self.post_v4("/api/v4/crypto/addresses", req).await
    }

    // ------------------------------------------------------------------
    // Crypto deposits
    // ------------------------------------------------------------------

    /// List the user's crypto deposit history.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v4/crypto/deposits` (secure, V4)
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Filter by coin symbol.
    /// * `status` -- Filter by deposit status.
    /// * `page` -- Page number (1-based).
    /// * `limit` -- Number of items per page.
    pub async fn get_crypto_deposits(
        &self,
        symbol: Option<&str>,
        status: Option<&str>,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<PaginatedItems<CryptoDeposit>> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(sym) = symbol {
            params.push(("symbol", sym.to_owned()));
        }
        if let Some(st) = status {
            params.push(("status", st.to_owned()));
        }
        if let Some(p) = page {
            params.push(("page", p.to_string()));
        }
        if let Some(lim) = limit {
            params.push(("limit", lim.to_string()));
        }
        self.get_v4("/api/v4/crypto/deposits", &params).await
    }

    // ------------------------------------------------------------------
    // Crypto withdrawals
    // ------------------------------------------------------------------

    /// List the user's crypto withdrawal history.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v4/crypto/withdraws` (secure, V4)
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Filter by coin symbol.
    /// * `status` -- Filter by withdrawal status.
    /// * `page` -- Page number (1-based).
    /// * `limit` -- Number of items per page.
    pub async fn get_crypto_withdraws(
        &self,
        symbol: Option<&str>,
        status: Option<&str>,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<PaginatedItems<CryptoWithdrawal>> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(sym) = symbol {
            params.push(("symbol", sym.to_owned()));
        }
        if let Some(st) = status {
            params.push(("status", st.to_owned()));
        }
        if let Some(p) = page {
            params.push(("page", p.to_string()));
        }
        if let Some(lim) = limit {
            params.push(("limit", lim.to_string()));
        }
        self.get_v4("/api/v4/crypto/withdraws", &params).await
    }

    /// Initiate a crypto withdrawal.
    ///
    /// # Bitkub API
    ///
    /// `POST /api/v4/crypto/withdraws` (secure, V4)
    ///
    /// # Arguments
    ///
    /// * `req` -- Withdrawal parameters including coin, network, address, memo,
    ///   and amount.
    pub async fn crypto_withdraw(
        &self,
        req: &CryptoWithdrawRequest,
    ) -> Result<CryptoWithdrawal> {
        self.post_v4("/api/v4/crypto/withdraws", req).await
    }

    // ------------------------------------------------------------------
    // Coin information
    // ------------------------------------------------------------------

    /// List available coins and their network configurations.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v4/crypto/coins` (secure, V4)
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Filter by coin symbol.
    /// * `network` -- Filter by network.
    pub async fn get_crypto_coins(
        &self,
        symbol: Option<&str>,
        network: Option<&str>,
    ) -> Result<PaginatedItems<CoinInfo>> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(sym) = symbol {
            params.push(("symbol", sym.to_owned()));
        }
        if let Some(net) = network {
            params.push(("network", net.to_owned()));
        }
        self.get_v4("/api/v4/crypto/coins", &params).await
    }

    // ------------------------------------------------------------------
    // Compensations
    // ------------------------------------------------------------------

    /// List the user's crypto compensation (airdrop/fork) history.
    ///
    /// # Bitkub API
    ///
    /// `GET /api/v4/crypto/compensations` (secure, V4)
    ///
    /// # Arguments
    ///
    /// * `symbol` -- Filter by coin symbol.
    /// * `compensation_type` -- Filter by compensation type.
    /// * `page` -- Page number (1-based).
    /// * `limit` -- Number of items per page.
    pub async fn get_crypto_compensations(
        &self,
        symbol: Option<&str>,
        compensation_type: Option<&str>,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<PaginatedItems<CryptoCompensation>> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(sym) = symbol {
            params.push(("symbol", sym.to_owned()));
        }
        if let Some(ct) = compensation_type {
            params.push(("type", ct.to_owned()));
        }
        if let Some(p) = page {
            params.push(("page", p.to_string()));
        }
        if let Some(lim) = limit {
            params.push(("limit", lim.to_string()));
        }
        self.get_v4("/api/v4/crypto/compensations", &params).await
    }
}
