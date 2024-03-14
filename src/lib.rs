//! # 📖 OpenBook
//!
//! A Rust library and CLI for interacting with the OpenBook market on the Solana blockchain.
//!
//! ## Quick Start
//!
//! Before using the `openbook` crate or CLI, make sure to set the following environment variables:
//!
//! ```bash
//! export RPC_URL=https://api.mainnet-beta.solana.com
//! export KEY_PATH=<path_to_your_key_file>
//! ```
//!
//! Get started with the `openbook` crate by following these simple steps:
//!
//! 1. Install the `openbook` crate by adding the following line to your `Cargo.toml` file:
//!
//! ```toml
//! [dependencies]
//! openbook = "0.0.2"
//! ```
//!
//! 2. Use the `Market` struct to perform various operations in the OpenBook market:
//!
//! ```rust
//! use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
//! use openbook::market::Market;
//! use openbook::utils::read_keypair;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
//!     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
//!    
//!     let rpc_client = RpcClient::new(rpc_url);
//!    
//!     let keypair = read_keypair(&key_path);
//!    
//!     let mut market = Market::new(rpc_client, 3, "openbook", keypair).await;
//!    
//!     println!("Initialized Market: {:?}", market);
//!
//!     let max_bid = 1;
//!     let r = market.place_limit_bid(max_bid).await?;
//!     println!("Place Order Results: {:?}", r);
//!
//!     let order_id_to_cancel = 2;
//!     let c = market.cancel_order(order_id_to_cancel).await?;
//!     println!("Cancel Order Results: {:?}", c);
//!
//!     let s = market.settle_balance().await?;
//!     println!("Settle Balance Results: {:?}", s);
//!
//!     let m = market.make_match_orders_transaction(1).await?;
//!     println!("Match Order Results: {:?}", m);
//!
//!     let open_orders_accounts = vec![Pubkey::new_from_array([0; 32])];
//!     let limit = 10;
//!
//!     let e = market.make_consume_events_instruction(open_orders_accounts.clone(), limit).await?;
//!     println!("Consume Events Results: {:?}", e);
//!
//!     let p =
//!         market.make_consume_events_permissioned_instruction(open_orders_accounts.clone(), limit).await?;
//!     println!("Consume Events Permissioned Results: {:?}", p);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Options
//!
//! | Subcommand                           | Description                                              |
//! |--------------------------------------|----------------------------------------------------------|
//! | `place --bid <BID>`                  | Place a limit bid with the specified amount.             |
//! | `cancel --order <ORDER>`             | Cancel an existing order with the given order ID.        |
//! | `settle`                             | Settle balances in the OpenBook market.                  |
//! | `match --order <ORDER>`         | Match orders transaction with the specified number of orders to match. |
//! | `consume --limit <LIMIT>`       | Consume events instruction with the specified limit. |
//! | `consume-permissioned --limit <LIMIT>` | Consume events permissioned instruction with the specified limit. |
//! | `load --num <NUM>`                   | Load orders for a specific owner with the specified number. |
//! | `find-open-accounts`                 | Find open orders accounts for a specific owner.           |
//!
//! ## GitHub Repository
//!
//! You can access the source code for the `openbook` crate on [GitHub](https://github.com/wiseaidev/openbook).
//!
//! ## Contributing
//!
//! Contributions and feedback are welcome! If you'd like to contribute, report an issue, or suggest an enhancement,
//! please engage with the project on [GitHub](https://github.com/wiseaidev/openbook).
//! Your contributions help improve this crate and CLI for the community.

#[cfg(feature = "cli")]
pub mod cli;
pub mod fees;
pub mod market;
pub mod orders;
pub mod tokens_and_markets;
pub mod utils;

// Re-export common func
pub use openbook_dex::state;
pub use solana_client::rpc_filter;
pub use solana_program::pubkey;
pub use solana_rpc_client::nonblocking::rpc_client;
pub use solana_sdk::account;
pub use solana_sdk::bs58;
pub use solana_sdk::signature;
pub use solana_sdk::signer::keypair;
