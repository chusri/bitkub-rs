//! Error types for the Bitkub client library.
//!
//! The Bitkub API uses two distinct error formats:
//!
//! - **V3**: `{"error": <int>, "result": ...}` where `0` means success.
//! - **V4**: `{"code": "<string>", "message": "<msg>", "data": ...}` where `"0"` means success.
//!
//! This module provides [`BitkubError`] to unify all error sources and
//! [`Result<T>`] as a convenience alias.

use std::fmt;

/// Convenience type alias used throughout the crate.
pub type Result<T> = std::result::Result<T, BitkubError>;

/// Top-level error type for all Bitkub client operations.
#[derive(Debug, thiserror::Error)]
pub enum BitkubError {
    /// V3 API error with an integer error code.
    ///
    /// The `code` field corresponds to the Bitkub V3 error code table
    /// (e.g., 1 = invalid JSON, 2 = missing API key).
    #[error("bitkub api error {code}: {message}")]
    Api { code: i32, message: String },

    /// V4 API error with a string error code.
    ///
    /// Codes follow the pattern `"B1000-CW"`, `"V1007-CW"`, `"A1001-CW"`, etc.
    #[error("bitkub v4 api error {code}: {message}")]
    ApiV4 { code: String, message: String },

    /// HTTP transport error from `reqwest`.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// WebSocket transport error from `tungstenite`.
    #[error("websocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// JSON serialization or deserialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Authentication error (missing or invalid credentials).
    #[error("auth error: {0}")]
    Auth(String),

    /// Invalid parameter passed to an API method.
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// HTTP 429 Too Many Requests.
    #[error("rate limited")]
    RateLimited,

    /// Internal client error that does not fit other categories.
    #[error("internal error: {0}")]
    Internal(String),
}

// ---------------------------------------------------------------------------
// V3 error code mapping
// ---------------------------------------------------------------------------

/// Known Bitkub V3 API error codes and their human-readable descriptions.
///
/// Derived from the official Bitkub API documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorCode {
    InvalidJson = 1,
    MissingApiKey = 2,
    InvalidApiKey = 3,
    ApiPending = 4,
    IpNotAllowed = 5,
    InvalidSignature = 6,
    MissingTimestamp = 7,
    InvalidTimestamp = 8,
    InvalidUser = 9,
    InvalidParameter = 10,
    InvalidSymbol = 11,
    InvalidAmount = 12,
    InvalidRate = 13,
    ImproperRate = 14,
    AmountTooLow = 15,
    FailedGetBalance = 16,
    WalletEmpty = 17,
    InsufficientBalance = 18,
    FailedInsertOrder = 19,
    FailedDeductBalance = 20,
    InvalidOrderCancel = 21,
    InvalidSide = 22,
    FailedUpdateOrder = 23,
    InvalidOrderLookup = 24,
    KycRequired = 25,
    LimitExceeded = 30,
    PendingWithdrawal = 40,
    InvalidWithdrawCurrency = 41,
    AddressNotWhitelisted = 42,
    FailedDeductCrypto = 43,
    FailedCreateWithdrawal = 44,
    WithdrawalLimitExceeded = 47,
    InvalidBankAccount = 48,
    BankLimitExceeded = 49,
    PendingTransaction = 50,
    WithdrawalMaintenance = 51,
    InvalidPermission = 52,
    InvalidInternalAddress = 53,
    AddressDeprecated = 54,
    CancelOnlyMode = 55,
    SuspendedPurchasing = 56,
    SuspendedSelling = 57,
    BankNotVerified = 58,
    BrokerCoinNotSupported = 61,
    ServerError = 90,
}

