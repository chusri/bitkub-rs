# bitkub-rs

Rust client library for the [Bitkub](https://www.bitkub.com/) cryptocurrency exchange API.

## Features

- **REST API V3** — Public market data, trading, user info, fiat operations
- **REST API V4** — Crypto addresses, deposits, withdrawals, coins
- **WebSocket** — Public trade/ticker streams, live orderbook, private order/match updates
- **HMAC-SHA256 authentication** for both REST and WebSocket
- **Async/await** with Tokio
- **Decimal precision** via `rust_decimal` for all financial values

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bitkub = { path = "../bitkub-rs" }
tokio = { version = "1", features = ["full"] }
```

## Quick Start

### Public Market Data

```rust
use bitkub::BitkubClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BitkubClient::new();

    // Server time
    let time = client.get_server_time().await?;
    println!("Server time: {time}");

    // Ticker
    let tickers = client.get_ticker(Some("btc_thb")).await?;
    for (symbol, t) in &tickers {
        println!("{symbol}: last={} bid={} ask={}", t.last, t.highest_bid, t.lowest_ask);
    }

    // Depth
    let depth = client.get_depth("btc_thb", Some(5)).await?;
    for (price, size) in &depth.bids {
        println!("bid: {price} x {size}");
    }

    Ok(())
}
```

### Authenticated Trading

```rust
use bitkub::BitkubClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BitkubClient::builder()
        .with_credentials("your_api_key", "your_api_secret")
        .build()?;

    // Wallet balances
    let wallet = client.get_wallet().await?;
    for (coin, balance) in &wallet {
        println!("{coin}: {balance}");
    }

    // Open orders
    let orders = client.get_my_open_orders("btc_thb").await?;
    for o in &orders {
        println!("{} {} @ {}", o.side, o.amount, o.rate);
    }

    Ok(())
}
```

### WebSocket Orderbook

```rust
use bitkub::ws::orderbook::{OrderBookClient, OrderBookMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = OrderBookClient::new(1); // BTC_THB = pairing_id 1

    let (mut msg_rx, mut err_rx) = client.connect().await?;

    loop {
        tokio::select! {
            Some(msg) = msg_rx.recv() => {
                match msg {
                    OrderBookMessage::TradesChanged(e) => {
                        println!("{} bids, {} asks", e.bids.len(), e.asks.len());
                    }
                    _ => {}
                }
            }
            Some(err) = err_rx.recv() => eprintln!("Error: {err}"),
            else => break,
        }
    }
    Ok(())
}
```

### Private WebSocket

```rust
use bitkub::auth::Credentials;
use bitkub::ws::private::{PrivateWsClient, PrivateWsMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = Credentials::new("your_api_key", "your_api_secret");
    let mut client = PrivateWsClient::new(creds);

    let (mut msg_rx, _err_rx) = client.connect(&["order_update", "match_update"]).await?;

    while let Some(msg) = msg_rx.recv().await {
        match msg {
            PrivateWsMessage::OrderUpdate(o) => {
                println!("Order {} {}: {}", o.order_id, o.symbol, o.status);
            }
            PrivateWsMessage::MatchUpdate(m) => {
                println!("Match {} @ {}", m.txn_id, m.price);
            }
            _ => {}
        }
    }
    Ok(())
}
```

## API Coverage

### REST V3 (Public)

| Endpoint | Method |
|----------|--------|
| `GET /api/status` | `get_status()` |
| `GET /api/v3/servertime` | `get_server_time()` |
| `GET /api/v3/market/symbols` | `get_symbols()` |
| `GET /api/v3/market/ticker` | `get_ticker()` |
| `GET /api/v3/market/bids` | `get_bids()` |
| `GET /api/v3/market/asks` | `get_asks()` |
| `GET /api/v3/market/depth` | `get_depth()` |
| `GET /api/v3/market/trades` | `get_trades()` |
| `GET /tradingview/history` | `get_tradingview_history()` |

### REST V3 (Authenticated)

| Endpoint | Method |
|----------|--------|
| `POST /api/v3/market/wallet` | `get_wallet()` |
| `POST /api/v3/market/balances` | `get_balances()` |
| `POST /api/v3/market/place-bid` | `place_bid()` |
| `POST /api/v3/market/place-ask` | `place_ask()` |
| `POST /api/v3/market/cancel-order` | `cancel_order()` |
| `POST /api/v3/market/wstoken` | `get_ws_token()` |
| `GET /api/v3/market/my-open-orders` | `get_my_open_orders()` |
| `GET /api/v3/market/my-order-history` | `get_my_order_history()` |
| `GET /api/v3/market/order-info` | `get_order_info()` |
| `POST /api/v3/user/trading-credits` | `get_trading_credits()` |
| `POST /api/v3/user/limits` | `get_user_limits()` |
| `GET /api/v3/user/coin-convert-history` | `get_coin_convert_history()` |
| `POST /api/v3/fiat/accounts` | `get_fiat_accounts()` |
| `POST /api/v3/fiat/withdraw` | `fiat_withdraw()` |
| `POST /api/v3/fiat/deposit-history` | `get_fiat_deposit_history()` |
| `POST /api/v3/fiat/withdraw-history` | `get_fiat_withdraw_history()` |

### REST V4 (Crypto)

| Endpoint | Method |
|----------|--------|
| `GET /api/v4/crypto/addresses` | `get_crypto_addresses()` |
| `POST /api/v4/crypto/addresses` | `generate_crypto_address()` |
| `GET /api/v4/crypto/deposits` | `get_crypto_deposits()` |
| `GET /api/v4/crypto/withdraws` | `get_crypto_withdraws()` |
| `POST /api/v4/crypto/withdraws` | `crypto_withdraw()` |
| `GET /api/v4/crypto/coins` | `get_crypto_coins()` |
| `GET /api/v4/crypto/compensations` | `get_crypto_compensations()` |

### WebSocket

| Stream | Client |
|--------|--------|
| `market.trade.<symbol>` | `PublicWsClient` |
| `market.ticker.<symbol>` | `PublicWsClient` |
| `orderbook/<symbol-id>` | `OrderBookClient` |
| Private `order_update` | `PrivateWsClient` |
| Private `match_update` | `PrivateWsClient` |

## Examples

```sh
# Public market data
cargo run --example market_data

# Live orderbook WebSocket
cargo run --example orderbook_ws

# Authenticated trading (requires API keys)
BITKUB_API_KEY=... BITKUB_API_SECRET=... cargo run --example trading

# Private WebSocket (requires API keys)
BITKUB_API_KEY=... BITKUB_API_SECRET=... cargo run --example private_ws
```

## License

MIT
