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
//! openbook = "0.0.7"
//! ```
//!
//! 2. Use the `Market` struct to perform various operations in the OpenBook market:
//!
//! ```rust
//! use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
//! use openbook::market::Market;
//! use openbook::utils::read_keypair;
//! use openbook::matching::Side;
//! use openbook::commitment_config::CommitmentConfig;
//! use openbook::market::OrderReturnType;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
//!     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
//!
//!     let commitment_config = CommitmentConfig::confirmed();
//!     let rpc_client = RpcClient::new_with_commitment(rpc_url, commitment_config);
//!    
//!     let keypair = read_keypair(&key_path);
//!    
//!     let mut market = Market::new(rpc_client, 3, "usdc", keypair).await;
//!
//!     println!("Initialized Market: {:?}", market);
//!
//!     println!("[*] Place Limit Order");
//!     if let Some(ord_ret_type) = market
//!         .place_limit_order(
//!             10.0,
//!             Side::Bid, // or Side::Ask
//!             0.5,
//!             true,
//!             15.0,
//!         )
//!         .await?
//!     {
//!         match ord_ret_type {
//!             OrderReturnType::Instructions(insts) => {
//!                 println!("[*] Got Instructions: {:?}", insts);
//!             }
//!             OrderReturnType::Signature(sign) => {
//!                 println!("[*] Transaction successful, signature: {:?}", sign);
//!             }
//!         }
//!     }
//!
//!     println!("[*] Cancel Orders");
//!     if let Some(ord_ret_type) = market
//!         .cancel_orders(
//!             true
//!         )
//!         .await?
//!     {
//!         match ord_ret_type {
//!             OrderReturnType::Instructions(insts) => {
//!                 println!("[*] Got Instructions: {:?}", insts);
//!             }
//!             OrderReturnType::Signature(sign) => {
//!                 println!("[*] Transaction successful, signature: {:?}", sign);
//!             }
//!         }
//!     }
//!
//!     println!("[*] Settle Balance");
//!     if let Some(ord_ret_type) = market
//!         .settle_balance(
//!             true
//!         )
//!         .await?
//!     {
//!         match ord_ret_type {
//!             OrderReturnType::Instructions(insts) => {
//!                 println!("[*] Got Instructions: {:?}", insts);
//!             }
//!             OrderReturnType::Signature(sign) => {
//!                 println!("[*] Transaction successful, signature: {:?}", sign);
//!             }
//!         }
//!     }
//!
//!     println!("[*] Cancel Settle Place Order");
//!     let result = market
//!         .cancel_settle_place(
//!             10.0,
//!             0.5,
//!             15.0,
//!             1.3,
//!         )
//!         .await?;
//!     println!("[*] Transaction successful, signature: {:?}", result);
//!
//!     println!("[*] Cancel Settle Place Bid Order");
//!     let result = market
//!         .cancel_settle_place_bid(0.5, 15.0)
//!         .await?;
//!     println!("[*] Transaction successful, signature: {:?}", result);
//!
//!     println!("[*] Cancel Settle Ask Order");
//!     let result = market
//!         .cancel_settle_place_ask(0.5, 15.0)
//!         .await?;
//!     println!("[*] Transaction successful, signature: {:?}", result);
//!
//!     let m = market.make_match_orders_transaction(1).await?;
//!     println!("Match Order Result: {:?}", m);
//!
//!     let open_orders_accounts = vec![Pubkey::new_from_array([0; 32])];
//!     let limit = 10;
//!
//!     let e = market.make_consume_events_instruction(open_orders_accounts.clone(), limit).await?;
//!     println!("Consume Events Result: {:?}", e);
//!
//!     let p = market.make_consume_events_permissioned_instruction(open_orders_accounts.clone(), limit).await?;
//!     println!("Consume Events Permissioned Result: {:?}", p);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Options
//!
//! | Option                                 | Default Value | Description                                              |
//! |----------------------------------------|---------------|----------------------------------------------------------|
//! | `place -t <TARGET_AMOUNT_QUOTE> -s <SIDE> -b <BEST_OFFSET_USDC> -e -p <PRICE_TARGET>` | - | Place a limit order with the specified parameters.       |
//! | `cancel -e`                            | -             | Cancel all existing order for the current owner.        |
//! | `settle -e`                            | -             | Settle balances in the OpenBook market.                  |
//! | `cancel-settle-place -u <USDC_ASK_TARGET> -b <TARGET_USDC_BID> -p <PRICE_JLP_USDC_BID> -a <ASK_PRICE_JLP_USDC>` | - | Cancel all limit orders, settle balances, and place new bid and ask orders. |
//! | `cancel-settle-place-bid -b <TARGET_SIZE_USDC_BID> -p <BID_PRICE_JLP_USDC>` | - | Cancel all limit orders, settle balances, and place a bid order. |
//! | `cancel-settle-place-ask -a <TARGET_SIZE_USDC_ASK> -p <ASK_PRICE_JLP_USDC>` | - | Cancel all limit orders, settle balances, and place an ask order. |
//! | `match --limit <LIMIT>`                | -             | Match orders transaction with the specified limit.      |
//! | `consume --limit <LIMIT>`              | -             | Consume events instruction with the specified limit.     |
//! | `consume-permissioned --limit <LIMIT>` | -             | Consume events permissioned instruction with the specified limit. |
//! | `load`                                 | -             | Load orders for the current owner, bids + asks.                      |
//! | `info`                                 | -             | Fetch OpenBook market info.                              |
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
pub mod rpc;
pub mod tokens_and_markets;
pub mod utils;

// Re-export common func
pub use openbook_dex::matching;
pub use openbook_dex::state;
pub use solana_client::rpc_config;
pub use solana_client::rpc_filter;
pub use solana_program::pubkey;
pub use solana_rpc_client::nonblocking::rpc_client;
pub use solana_sdk::account;
pub use solana_sdk::bs58;
pub use solana_sdk::commitment_config;
pub use solana_sdk::signature;
pub use solana_sdk::signer::keypair;
