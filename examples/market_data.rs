//! Example: Fetch public market data from Bitkub
//!
//! Run with: cargo run --example market_data

use bitkub::BitkubClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a public (unauthenticated) client
    let client = BitkubClient::new();

    // 1. Server time
    let server_time = client.get_server_time().await?;
    println!("Server time: {server_time}");

    // 2. API status
    let status = client.get_status().await?;
    for s in &status {
        println!("Endpoint: {} — Status: {} {}", s.name, s.status, s.message);
    }

    // 3. Symbols
    let symbols = client.get_symbols().await?;
    println!("\nAvailable symbols ({} total):", symbols.len());
    for sym in symbols.iter().take(5) {
        println!(
            "  {} (id={}) — {} [{}, step={}]",
            sym.symbol, sym.pairing_id, sym.description, sym.status, sym.price_step
        );
    }

    // 4. Ticker for BTC_THB
    let tickers = client.get_ticker(Some("btc_thb")).await?;
    for (symbol, t) in &tickers {
        println!(
            "\nTicker {symbol}: last={} bid={} ask={} vol={} change={}%",
            t.last, t.highest_bid, t.lowest_ask, t.base_volume, t.percent_change
        );
    }

    // 5. Depth (top 5 levels)
    let depth = client.get_depth("btc_thb", Some(5)).await?;
    println!("\nOrder book depth (BTC_THB):");
    println!("  Bids:");
    for (price, size) in &depth.bids {
        println!("    {price} x {size}");
    }
    println!("  Asks:");
    for (price, size) in &depth.asks {
        println!("    {price} x {size}");
    }

    // 6. Recent trades
    let trades = client.get_trades("btc_thb", Some(5)).await?;
    println!("\nRecent trades (BTC_THB):");
    for t in &trades {
        println!("  {} {} @ {} ({})", t.side, t.amount, t.rate, t.timestamp);
    }

    Ok(())
}
