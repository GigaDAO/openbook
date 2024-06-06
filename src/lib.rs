#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc = include_str!("../README.md")]

#[cfg(feature = "cli")]
pub mod cli;
pub mod rpc;
pub mod traits;
#[cfg(feature = "cli")]
pub mod tui;
pub mod utils;
#[cfg(feature = "v1")]
pub mod v1;
#[cfg(feature = "v2")]
pub mod v2;

// Re-export common func
#[cfg(feature = "v1")]
pub use openbook_dex::matching;
#[cfg(feature = "v1")]
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
