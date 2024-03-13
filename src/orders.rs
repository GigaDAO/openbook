//! This module contains structs and functions related to open orders on the Solana blockchain.

#![allow(dead_code, deprecated)]
use crate::{
    rpc_client::RpcClient, tokens_and_markets::get_layout_version,
    utils::get_filtered_program_accounts,
};
use borsh::{BorshDeserialize, BorshSerialize};
use log::debug;
use memoffset::offset_of;
use solana_client::{
    rpc_filter::MemcmpEncodedBytes,
    rpc_filter::{Memcmp, MemcmpEncodedBytes::Base58, MemcmpEncoding, RpcFilterType},
};
use solana_sdk::{
    account::Account, bs58, compute_budget::ComputeBudgetInstruction,
    nonce::state::Data as NonceData, pubkey::Pubkey, signature::Signer, signer::keypair::Keypair,
    transaction::Transaction,
};
use std::{borrow::Borrow, convert::TryInto, error::Error};

/// Struct representing an open orders account.
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct OpenOrders {
    /// The public key of the open orders account.
    pub address: Pubkey,

    /// The public key of the market associated with the open orders.
    pub market: Pubkey,

    /// The public key of the owner of the open orders.
    pub owner: Pubkey,

    /// The amount of base token that is free and available in the open orders account.
    pub base_token_free: u64,

    /// The total amount of base token in the open orders account.
    pub base_token_total: u64,

    /// The amount of quote token that is free and available in the open orders account.
    pub quote_token_free: u64,

    /// The total amount of quote token in the open orders account.
    pub quote_token_total: u64,

    /// Bit field representing free slots in the open orders account.
    pub free_slot_bits: u64,

    /// Bit field representing whether each slot contains a bid order.
    pub is_bid_bits: u64,

    /// List of order IDs in the open orders account.
    pub orders: Vec<u64>,

    /// List of client IDs associated with the orders in the open orders account.
    pub client_ids: Vec<u64>,
}

impl OpenOrders {
    /// Creates a new `OpenOrders` instance from the given data.
    ///
    /// # Arguments
    ///
    /// * `address` - The public key of the open orders account.
    /// * `decoded` - The decoded layout data of the open orders account.
    /// * `_program_id` - The program ID associated with the open orders.
    ///
    /// # Returns
    ///
    /// An instance of `OpenOrders`.
    pub fn new(address: Pubkey, decoded: OpenOrdersLayoutV1, _program_id: Pubkey) -> Self {
        let OpenOrdersLayoutV1 {
            market,
            owner,
            base_token_free,
            base_token_total,
            quote_token_free,
            quote_token_total,
            free_slot_bits,
            is_bid_bits,
            orders,
            client_ids,
            account_flags: _,
            padding: _,
        } = decoded;

        Self {
            address,
            market: market.into(),
            owner: owner.into(),
            base_token_free,
            base_token_total,
            quote_token_free,
            quote_token_total,
            free_slot_bits: free_slot_bits.try_into().unwrap(),
            is_bid_bits: is_bid_bits.try_into().unwrap(),
            orders: orders.into(),
            client_ids: client_ids.into(),
        }
    }

    /// Returns the layout size of the `OpenOrders` struct based on the program ID.
    pub fn get_layout(program_id: Pubkey) -> usize {
        match get_layout_version(&program_id) {
            1 => std::mem::size_of::<openbook_dex::state::OpenOrders>(),
            _ => std::mem::size_of::<openbook_dex::state::OpenOrders>(),
        }
    }

    /// Returns the derived open orders account pubkey and seed for the given owner and market.
    ///
    /// # Arguments
    ///
    /// * `owner_address` - The public key of the owner.
    /// * `market_address` - The public key of the market.
    /// * `program_id` - The program ID associated with the open orders.
    ///
    /// # Returns
    ///
    /// A tuple containing the derived pubkey and seed.
    ///
    /// # Errors
    ///
    /// Returns a `Box<dyn Error>` if there is an error during pubkey creation.
    pub fn get_derived_oo_account_pubkey(
        owner_address: Pubkey,
        market_address: Pubkey,
        program_id: Pubkey,
    ) -> Result<Pubkey, Box<dyn Error>> {
        let seed = market_address
            .to_string()
            .chars()
            .take(32)
            .collect::<String>();
        let public_key = Pubkey::create_with_seed(&owner_address, &seed, &program_id)?;
        Ok(public_key)
    }

