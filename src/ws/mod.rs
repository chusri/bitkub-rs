//! WebSocket clients for the Bitkub exchange.
//!
//! Three client types are provided:
//!
//! - [`public::PublicWsClient`] -- public trade and ticker streams
//! - [`orderbook::OrderBookClient`] -- live order book events
//! - [`private::PrivateWsClient`] -- authenticated order/match updates

pub mod orderbook;
pub mod private;
pub mod public;
