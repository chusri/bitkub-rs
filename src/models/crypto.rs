use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A crypto deposit address from `GET /api/v4/crypto/addresses`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoAddress {
    pub symbol: String,
    pub network: String,
    pub address: String,
    pub memo: Option<String>,
    pub created_at: Option<String>,
}

/// Request body to generate a new deposit address via `POST /api/v4/crypto/addresses`.
#[derive(Debug, Clone, Serialize)]
pub struct GenerateAddressRequest {
    pub symbol: String,
    pub network: String,
}

/// A crypto deposit record from `GET /api/v4/crypto/deposits`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoDeposit {
    pub hash: String,
    pub symbol: String,
    pub network: String,
    pub amount: Decimal,
    pub from_address: Option<String>,
    pub to_address: String,
    pub confirmations: i32,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// A crypto withdrawal record from `GET /api/v4/crypto/withdraws`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoWithdrawal {
    pub txn_id: String,
    pub external_ref: Option<String>,
    pub hash: Option<String>,
    pub symbol: String,
    pub network: String,
    pub amount: Decimal,
    pub fee: Decimal,
    pub address: String,
    pub memo: Option<String>,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Request body to initiate a crypto withdrawal via `POST /api/v4/crypto/withdraws`.
#[derive(Debug, Clone, Serialize)]
pub struct CryptoWithdrawRequest {
    pub symbol: String,
    pub amount: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    pub network: String,
}

/// Coin metadata from `GET /api/v4/crypto/coins`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinInfo {
    pub name: String,
    pub symbol: String,
    pub networks: Vec<NetworkInfo>,
    pub deposit_enable: bool,
    pub withdraw_enable: bool,
}

/// Network-specific information for a coin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub name: String,
    pub network: String,
    pub address_regex: String,
    pub memo_regex: String,
    pub explorer: String,
    pub contract_address: String,
    pub withdraw_min: Decimal,
    pub withdraw_fee: Decimal,
    pub withdraw_internal_min: Option<String>,
    pub withdraw_internal_fee: Option<String>,
    pub withdraw_decimal_places: i32,
    pub min_confirm: i32,
    pub decimal: i32,
    pub deposit_enable: bool,
    pub withdraw_enable: bool,
    pub is_memo: bool,
}

/// A compensation record from `GET /api/v4/crypto/compensations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoCompensation {
    pub txn_id: String,
    pub symbol: String,
    #[serde(rename = "type")]
    pub compensation_type: String,
    pub amount: Decimal,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub user_id: String,
}
