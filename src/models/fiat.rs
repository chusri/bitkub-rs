use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A linked bank account from `POST /api/v3/fiat/accounts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    pub id: String,
    pub bank: String,
    pub name: String,
    pub time: i64,
}

/// Request body for a fiat withdrawal via `POST /api/v3/fiat/withdraw`.
#[derive(Debug, Clone, Serialize)]
pub struct FiatWithdrawRequest {
    pub id: String,
    pub amt: Decimal,
}

/// Response after initiating a fiat withdrawal.
#[derive(Debug, Clone, Deserialize)]
pub struct FiatWithdrawResponse {
    pub txn: String,
    pub acc: String,
    pub cur: String,
    pub amt: Decimal,
    pub fee: Decimal,
    pub rec: Decimal,
    pub ts: i64,
}

/// A fiat deposit record from `POST /api/v3/fiat/deposit-history`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiatDeposit {
    pub txn_id: String,
    pub currency: String,
    pub amount: Decimal,
    pub status: String,
    pub time: i64,
}

/// A fiat withdrawal record from `POST /api/v3/fiat/withdraw-history`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiatWithdrawal {
    pub txn_id: String,
    pub currency: String,
    pub amount: Decimal,
    pub fee: Decimal,
    pub status: String,
    pub time: i64,
}
