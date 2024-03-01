//! This module contains structs and functions related to open orders on the Solana blockchain.

#![allow(dead_code, deprecated)]
use crate::tokens_and_markets::get_layout_version;
use crate::utils::get_filtered_program_accounts;
use borsh::{BorshDeserialize, BorshSerialize};
use memoffset::offset_of;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_filter::Memcmp;
use solana_client::rpc_filter::MemcmpEncodedBytes::Base58;
use solana_client::rpc_filter::RpcFilterType;
use solana_program::instruction::Instruction;
use solana_program::system_instruction::create_account_with_seed;
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;
use std::convert::TryInto;
use std::error::Error;

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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use solana_sdk::pubkey::Pubkey;
    /// use openbook::orders::OpenOrders;
    ///
    /// let address = Pubkey::default();
    /// let decoded = Default::default(); // Replace with actual decoded layout
    /// let program_id = Pubkey::default();
    /// let open_orders = OpenOrders::new(address, decoded, program_id);
    /// ```
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
            1 => std::mem::size_of::<OpenOrdersLayoutV1>(),
            _ => std::mem::size_of::<OpenOrdersLayoutV2>(),
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use solana_sdk::pubkey::Pubkey;
    /// use openbook::orders::OpenOrders;
    ///
    /// let owner_address = Pubkey::default();
    /// let market_address = Pubkey::default();
    /// let program_id = Pubkey::default();
    /// let result = OpenOrders::get_derived_oo_account_pubkey(owner_address, market_address, program_id);
    /// ```
    pub fn get_derived_oo_account_pubkey(
        owner_address: Pubkey,
        market_address: Pubkey,
        program_id: Pubkey,
    ) -> Result<(Pubkey, String), Box<dyn Error>> {
        let seed = market_address
            .to_string()
            .chars()
            .take(32)
            .collect::<String>();
        let public_key = Pubkey::create_with_seed(&owner_address, &seed, &program_id)?;
        Ok((public_key, seed))
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use solana_rpc_client::rpc_client::RpcClient;
    /// use solana_sdk::pubkey::Pubkey;
    /// use openbook::orders::OpenOrders;
    ///
    /// let rpc_url = String::default(); // Replace with actual RPC URL
    /// let connection = RpcClient::new(rpc_url);
    /// let owner_address = Pubkey::default();
    /// let program_id = Pubkey::default();
    ///
    /// match OpenOrders::find_for_owner(&connection, owner_address, program_id) {
    ///     Ok(open_orders) => println!("Found open orders: {:?}", open_orders),
    ///     Err(err) => eprintln!("Error finding open orders: {:?}", err),
    /// }
    /// ```
    pub fn find_for_owner(
        connection: &RpcClient,
        owner_address: Pubkey,
        program_id: Pubkey,
    ) -> Result<Vec<Self>, Box<dyn Error>> {
        let offset = offset_of!(OpenOrdersLayoutV1, owner);
        let filters = vec![
            RpcFilterType::Memcmp(Memcmp {
                offset,
                bytes: Base58(owner_address.to_string()),
                encoding: None,
            }),
            RpcFilterType::DataSize(OpenOrders::get_layout(program_id) as u64),
        ];

        let accounts = get_filtered_program_accounts(connection, &program_id, filters)?;

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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use solana_rpc_client::rpc_client::RpcClient;
    /// use solana_sdk::pubkey::Pubkey;
    /// use openbook::orders::OpenOrders;
    ///
    /// let rpc_url = String::default(); // Replace with actual RPC URL
    /// let connection = RpcClient::new(rpc_url);
    /// let market_address = Pubkey::default();
    /// let owner_address = Pubkey::default();
    /// let program_id = Pubkey::default();
    /// let force_seed_account = false; // Set to true to force seed account creation
    ///
    /// match OpenOrders::find_for_market_and_owner(
    ///     &connection,
    ///     market_address,
    ///     owner_address,
    ///     program_id,
    ///     force_seed_account,
    /// ) {
    ///     Ok(open_orders_accounts) => println!("Open Orders Accounts: {:?}", open_orders_accounts),
    ///     Err(err) => eprintln!("Error finding open orders accounts: {:?}", err),
    /// }
    /// ```
    pub fn find_for_market_and_owner(
        connection: &RpcClient,
        market_address: Pubkey,
        owner_address: Pubkey,
        program_id: Pubkey,
        force_seed_account: bool,
    ) -> Result<Vec<Self>, Box<dyn Error>> {
        let account =
            OpenOrders::get_derived_oo_account_pubkey(owner_address, market_address, program_id)?;

        if let Ok(account_info) = connection.get_account(&account.0) {
            return Ok(vec![OpenOrders::from_account_info(
                account_info,
                program_id,
            )?]);
        }

        if force_seed_account {
            return Ok(vec![]);
        }

        let market_offset = offset_of!(OpenOrdersLayoutV1, market);
        let owner_offset = offset_of!(OpenOrdersLayoutV1, owner);

        let filters = vec![
            RpcFilterType::Memcmp(Memcmp {
                offset: market_offset,
                bytes: Base58(market_address.to_string()),
                encoding: None,
            }),
            RpcFilterType::Memcmp(Memcmp {
                offset: owner_offset,
                bytes: Base58(owner_address.to_string()),
                encoding: None,
            }),
            RpcFilterType::DataSize(OpenOrders::get_layout(program_id) as u64),
        ];
        let accounts = get_filtered_program_accounts(connection, &program_id, filters)?;

        let open_orders_result: Result<Vec<_>, _> = accounts
            .into_iter()
            .map(|account| OpenOrders::from_account_info(account.clone(), program_id))
            .collect();
        println!("{:?}", open_orders_result);

        open_orders_result
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use solana_rpc_client::rpc_client::RpcClient;
    /// use solana_sdk::pubkey::Pubkey;
    /// use openbook::orders::OpenOrders;
    ///
    /// let rpc_url = String::default(); // Replace with actual RPC URL
    /// let connection = RpcClient::new(rpc_url);
    /// let address = Pubkey::default();
    /// let program_id = Pubkey::default();
    ///
    /// match OpenOrders::load(&connection, address, program_id) {
    ///     Ok(open_orders) => println!("Loaded open orders: {:?}", open_orders),
    ///     Err(err) => eprintln!("Error loading open orders: {:?}", err),
    /// }
    /// ```
    pub fn load(
        connection: &RpcClient,
        address: Pubkey,
        program_id: Pubkey,
    ) -> Result<Self, Box<dyn Error>> {
        let account = connection.get_account(&address)?;
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use solana_sdk::account::Account;
    /// use solana_sdk::pubkey::Pubkey;
    /// use openbook::orders::OpenOrders;
    ///
    /// let account_info = Account::default(); // Replace with actual account info
    /// let program_id = Pubkey::default();
    ///
    /// match OpenOrders::from_account_info(account_info, program_id) {
    ///     Ok(open_orders) => println!("Created from account info: {:?}", open_orders),
    ///     Err(err) => eprintln!("Error creating from account info: {:?}", err),
    /// }
    /// ```
    pub fn from_account_info(
        mut account_info: Account,
        program_id: Pubkey,
    ) -> Result<Self, Box<dyn Error>> {
        // Fix: Not all bytes read
        let data_size: usize = OpenOrders::get_layout(program_id);
        account_info.data.resize(data_size, 0);
        let decoded = OpenOrdersLayoutV1::try_from_slice(&account_info.data)?;
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
    /// * `_market_address` - The public key of the market associated with the open orders.
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use solana_rpc_client::rpc_client::RpcClient;
    /// use solana_sdk::pubkey::Pubkey;
    /// use solana_sdk::signature::Keypair;
    /// use solana_program::instruction::Instruction;
    /// use openbook::orders::OpenOrders;
    ///
    /// let rpc_url = String::default(); // Replace with actual RPC URL
    /// let connection = RpcClient::new(rpc_url);
    /// let market_address = Pubkey::default();
    /// let owner_address = Pubkey::default();
    /// let new_account_address = Pubkey::default();
    /// let program_id = Pubkey::default();
    /// let seed = String::default(); // Replace with actual seed
    ///
    /// match OpenOrders::make_create_account_transaction(
    ///     &connection,
    ///     market_address,
    ///     owner_address,
    ///     new_account_address,
    ///     program_id,
    ///     seed,
    /// ) {
    ///     Ok(instruction) => println!("Transaction instruction: {:?}", instruction),
    ///     Err(err) => eprintln!("Error creating transaction instruction: {:?}", err),
    /// }
    /// ```
    pub async fn make_create_account_transaction(
        connection: &RpcClient,
        _market_address: Pubkey,
        owner_address: Pubkey,
        new_account_address: Pubkey,
        program_id: Pubkey,
        seed: String,
    ) -> Result<Instruction, Box<dyn Error>> {
        let minimum_balance = connection
            .get_minimum_balance_for_rent_exemption(OpenOrders::get_layout(program_id))?;
        let space = OpenOrders::get_layout(program_id).try_into().unwrap();

        let transaction = create_account_with_seed(
            &owner_address,
            &owner_address,
            &new_account_address,
            &seed,
            minimum_balance,
            space,
            &program_id,
        );

        Ok(transaction)
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

#[derive(Debug)]
pub struct OpenOrderState {
    pub min_ask: u64,
    pub max_bid: u64,
    pub open_asks: Vec<u128>,
    pub open_bids: Vec<u128>,
    pub bids_address: Pubkey,
    pub asks_address: Pubkey,
}
