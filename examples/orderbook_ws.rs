//! Example: Stream live orderbook updates via WebSocket
//!
//! Run with: cargo run --example orderbook_ws
//!
//! This connects to the Bitkub orderbook WebSocket for BTC_THB (pairing_id=1)
//! and prints orderbook events as they arrive.

use bitkub::ws::orderbook::{OrderBookClient, OrderBookMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // BTC_THB has pairing_id = 1
    let symbol_id = 1;
    let mut client = OrderBookClient::new(symbol_id);

    println!("Connecting to orderbook stream for symbol_id={symbol_id}...");
    let (mut msg_rx, mut err_rx) = client.connect().await?;
    println!("Connected! Listening for events...\n");

    loop {
        tokio::select! {
            Some(msg) = msg_rx.recv() => {
                match msg {
                    OrderBookMessage::BidsChanged(event) => {
                        println!(
                            "[bidschanged] pairing_id={} — {} bid levels",
                            event.pairing_id,
                            event.data.len()
                        );
                        if let Some(best) = event.data.first() {
                            println!("  best bid: {} @ {}", best.amount, best.rate);
                        }
                    }
                    OrderBookMessage::AsksChanged(event) => {
                        println!(
                            "[askschanged] pairing_id={} — {} ask levels",
                            event.pairing_id,
                            event.data.len()
                        );
                        if let Some(best) = event.data.first() {
                            println!("  best ask: {} @ {}", best.amount, best.rate);
                        }
                    }
                    OrderBookMessage::TradesChanged(event) => {
                        println!(
                            "[tradeschanged] pairing_id={} — {} trades, {} bids, {} asks",
                            event.pairing_id,
                            event.trades.len(),
                            event.bids.len(),
                            event.asks.len()
                        );
                        if let Some(trade) = event.trades.first() {
                            println!(
                                "  latest trade: {} {} @ {}",
                                trade.side, trade.amount, trade.rate
                            );
                        }
                    }
                    OrderBookMessage::Ticker(event) => {
                        println!(
                            "[ticker] pairing_id={} — last={} bid={} ask={}",
                            event.pairing_id,
                            event.data.last,
                            event.data.highest_bid,
                            event.data.lowest_ask
                        );
                    }
                    OrderBookMessage::GlobalTicker(event) => {
                        println!(
                            "[global.ticker] last={} bid={} ask={}",
                            event.data.last,
                            event.data.highest_bid,
                            event.data.lowest_ask
                        );
                    }
                }
            }
            Some(err) = err_rx.recv() => {
                eprintln!("Error: {err}");
            }
            else => {
                println!("All channels closed, exiting.");
                break;
            }
        }
    }

    Ok(())
}
