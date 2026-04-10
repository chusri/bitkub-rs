//! REST API endpoint implementations for the Bitkub exchange.
//!
//! Endpoints are organized by domain and implemented as methods on
//! [`BitkubClient`](crate::client::BitkubClient):
//!
//! - [`public`] -- Unauthenticated market data (status, ticker, orderbook, trades).
//! - [`market`] -- Authenticated trading operations (wallet, orders, cancellation).
//! - [`user`] -- User account information (limits, trading credits).
//! - [`fiat`] -- Fiat deposit/withdrawal operations.

pub mod crypto;
pub mod fiat;
pub mod market;
pub mod public;
pub mod user;
