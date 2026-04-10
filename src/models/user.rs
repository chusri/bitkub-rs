use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// User trading limits from `POST /api/v3/user/limits`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLimits {
    pub limits: LimitInfo,
    pub usage: UsageInfo,
    pub rate: Decimal,
}

/// Deposit and withdrawal limits grouped by asset class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitInfo {
    pub crypto: CryptoLimit,
    pub fiat: FiatLimit,
}

/// Crypto deposit and withdrawal limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoLimit {
    pub deposit: Decimal,
    pub withdraw: Decimal,
}

/// Fiat deposit and withdrawal limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiatLimit {
    pub deposit: Decimal,
    pub withdraw: Decimal,
}

/// Current usage against limits grouped by asset class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub crypto: CryptoUsage,
    pub fiat: FiatUsage,
}

/// Current crypto deposit and withdrawal usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoUsage {
    pub deposit: Decimal,
    pub withdraw: Decimal,
    pub deposit_percentage: Decimal,
    pub withdraw_percentage: Decimal,
    pub deposit_thb_equivalent: Decimal,
    pub withdraw_thb_equivalent: Decimal,
}

/// Current fiat deposit and withdrawal usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiatUsage {
    pub deposit: Decimal,
    pub withdraw: Decimal,
    pub deposit_percentage: Decimal,
    pub withdraw_percentage: Decimal,
}

/// A coin conversion history entry from `GET /api/v3/user/coin-convert-history`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinConvertHistory {
    pub transaction_id: String,
    pub status: String,
    pub amount: Decimal,
    pub from_currency: String,
    pub trading_fee_received: Decimal,
    pub timestamp: i64,
}
