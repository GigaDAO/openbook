#![allow(async_fn_in_trait)]

#[cfg(feature = "v1")]
use crate::v1::{market::Market, orders::OpenOrders};

use crate::rpc::Rpc;

use anyhow::Error;
use solana_sdk::{pubkey::Pubkey, signer::keypair::Keypair, sysvar::slot_history::AccountInfo};
use std::fmt::Debug;

/// Trait for interacting with market-related functionality.
pub trait MarketInfo: Debug {
    /// Initializes a new instance of the `Market` struct.
    async fn new(
        rpc_client: Rpc,
        program_id: Pubkey,
        market_id: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
        load: bool,
    ) -> Result<Market, Error>;

    /// Loads market information from the provided RPC client.
    async fn load(&mut self, rpc_client: &Rpc) -> Result<(), Error>;

    /// Loads the market state information from the provided account information.
    async fn load_market_state_info(&mut self, account_info: &AccountInfo<'_>)
        -> Result<(), Error>;

    /// Initializes the vault signer key.
    async fn init_vault_signer_key(&mut self) -> Result<(), Error>;
}

/// Trait for open orders functionality.
pub trait OpenOrdersT {
    /// Creates a new `OpenOrders` instance from the given data.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - RPC client for interacting with the Solana blockchain.
    /// * `program` - The program id of the owner used for signing transactions.
    /// * `keypair` - The keypair of the owner used for signing transactions.
    /// * `market_address` - The public key of the market.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an instance of `OpenOrders` or an error.
    ///
    /// # Errors
    ///
    /// Returns a `Box<dyn Error>` if there is an error during the RPC call or transaction creation.
    async fn new(
        rpc_client: Rpc,
        program_id: Pubkey,
        keypair: Keypair,
        market_id: Pubkey,
    ) -> Result<OpenOrders, Error>;

    /// Generates a new open orders account associated with a wallet.
    ///
    /// This method creates a new open orders account on the Solana blockchain and initializes it with the provided parameters.
    /// Open orders accounts are used to manage orders within a specific market.
    ///
    /// # Arguments
    ///
    /// * `connection` - The RPC client for interacting with the Solana blockchain.
    /// * `keypair` - The keypair of the wallet owner, used for signing the transaction.
    /// * `market_account` - The public key of the market associated with the open orders account.
    ///
    /// # Returns
    ///
    /// A `Result` containing the public key of the newly created open orders account or an error.
    ///
    /// # Errors
    ///
    /// Returns a `Box<dyn Error>` if there is an error during the RPC call or transaction creation.
    async fn make_create_account_transaction(
        &mut self,
        connection: &Rpc,
        program_id: Pubkey,
        keypair: &Keypair,
        market_account: Pubkey,
    ) -> Result<Pubkey, Error>;
}
