//! Rust client library for the Bitkub cryptocurrency exchange API.
//!
//! This crate provides an async REST client and WebSocket clients for
//! interacting with the [Bitkub](https://www.bitkub.com/) exchange.
//!
//! # Quick start
//!
//! ```no_run
//! use bitkub::{BitkubClient, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Public endpoints need no credentials.
//!     let client = BitkubClient::new();
//!     let symbols = client.get_symbols().await?;
//!     println!("found {} symbols", symbols.len());
//!
//!     // Authenticated endpoints require API key and secret.
//!     let client = BitkubClient::builder()
//!         .with_credentials("YOUR_API_KEY", "YOUR_API_SECRET")
//!         .build()?;
//!     let wallet = client.get_wallet().await?;
//!     println!("wallet: {wallet:?}");
//!
//!     Ok(())
//! }
//! ```

pub mod auth;
pub mod client;
pub mod error;
pub mod models;
pub mod rest;
pub mod ws;

pub use client::{BitkubClient, BitkubClientBuilder};
pub use error::{BitkubError, Result};