impl ApiErrorCode {
    /// Attempt to convert a raw integer code to a known variant.
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            1 => Some(Self::InvalidJson),
            2 => Some(Self::MissingApiKey),
            3 => Some(Self::InvalidApiKey),
            4 => Some(Self::ApiPending),
            5 => Some(Self::IpNotAllowed),
            6 => Some(Self::InvalidSignature),
            7 => Some(Self::MissingTimestamp),
            8 => Some(Self::InvalidTimestamp),
            9 => Some(Self::InvalidUser),
            10 => Some(Self::InvalidParameter),
            11 => Some(Self::InvalidSymbol),
            12 => Some(Self::InvalidAmount),
            13 => Some(Self::InvalidRate),
            14 => Some(Self::ImproperRate),
            15 => Some(Self::AmountTooLow),
            16 => Some(Self::FailedGetBalance),
            17 => Some(Self::WalletEmpty),
            18 => Some(Self::InsufficientBalance),
            19 => Some(Self::FailedInsertOrder),
            20 => Some(Self::FailedDeductBalance),
            21 => Some(Self::InvalidOrderCancel),
            22 => Some(Self::InvalidSide),
            23 => Some(Self::FailedUpdateOrder),
            24 => Some(Self::InvalidOrderLookup),
            25 => Some(Self::KycRequired),
            30 => Some(Self::LimitExceeded),
            40 => Some(Self::PendingWithdrawal),
            41 => Some(Self::InvalidWithdrawCurrency),
            42 => Some(Self::AddressNotWhitelisted),
            43 => Some(Self::FailedDeductCrypto),
            44 => Some(Self::FailedCreateWithdrawal),
            47 => Some(Self::WithdrawalLimitExceeded),
            48 => Some(Self::InvalidBankAccount),
            49 => Some(Self::BankLimitExceeded),
            50 => Some(Self::PendingTransaction),
            51 => Some(Self::WithdrawalMaintenance),
            52 => Some(Self::InvalidPermission),
            53 => Some(Self::InvalidInternalAddress),
            54 => Some(Self::AddressDeprecated),
            55 => Some(Self::CancelOnlyMode),
            56 => Some(Self::SuspendedPurchasing),
            57 => Some(Self::SuspendedSelling),
            58 => Some(Self::BankNotVerified),
            61 => Some(Self::BrokerCoinNotSupported),
            90 => Some(Self::ServerError),
            _ => None,
        }
    }

    /// Return the human-readable description for this error code.
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidJson => "invalid JSON payload",
            Self::MissingApiKey => "missing X-BTK-APIKEY",
            Self::InvalidApiKey => "invalid API key",
            Self::ApiPending => "API pending for activation",
            Self::IpNotAllowed => "IP not allowed",
            Self::InvalidSignature => "missing / invalid signature",
            Self::MissingTimestamp => "missing timestamp",
            Self::InvalidTimestamp => "invalid timestamp",
            Self::InvalidUser => "invalid user",
            Self::InvalidParameter => "invalid parameter",
            Self::InvalidSymbol => "invalid symbol",
            Self::InvalidAmount => "invalid amount",
            Self::InvalidRate => "invalid rate",
            Self::ImproperRate => "improper rate",
            Self::AmountTooLow => "amount too low",
            Self::FailedGetBalance => "failed to get balance",
            Self::WalletEmpty => "wallet is empty",
            Self::InsufficientBalance => "insufficient balance",
            Self::FailedInsertOrder => "failed to insert order into db",
            Self::FailedDeductBalance => "failed to deduct balance",
            Self::InvalidOrderCancel => "invalid order for cancellation",
            Self::InvalidSide => "invalid side",
            Self::FailedUpdateOrder => "failed to update order status",
            Self::InvalidOrderLookup => "invalid order for lookup",
            Self::KycRequired => "KYC level 1 is required",
            Self::LimitExceeded => "limit exceeds",
            Self::PendingWithdrawal => "pending withdrawal exists",
            Self::InvalidWithdrawCurrency => "invalid currency for withdrawal",
            Self::AddressNotWhitelisted => "address is not in whitelist",
            Self::FailedDeductCrypto => "failed to deduct crypto",
            Self::FailedCreateWithdrawal => "failed to create withdrawal record",
            Self::WithdrawalLimitExceeded => "withdrawal limit exceeds",
            Self::InvalidBankAccount => "invalid bank account",
            Self::BankLimitExceeded => "bank limit exceeds",
            Self::PendingTransaction => "pending withdrawal / transaction exists",
            Self::WithdrawalMaintenance => "withdrawal is under maintenance",
            Self::InvalidPermission => "invalid permission",
            Self::InvalidInternalAddress => "invalid internal address",
            Self::AddressDeprecated => "address has been deprecated",
            Self::CancelOnlyMode => "cancel only mode",
            Self::SuspendedPurchasing => "user suspended from purchasing",
            Self::SuspendedSelling => "user suspended from selling",
            Self::BankNotVerified => "user bank is not verified",
            Self::BrokerCoinNotSupported => "endpoint doesn't support broker coins",
            Self::ServerError => "server error",
        }
    }
}

impl fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

/// Convert a V3 integer error code into a [`BitkubError::Api`].
///
/// If the code is known, the message is derived from [`ApiErrorCode`].
/// Unknown codes produce a generic "unknown error" message.
pub fn map_api_error(code: i32) -> BitkubError {
    let message = ApiErrorCode::from_code(code)
        .map(|c| c.message().to_owned())
        .unwrap_or_else(|| format!("unknown error {code}"));
    BitkubError::Api { code, message }
}

