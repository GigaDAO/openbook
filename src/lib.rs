#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc = include_str!("../README.md")]

#[cfg(feature = "cli")]
pub mod cli;
pub mod fees;
pub mod market;
pub mod ob_client;
pub mod orders;
pub mod rpc;
pub mod tokens_and_markets;
pub mod traits;
#[cfg(feature = "cli")]
pub mod tui;
pub mod utils;

// Re-export common func
pub use openbook_dex::matching;
pub use openbook_dex::state;
pub use solana_client::nonblocking::rpc_client;
pub use solana_client::rpc_config;
pub use solana_client::rpc_filter;
pub use solana_sdk::account;
pub use solana_sdk::bs58;
pub use solana_sdk::commitment_config;
pub use solana_sdk::pubkey;
pub use solana_sdk::signature;
pub use solana_sdk::signer::keypair;