    /// Finds open orders accounts associated with the given owner.
    ///
    /// # Arguments
    ///
    /// * `connection` - The RPC client for interacting with the Solana blockchain.
    /// * `owner_address` - The public key of the owner.
    /// * `program_id` - The program ID associated with the open orders.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `OpenOrders` or an error.
    ///
    /// # Errors
    ///
    /// Returns a `Box<dyn Error>` if there is an error during the RPC call or deserialization.
    pub async fn find_for_owner(
        connection: &RpcClient,
        owner_address: Pubkey,
        program_id: Pubkey,
    ) -> Result<Vec<Self>, Box<dyn Error>> {
        let offset = offset_of!(OpenOrdersLayoutV1, owner);
        let filters = vec![
            RpcFilterType::Memcmp(Memcmp {
                offset,
                bytes: Base58(owner_address.to_string()),
                encoding: Some(MemcmpEncoding::Binary),
            }),
            RpcFilterType::DataSize(OpenOrders::get_layout(program_id) as u64),
        ];

        let accounts = get_filtered_program_accounts(connection, filters).await?;

        let open_orders_result: Result<Vec<_>, _> = accounts
            .into_iter()
            .map(|account| OpenOrders::from_account_info(account.clone(), program_id))
            .collect();

        open_orders_result
    }

    /// Finds open orders accounts for a given market and owner.
    ///
    /// This function queries the blockchain for open orders accounts associated with a specific
    /// market and owner. Optionally, it can force the creation of a seed account if none is found.
    ///
    /// # Arguments
    ///
    /// * `connection` - The RPC client for interacting with the Solana blockchain.
    /// * `market_address` - The public key representing the market associated with the open orders.
    /// * `owner_address` - The public key representing the owner of the open orders.
    /// * `program_id` - The program ID associated with the open orders.
    /// * `force_seed_account` - A boolean indicating whether to force the creation of a seed account
    ///                          if no open orders account is found.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `OpenOrders` instances or an error.
    ///
    /// # Errors
    ///
    /// Returns a `Box<dyn Error>` if there is an error during the RPC call, deserialization, or
    /// ownership check.
    pub async fn find_for_market_and_owner(
        connection: &RpcClient,
        market_address: Pubkey,
        owner_address: Pubkey,
        force_seed_account: bool,
    ) -> Result<Vec<Account>, Box<dyn Error>> {
        let _account_info = connection.get_account(&owner_address).await?;

        if force_seed_account {
            return Ok(vec![]);
        }

        let _market_offset = offset_of!(OpenOrdersLayoutV1, market);
        let _owner_offset = offset_of!(OpenOrdersLayoutV1, owner);

        let filters = vec![
            RpcFilterType::Memcmp(Memcmp {
                offset: 32,
                bytes: MemcmpEncodedBytes::Base58(bs58::encode(owner_address).into_string()),
                encoding: Some(MemcmpEncoding::Binary),
            }),
            RpcFilterType::Memcmp(Memcmp {
                offset: 32,
                bytes: MemcmpEncodedBytes::Base58(bs58::encode(market_address).into_string()),
                encoding: Some(MemcmpEncoding::Binary),
            }),
            RpcFilterType::DataSize(165),
        ];
        let accounts = get_filtered_program_accounts(connection, filters).await?;

        let _open_orders_result: Result<Vec<_>, _> = accounts
            .clone()
            .into_iter()
            .map(|account| account.deserialize_data::<NonceData>())
            .collect();

        Ok(accounts)
    }

    /// Loads an open orders account from the blockchain.
    ///
    /// # Arguments
    ///
    /// * `connection` - The RPC client for interacting with the Solana blockchain.
    /// * `address` - The public key of the open orders account.
    /// * `program_id` - The program ID associated with the open orders.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `OpenOrders` instance or an error.
    ///
    /// # Errors
    ///
    /// Returns a `Box<dyn Error>` if there is an error during the RPC call or deserialization.
    pub async fn load(
        connection: &RpcClient,
        address: Pubkey,
        program_id: Pubkey,
    ) -> Result<Self, Box<dyn Error>> {
        let account = connection.get_account(&address).await?;
        OpenOrders::from_account_info(account, program_id)
    }

