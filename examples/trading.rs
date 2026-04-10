//! Example: Authenticated trading operations
//!
//! Run with:
//!   BITKUB_API_KEY=your_key BITKUB_API_SECRET=your_secret cargo run --example trading
//!
//! WARNING: This example places a real limit buy order if uncommented.
//! Review carefully before running.

use bitkub::BitkubClient;
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("BITKUB_API_KEY")
        .expect("Set BITKUB_API_KEY environment variable");
    let api_secret = std::env::var("BITKUB_API_SECRET")
        .expect("Set BITKUB_API_SECRET environment variable");

    let client = BitkubClient::builder()
        .with_credentials(&api_key, &api_secret)
        .build()?;

    // 1. Check wallet balances
    println!("=== Wallet ===");
    let wallet = client.get_wallet().await?;
    for (coin, balance) in &wallet {
        if !balance.is_zero() {
            println!("  {coin}: {balance}");
        }
    }

    // 2. Detailed balances (available + reserved)
    println!("\n=== Balances ===");
    let balances = client.get_balances().await?;
    for (coin, bal) in &balances {
        if !bal.available.is_zero() || !bal.reserved.is_zero() {
            println!("  {coin}: available={} reserved={}", bal.available, bal.reserved);
        }
    }

    // 3. User limits
    println!("\n=== User Limits ===");
    let limits = client.get_user_limits().await?;
    println!("  Crypto deposit limit: {} BTC", limits.limits.crypto.deposit);
    println!("  Crypto withdraw limit: {} BTC", limits.limits.crypto.withdraw);
    println!("  Fiat deposit limit: {} THB", limits.limits.fiat.deposit);
    println!("  Fiat withdraw limit: {} THB", limits.limits.fiat.withdraw);
    println!("  THB rate: {}", limits.rate);

    // 4. Trading credits
    let credits = client.get_trading_credits().await?;
    println!("\n=== Trading Credits ===");
    println!("  Credits: {credits}");

    // 5. Open orders for BTC_THB
    println!("\n=== Open Orders (BTC_THB) ===");
    let open_orders = client.get_my_open_orders("btc_thb").await?;
    if open_orders.is_empty() {
        println!("  No open orders.");
    }
    for order in &open_orders {
        println!(
            "  id={} {} {} rate={} amount={}",
            order.id, order.side, order.order_type, order.rate, order.amount
        );
    }

    // 6. Place a limit buy order (UNCOMMENT TO ACTUALLY PLACE)
    // use bitkub::models::trading::PlaceOrderRequest;
    // let order = PlaceOrderRequest {
    //     sym: "btc_thb".to_string(),
    //     amt: dec!(100),       // 100 THB
    //     rat: dec!(1000000),   // rate 1,000,000 THB/BTC (well below market)
    //     typ: "limit".to_string(),
    //     client_id: Some("rust-example-001".to_string()),
    //     post_only: Some(true),
    // };
    // let result = client.place_bid(&order).await?;
    // println!("\nOrder placed: id={} fee={} receive={}", result.id, result.fee, result.rec);

    // 7. Cancel an order (UNCOMMENT AND SET ORDER ID)
    // use bitkub::models::trading::CancelOrderRequest;
    // let cancel = CancelOrderRequest {
    //     sym: "btc_thb".to_string(),
    //     id: "ORDER_ID_HERE".to_string(),
    //     sd: "buy".to_string(),
    // };
    // client.cancel_order(&cancel).await?;
    // println!("Order cancelled.");

    let _ = dec!(0); // suppress unused import warning

    Ok(())
}