// ---------------------------------------------------------------------------
// V4 error code constants
// ---------------------------------------------------------------------------

/// Known V4 error code constants derived from the Bitkub V4 API documentation.
pub mod v4_codes {
    // Server errors
    pub const INTERNAL_SERVICE_ERROR: &str = "S1000-CW";

    // Business errors
    pub const ACCOUNT_SUSPENDED: &str = "B1000-CW";
    pub const NETWORK_DISABLED: &str = "B1001-CW";
    pub const WALLET_NOT_FOUND: &str = "B1002-CW";
    pub const INSUFFICIENT_BALANCE: &str = "B1003-CW";
    pub const USER_MISMATCH: &str = "B1004-CW";
    pub const DUPLICATE_KEY: &str = "B1005-CW";
    pub const AIRDROP_ALREADY_TRANSFER: &str = "B1006-CW";
    pub const SYMBOL_REQUIRED: &str = "B1007-CW";
    pub const EVENT_SYMBOL_MISMATCHED: &str = "B1008-CW";
    pub const PENDING_WITHDRAWAL: &str = "B1009-CW";
    pub const ACCOUNT_FROZEN: &str = "B1010-CW";
    pub const WITHDRAWAL_EXCEEDS_LIMIT: &str = "B1011-CW";
    pub const ADDRESS_NOT_TRUSTED: &str = "B1012-CW";
    pub const WITHDRAWAL_FROZEN: &str = "B1013-CW";
    pub const ADDRESS_NOT_WHITELISTED: &str = "B1014-CW";
    pub const REQUEST_PROCESSING: &str = "B1015-CW";
    pub const DEPOSIT_FROZEN: &str = "B1016-CW";

    // Validation errors
    pub const USER_NOT_FOUND: &str = "V1000-CW";
    pub const ASSET_NOT_FOUND: &str = "V1001-CW";
    pub const EVENT_NOT_FOUND: &str = "V1002-CW";
    pub const INVALID_SIGNATURE: &str = "V1003-CW";
    pub const SIGNATURE_EXPIRED: &str = "V1004-CW";
    pub const TRANSACTION_NOT_FOUND: &str = "V1005-CW";
    pub const INVALID_PARAMETER: &str = "V1006-CW";
    pub const SYMBOL_NOT_FOUND: &str = "V1007-CW";
    pub const ADDRESS_NOT_YET_GENERATED: &str = "V1008-CW";
    pub const MEMO_NOT_FOUND: &str = "V1009-CW";
    pub const ADDRESS_NOT_FOUND: &str = "V1010-CW";
    pub const ADDRESS_ALREADY_EXISTS: &str = "V1011-CW";
    pub const DESTINATION_NOT_ACTIVE: &str = "V1012-CW";
    pub const COIN_NOT_FOUND: &str = "V1015-CW";

    // Authentication errors
    pub const UNAUTHORIZED: &str = "A1000-CW";
    pub const PERMISSION_DENIED: &str = "A1001-CW";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_known_error_code() {
        let err = map_api_error(18);
        match err {
            BitkubError::Api { code, message } => {
                assert_eq!(code, 18);
                assert_eq!(message, "insufficient balance");
            }
            other => panic!("unexpected variant: {other}"),
        }
    }

    #[test]
    fn map_unknown_error_code() {
        let err = map_api_error(999);
        match err {
            BitkubError::Api { code, message } => {
                assert_eq!(code, 999);
                assert_eq!(message, "unknown error 999");
            }
            other => panic!("unexpected variant: {other}"),
        }
    }

    #[test]
    fn api_error_code_round_trip() {
        for code in [1, 2, 3, 10, 18, 25, 30, 42, 55, 90] {
            let variant = ApiErrorCode::from_code(code);
            assert!(variant.is_some(), "code {code} should be known");
            assert_eq!(variant.unwrap() as i32, code);
        }
    }

    #[test]
    fn display_formatting() {
        let err = BitkubError::Api {
            code: 6,
            message: "missing / invalid signature".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "bitkub api error 6: missing / invalid signature"
        );

        let err_v4 = BitkubError::ApiV4 {
            code: "B1003-CW".to_owned(),
            message: "insufficient balance".to_owned(),
        };
        assert_eq!(
            err_v4.to_string(),
            "bitkub v4 api error B1003-CW: insufficient balance"
        );
    }

    #[test]
    fn rate_limited_display() {
        let err = BitkubError::RateLimited;
        assert_eq!(err.to_string(), "rate limited");
    }
}