    /// Creates an `OpenOrders` instance from the given account information.
    ///
    /// # Arguments
    ///
    /// * `account_info` - The account information received from the blockchain.
    /// * `program_id` - The program ID associated with the open orders.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `OpenOrders` instance or an error.
    ///
    /// # Errors
    ///
    /// Returns a `Box<dyn Error>` if there is an error during deserialization or ownership check.
    pub fn from_account_info(
        mut account_info: Account,
        program_id: Pubkey,
    ) -> Result<Self, Box<dyn Error>> {
        // Fix: Not all bytes read
        let data_size: usize = 165;
        account_info.data.resize(data_size, 0);
        let decoded = OpenOrdersLayoutV1::try_from_slice(account_info.data.borrow())?;
        let _account_flags = decoded.account_flags;
        if !account_info.owner.eq(&program_id) {
            return Err("Address not owned by program".into());
        }

        // if !account_flags.initialized || !account_flags.open_orders {
        //     return Err("Invalid open orders account".into());
        // }

        let OpenOrdersLayoutV1 {
            account_flags: _,
            market,
            owner,
            base_token_free,
            base_token_total,
            quote_token_free,
            quote_token_total,
            free_slot_bits,
            is_bid_bits,
            orders,
            client_ids,
            padding: _,
        } = decoded;

        Ok(Self {
            address: account_info.owner,
            market: market.into(),
            owner: owner.into(),
            base_token_free,
            base_token_total,
            quote_token_free,
            quote_token_total,
            free_slot_bits: free_slot_bits.try_into().unwrap(),
            is_bid_bits: is_bid_bits.try_into().unwrap(),
            orders: orders.into(),
            client_ids: client_ids.into(),
        })
    }

    /// Generates a transaction instruction for creating a new open orders account.
    ///
    /// # Arguments
    ///
    /// * `connection` - The RPC client for interacting with the Solana blockchain.
    /// * `owner_address` - The public key of the owner.
    /// * `new_account_address` - The public key for the new open orders account.
    /// * `program_id` - The program ID associated with the open orders.
    /// * `seed` - The seed for deriving the new account address.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `Instruction` or an error.
    ///
    /// # Errors
    ///
    /// Returns a `Box<dyn Error>` if there is an error during the RPC call or transaction creation.
    pub async fn make_create_account_transaction(
        connection: &RpcClient,
        program_id: Pubkey,
        keypair: &Keypair,
        market_account: Pubkey,
    ) -> Result<Pubkey, Box<dyn Error>> {
        let new_account_address = Keypair::new();
        let minimum_balance = connection
            .get_minimum_balance_for_rent_exemption(OpenOrders::get_layout(program_id))
            .await?;
        let space: u64 = OpenOrders::get_layout(program_id).try_into().unwrap();

        // let seed = market_account
        //     .to_string()
        //     .chars()
        //     .take(32)
        //     .collect::<String>();

        // let instruction = create_account_with_seed(
        //     &keypair.pubkey(),
        //     &new_account_address.pubkey(),
        //     &new_account_address.pubkey(),
        //     &seed,
        //     minimum_balance,
        //     space + 12,
        //     &program_id,
        // );
        let instruction = solana_sdk::system_instruction::create_account(
            &keypair.pubkey(),
            &new_account_address.pubkey(),
            minimum_balance,
            space + 12,
            &program_id,
        );
        let init_ix = openbook_dex::instruction::init_open_orders(
            &program_id,
            &new_account_address.pubkey(),
            &keypair.pubkey(),
            &market_account,
            None,
        )?;
        debug!(
            "Got New Account Address: {:?}",
            new_account_address.pubkey()
        );

        let mut instructions = Vec::new();
        let r = connection
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

        debug!("Using Pubkey: {}", &keypair.pubkey().to_string());

        let recent_hash = connection.get_latest_blockhash().await?;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&keypair.pubkey()),
            &[&new_account_address, &keypair],
            recent_hash,
        );

        let result = connection.send_transaction(&txn).await;

        match result {
            Ok(sig) => debug!("Transaction successful, signature: {:?}", sig),
            Err(err) => debug!("Transaction failed: {:?}", err),
        };

        Ok(new_account_address.pubkey())
    }

    pub fn get_public_key(&self) -> Pubkey {
        self.address
    }
}

