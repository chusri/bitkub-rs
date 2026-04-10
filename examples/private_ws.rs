//! Example: Private WebSocket — order and match updates
//!
//! Run with:
//!   BITKUB_API_KEY=your_key BITKUB_API_SECRET=your_secret cargo run --example private_ws

use bitkub::auth::Credentials;
use bitkub::ws::private::{PrivateWsClient, PrivateWsMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("BITKUB_API_KEY")
        .expect("Set BITKUB_API_KEY environment variable");
    let api_secret = std::env::var("BITKUB_API_SECRET")
        .expect("Set BITKUB_API_SECRET environment variable");

    let credentials = Credentials::new(api_key, api_secret);
    let mut client = PrivateWsClient::new(credentials);

    println!("Connecting to private WebSocket...");
    let (mut msg_rx, mut err_rx) = client
        .connect(&["order_update", "match_update"])
        .await?;
    println!("Connected and subscribed!\n");

    loop {
        tokio::select! {
            Some(msg) = msg_rx.recv() => {
                match msg {
                    PrivateWsMessage::Authenticated => {
                        println!("[auth] Successfully authenticated");
                    }
                    PrivateWsMessage::Subscribed(channel) => {
                        println!("[subscribed] Channel: {channel}");
                    }
                    PrivateWsMessage::OrderUpdate(order) => {
                        println!(
                            "[order_update] id={} sym={} side={} status={} amt={} filled={}",
                            order.order_id,
                            order.symbol,
                            order.side,
                            order.status,
                            order.order_amount,
                            order.executed_amount,
                        );
                    }
                    PrivateWsMessage::MatchUpdate(trade) => {
                        println!(
                            "[match_update] order={} txn={} sym={} side={} price={} exec={} recv={}",
                            trade.order_id,
                            trade.txn_id,
                            trade.symbol,
                            trade.side,
                            trade.price,
                            trade.executed_amount,
                            trade.received_amount,
                        );
                    }
                    PrivateWsMessage::Pong => {
                        println!("[pong] Keep-alive received");
                    }
                }
            }
            Some(err) = err_rx.recv() => {
                eprintln!("Error: {err}");
            }
            else => {
                println!("Connection closed, exiting.");
                break;
            }
        }
    }

    Ok(())
}
