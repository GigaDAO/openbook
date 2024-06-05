//! This module contains structs and functions related to open orders on the Solana blockchain.

use crate::rpc::Rpc;
use crate::traits::OpenOrdersT;
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    signer::keypair::Keypair,
    transaction::Transaction,
};
use std::fmt::{Debug, Formatter};
use tracing::{debug, error};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct OpenOrders {
    /// The public key of the open orders account.
    pub oo_key: Pubkey,

    /// The minimum ask price in the open orders account.
    pub min_ask: u64,

    /// The maximum bid price in the open orders account.
    pub max_bid: u64,

    /// Vector containing the prices of open asks in the open orders account.
    pub open_asks: Vec<u128>,

    /// Vector containing the prices of open bids in the open orders account.
    pub open_bids: Vec<u128>,

    /// The public key of the bids associated with the open orders account.
    pub bids_address: Pubkey,

    /// The public key of the asks associated with the open orders account.
    pub asks_address: Pubkey,

    /// Vector containing the prices of open asks in the open orders account.
    pub open_asks_prices: Vec<f64>,

    /// Vector containing the prices of open bids in the open orders account.
    pub open_bids_prices: Vec<f64>,

    /// The total amount of base currency (coin) in the open orders account.
    pub base_total: f64,

    /// The total amount of quote currency (pc) in the open orders account.
    pub quote_total: f64,
}

impl Debug for OpenOrders {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(f, "OpenOrders {{")?;
        writeln!(f, "        oo_key: {:?}", self.oo_key)?;
        writeln!(f, "        min_ask: {:?}", self.min_ask)?;
        writeln!(f, "        max_bid: {:?}", self.max_bid)?;
        writeln!(f, "        open_asks: {:?}", self.open_asks)?;
        writeln!(f, "        open_bids: {:?}", self.open_bids)?;
        writeln!(f, "        bids_address: {:?}", self.bids_address)?;
        writeln!(f, "        asks_address: {:?}", self.asks_address)?;
        writeln!(f, "        open_asks_prices: {:?}", self.open_asks_prices)?;
        writeln!(f, "        open_bids_prices: {:?}", self.open_bids_prices)?;
        writeln!(f, "        base_total: {:?}", self.base_total)?;
        writeln!(f, "        quote_total: {:?}", self.quote_total)?;
        writeln!(f, "    }}")
    }
}

impl OpenOrdersT for OpenOrders {
    /// Creates a new `OpenOrders` instance from the given data.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - RPC client for interacting with the Solana blockchain.
    /// * `program_id` - The program ID representing the market.
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
        market_address: Pubkey,
    ) -> Result<Self, Error> {
        let mut oo_account = Self::default();

        let _ = oo_account
            .make_create_account_transaction(&rpc_client, program_id, &keypair, market_address)
            .await?;

        Ok(oo_account)
    }

    /// Generates a new open orders account associated with a wallet.
    ///
    /// This method creates a new open orders account on the Solana blockchain and initializes it with the provided parameters.
    /// Open orders accounts are used to manage orders within a specific market.
    ///
    /// # Arguments
    ///
    /// * `connection` - The RPC client for interacting with the Solana blockchain.
    /// * `program_id` - The program ID associated with the open orders.
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
    ) -> Result<Pubkey, Error> {
        let new_account_address = Keypair::new();
        let space = 0;
        let minimum_balance = connection
            .inner()
            .get_minimum_balance_for_rent_exemption(space)
            .await?;

        let instruction = solana_sdk::system_instruction::create_account(
            &keypair.pubkey(),
            &new_account_address.pubkey(),
            minimum_balance,
            space as u64,
            &keypair.pubkey(),
        );
        let init_ix = openbook_dex::instruction::init_open_orders(
            &program_id,
            &new_account_address.pubkey(),
            &keypair.pubkey(),
            &market_account,
            None,
        )?;
        debug!(
            "[*] Got New Account Address: {:?}",
            new_account_address.pubkey()
        );

        let mut instructions = Vec::new();
        let r = connection
            .inner()
            .get_recent_prioritization_fees(&[])
            .await
            .unwrap();
        let mut max_fee = 1;
        for f in r {
            if f.prioritization_fee > max_fee {
                max_fee = f.prioritization_fee;
            }
        }

        let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);
        let fee_ix = ComputeBudgetInstruction::set_compute_unit_price(max_fee);
        instructions.push(budget_ix);
        instructions.push(fee_ix);

        instructions.push(instruction);
        instructions.push(init_ix);

        debug!("[*] Using Pubkey: {}", &keypair.pubkey().to_string());

        let recent_hash = connection.inner().get_latest_blockhash().await?;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&keypair.pubkey()),
            &[&new_account_address, &keypair],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;

        let result = connection
            .inner()
            .send_transaction_with_config(&txn, config)
            .await;

        match result {
            Ok(sig) => debug!("[*] Transaction successful, signature: {:?}", sig),
            Err(err) => error!("[*] Transaction failed: {:?}", err),
        };

        self.oo_key = new_account_address.pubkey();

        Ok(new_account_address.pubkey())
    }
}

#[derive(Debug, Clone)]
pub struct OpenOrdersCacheEntry {
    pub open_orders: OpenOrders,
    pub ts: u128,
}

#[derive(Debug)]
pub enum OrderReturnType {
    Instructions(Vec<Instruction>),
    Signature(Signature),
}