/// Enumeration representing account flags.
#[derive(Debug)]
struct AccountFlags {
    /// Flag indicating whether the account is initialized.
    initialized: bool,

    /// Flag indicating whether the account is open orders.
    open_orders: bool,
}

/// Layout of the open orders account for version 1.
#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub struct OpenOrdersLayoutV1 {
    /// Account flags indicating initialization and open orders status.
    pub account_flags: u64,

    /// Public key of the market associated with the open orders.
    pub market: [u8; 32],

    /// Public key of the owner of the open orders.
    pub owner: [u8; 32],

    /// Amount of base token that is free and available in the open orders account.
    pub base_token_free: u64,

    /// Total amount of base token in the open orders account.
    pub base_token_total: u64,

    /// Amount of quote token that is free and available in the open orders account.
    pub quote_token_free: u64,

    /// Total amount of quote token in the open orders account.
    pub quote_token_total: u64,

    /// Bit field representing free slots in the open orders account.
    pub free_slot_bits: u128,

    /// Bit field representing whether each slot contains a bid order.
    pub is_bid_bits: u128,

    /// List of order IDs in the open orders account.
    pub orders: [u64; 128],

    /// List of client IDs associated with the orders in the open orders account.
    pub client_ids: [u64; 128],

    /// Padding for alignment.
    pub padding: [u8; 7],
}

impl Default for OpenOrdersLayoutV1 {
    fn default() -> Self {
        Self {
            account_flags: 0,
            market: [0; 32],
            owner: [0; 32],
            base_token_free: 0,
            base_token_total: 0,
            quote_token_free: 0,
            quote_token_total: 0,
            free_slot_bits: 0,
            is_bid_bits: 0,
            orders: [0; 128],
            client_ids: [0; 128],
            padding: [0; 7],
        }
    }
}

/// Layout of the open orders account for version 2.
#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub struct OpenOrdersLayoutV2 {
    /// Account flags indicating initialization and open orders status.
    pub account_flags: u64,

    /// Public key of the market associated with the open orders.
    pub market: [u8; 32],

    /// Public key of the owner of the open orders.
    pub owner: [u8; 32],

    /// Amount of base token that is free and available in the open orders account.
    pub base_token_free: u64,

    /// Total amount of base token in the open orders account.
    pub base_token_total: u64,

    /// Amount of quote token that is free and available in the open orders account.
    pub quote_token_free: u64,

    /// Total amount of quote token in the open orders account.
    pub quote_token_total: u64,

    /// Bit field representing free slots in the open orders account.
    pub free_slot_bits: u128,

    /// Bit field representing whether each slot contains a bid order.
    pub is_bid_bits: u128,

    /// List of order IDs in the open orders account.
    pub orders: [u64; 128],

    /// List of client IDs associated with the orders in the open orders account.
    pub client_ids: [u64; 128],

    /// Additional field for version 2.
    pub referrer_rebates_accrued: u64,

    /// Padding for alignment.
    pub padding: [u8; 7],
}
impl Default for OpenOrdersLayoutV2 {
    fn default() -> Self {
        Self {
            account_flags: 0,
            market: [0; 32],
            owner: [0; 32],
            base_token_free: 0,
            base_token_total: 0,
            quote_token_free: 0,
            quote_token_total: 0,
            free_slot_bits: 0,
            is_bid_bits: 0,
            orders: [0; 128],
            client_ids: [0; 128],
            referrer_rebates_accrued: 0,
            padding: [0; 7],
        }
    }
}

#[derive(Debug)]
pub struct OpenOrdersCacheEntry {
    pub accounts: Vec<OpenOrders>,
    pub ts: u128,
}

#[derive(Debug, Clone, Default)]
pub struct MarketInfo {
    pub min_ask: u64,
    pub max_bid: u64,
    pub open_asks: Vec<u128>,
    pub open_bids: Vec<u128>,
    pub bids_address: Pubkey,
    pub asks_address: Pubkey,
    pub open_asks_prices: Vec<f64>,
    pub open_bids_prices: Vec<f64>,
    pub base_total: f64,
    pub quote_total: f64,
}
