//! This module contains structs and functions related to the openbook market.

use crate::orders::OpenOrders;
use crate::orders::OpenOrdersCacheEntry;
use crate::utils::{get_unix_secs, u64_slice_to_bytes};
use openbook_dex::critbit::Slab;
use openbook_dex::instruction::SelfTradeBehavior;
use openbook_dex::matching::{OrderType, Side};
use openbook_dex::state::MarketState;
use rand::random;
use solana_client::client_error::ClientError;
use solana_program::sysvar::slot_history::ProgramError;
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::account::Account;
use solana_sdk::signature::Signature;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::cell::RefMut;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU64;
use std::time::{SystemTime, UNIX_EPOCH};

/// Struct representing a market with associated state and information.
#[derive(Debug)]
pub struct Market {
    /// The RPC client for interacting with the Solana blockchain.
    pub rpc_client: DebuggableRpcClient,

    /// The public key of the program associated with the market.
    pub program_id: Pubkey,

    /// The public key of the market.
    pub market_address: Pubkey,

    /// The keypair used for signing transactions related to the market.
    pub keypair: Keypair,

    /// The number of decimal places for the base currency (coin) in the market.
    pub coin_decimals: u8,

    /// The number of decimal places for the quote currency (pc) in the market.
    pub pc_decimals: u8,

    /// The lot size for the base currency (coin) in the market.
    pub coin_lot_size: u64,

    /// The maximum bid price in the market.
    pub max_bid: f64,

    /// The account flags associated with the market.
    pub account_flags: u64,

    /// The lot size for the quote currency (pc) in the market.
    pub pc_lot_size: u64,

    /// The public key of the bids account associated with the market.
    pub bids_address: Pubkey,

    /// The public key of the asks account associated with the market.
    pub asks_address: Pubkey,

    /// The public key of the account holding USDC tokens.
    pub usdc_ata: Pubkey,

    /// The public key of the account holding WSOL tokens.
    pub wsol_ata: Pubkey,

    /// The public key of the vault holding base currency (coin) tokens.
    pub coin_vault: Pubkey,

    /// The public key of the vault holding quote currency (pc) tokens.
    pub pc_vault: Pubkey,

    /// The public key of the vault signer key associated with the market.
    pub vault_signer_key: Pubkey,

    /// The public key of the orders account associated with the market.
    pub orders_key: Pubkey,

    /// The public key of the event queue associated with the market.
    pub event_queue: Pubkey,

    /// The public key of the request queue associated with the market.
    pub request_queue: Pubkey,
    open_orders_accounts_cache: HashMap<Pubkey, OpenOrdersCacheEntry>,
}

/// Wrapper type for RpcClient to enable Debug trait implementation.
pub struct DebuggableRpcClient(solana_rpc_client::rpc_client::RpcClient);

/// Implement the Debug trait for the wrapper type `DebuggableRpcClient`.
impl fmt::Debug for DebuggableRpcClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Include relevant information about RpcClient
        f.debug_struct("RpcClient").finish()
    }
}

impl Market {
    /// Creates a new instance of the `Market` struct with initialized default values.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - The RPC client for interacting with the Solana blockchain.
    /// * `program_id` - The program ID associated with the market.
    /// * `market_address` - The public key representing the market.
    /// * `keypair` - The keypair for signing transactions.
    ///
    /// # Returns
    ///
    /// An instance of the `Market` struct.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// println!("Market Info: {:?}", market);
    /// ```
    pub fn new(
        rpc_client: RpcClient,
        program_id: Pubkey,
        market_address: Pubkey,
        keypair: Keypair,
    ) -> Self {
        let bids_address = Default::default();
        let asks_address = Default::default();
        let usdc_ata = Default::default();
        let wsol_ata = Default::default();
        let coin_vault = Default::default();
        let pc_vault = Default::default();
        let vault_signer_key = Default::default();
        let orders_key = Default::default();
        let event_queue = Default::default();
        let request_queue = Default::default();

        let decoded = Default::default();
        let open_orders = OpenOrders::new(market_address, decoded, keypair.pubkey());
        let mut open_orders_accounts_cache = HashMap::new();

        let open_orders_cache_entry = OpenOrdersCacheEntry {
            accounts: vec![open_orders],
            ts: 123456789,
        };

        open_orders_accounts_cache.insert(keypair.pubkey(), open_orders_cache_entry);

        let mut market = Self {
            rpc_client: DebuggableRpcClient(rpc_client),
            program_id,
            market_address,
            keypair,
            coin_decimals: 9,
            pc_decimals: 6,
            coin_lot_size: 1_000_000,
            pc_lot_size: 1,
            max_bid: 0.0,
            bids_address,
            asks_address,
            usdc_ata,
            wsol_ata,
            coin_vault,
            pc_vault,
            vault_signer_key,
            orders_key,
            event_queue,
            request_queue,
            account_flags: 0,
            open_orders_accounts_cache,
        };
        market.load().unwrap();

        market
    }

    /// Loads market information, including account details and state, using the provided RPC client.
    ///
    /// This function fetches and processes the necessary account information from Solana
    /// blockchain to initialize the `Market` struct. It retrieves the market state, bids
    /// information, and other relevant details.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the `Market` struct.
    ///
    /// # Returns
    ///
    /// `Result` indicating success or an error if loading the market information fails.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with fetching accounts
    /// or processing the market information.
    pub fn load(&mut self) -> anyhow::Result<()> {
        let mut account = self.rpc_client.0.get_account(&self.market_address)?;
        let program_id_binding = self.program_id;
        let market_account_binding = self.market_address;
        let account_info = self.create_account_info_from_account(
            &mut account,
            &market_account_binding,
            &program_id_binding,
            false,
            false,
        );

        let _ = self.load_market_state_bids_info(&account_info)?;

        Ok(())
    }

    /// Loads the market state and bids information from the provided account information.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the `Market` struct.
    /// * `binding` - A reference to the account information used to load the market state.
    ///
    /// # Returns
    ///
    /// A `Result` containing a mutable reference to the loaded `MarketState` if successful,
    /// or an error if loading the market state fails.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with loading the market state.
    pub fn load_market_state_bids_info<'a>(
        &'a mut self,
        binding: &'a AccountInfo,
    ) -> anyhow::Result<RefMut<MarketState>> {
        let market_state = MarketState::load(binding, &self.program_id, false)?;

        {
            let coin_vault_array: [u8; 32] = u64_slice_to_bytes(market_state.coin_vault);
            let pc_vault_array: [u8; 32] = u64_slice_to_bytes(market_state.pc_vault);
            let request_queue_array: [u8; 32] = u64_slice_to_bytes(market_state.req_q);
            let event_queue_array: [u8; 32] = u64_slice_to_bytes(market_state.event_q);

            let coin_vault_temp = Pubkey::new_from_array(coin_vault_array);
            let pc_vault_temp = Pubkey::new_from_array(pc_vault_array);
            let request_queue_temp = Pubkey::new_from_array(request_queue_array);
            let event_queue_temp = Pubkey::new_from_array(event_queue_array);

            self.coin_vault = coin_vault_temp;
            self.pc_vault = pc_vault_temp;
            self.request_queue = request_queue_temp;
            self.account_flags = market_state.account_flags;
            self.coin_lot_size = market_state.coin_lot_size;
            self.pc_lot_size = market_state.pc_lot_size;
            self.coin_lot_size = market_state.coin_lot_size;
            self.event_queue = event_queue_temp;
        }

        let (bids_address, asks_address) = self.get_bids_asks_addresses(&market_state);

        let mut bids_account = self.rpc_client.0.get_account(&bids_address)?;
        let bids_info = self.create_account_info_from_account(
            &mut bids_account,
            &bids_address,
            &self.program_id,
            false,
            false,
        );
        let mut bids = market_state.load_bids_mut(&bids_info)?;
        let max_bid = self.process_bids(&mut bids)?;

        self.bids_address = bids_address;
        self.asks_address = asks_address;
        self.max_bid = max_bid;

        Ok(market_state)
    }

    /// Loads information about bids, asks, and the maximum bid price from the market state.
    ///
    /// This function fetches and processes bids information from the provided `MarketState`,
    /// including extracting the bids and asks addresses, loading the bids account, and determining
    /// the maximum bid price.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `market_state` - A mutable reference to the `MarketState` representing the current state of the market.
    ///
    /// # Returns
    ///
    /// A `Result` containing a tuple of `(bids_address, asks_address, max_bid)` if successful,
    /// or an error if loading the bids information fails.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with fetching accounts
    /// or processing the bids information.
    pub fn load_bids_info(
        &mut self,
        market_state: &RefMut<MarketState>,
    ) -> anyhow::Result<(Pubkey, Pubkey, f64)> {
        let (bids_address, asks_address) = self.get_bids_asks_addresses(market_state);

        let mut bids_account = self.rpc_client.0.get_account(&bids_address)?;
        let bids_info = self.create_account_info_from_account(
            &mut bids_account,
            &bids_address,
            &self.program_id,
            false,
            false,
        );
        let mut bids = market_state.load_bids_mut(&bids_info)?;

        let max_bid = self.process_bids(&mut bids)?;

        self.bids_address = bids_address;
        self.asks_address = asks_address;
        self.max_bid = max_bid;

        Ok((bids_address, asks_address, max_bid))
    }

    /// Processes bids information to find the maximum bid price.
    ///
    /// This function iteratively removes bids from the provided `Slab` until
    /// it finds the maximum bid price.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `bids` - A mutable reference to the `Slab` containing bids information.
    ///
    /// # Returns
    ///
    /// A `Result` containing the maximum bid price if successful, or an error if processing bids fails.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with processing the bids information.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// // let max_bid = market.process_bids(&mut bids).expect("Failed to process bids");
    /// ```
    pub fn process_bids(&self, bids: &mut RefMut<Slab>) -> anyhow::Result<f64> {
        let max_bid;
        loop {
            let node = bids.remove_max();
            match node {
                Some(node) => {
                    let price_raw = node.price().get();
                    // let price = price_raw as f64 / 1e3;
                    max_bid = price_raw as f64;
                    break;
                }
                None => {
                    panic!("failed to load bids");
                }
            }
        }
        Ok(max_bid)
    }

    /// Retrieves the bids and asks addresses from the given market state.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `market_state` - A reference to the `MarketState` representing the current state of the market.
    ///
    /// # Returns
    ///
    /// A tuple containing the bids and asks addresses.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// let (bids_address, asks_address) = market.get_bids_asks_addresses(&market_state);
    /// ```
    pub fn get_bids_asks_addresses(&self, market_state: &MarketState) -> (Pubkey, Pubkey) {
        let bids = market_state.bids;
        let asks = market_state.asks;
        let bids_bytes = u64_slice_to_bytes(bids);
        let asks_bytes = u64_slice_to_bytes(asks);

        let bids_address = Pubkey::new_from_array(bids_bytes);
        let asks_address = Pubkey::new_from_array(asks_bytes);

        (bids_address, asks_address)
    }

    /// Creates an `AccountInfo` instance from an `Account`.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `account` - A mutable reference to the account from which to create `AccountInfo`.
    /// * `key` - A reference to the public key associated with the account.
    /// * `my_program_id` - A reference to the program's public key.
    /// * `is_signer` - A boolean indicating whether the account is a signer.
    /// * `is_writable` - A boolean indicating whether the account is writable.
    ///
    /// # Returns
    ///
    /// An `AccountInfo` instance created from the provided parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// let account_info = market.create_account_info_from_account(&mut account, &key, &my_program_id, true, true);
    /// ```
    pub fn create_account_info_from_account<'a>(
        &self,
        account: &'a mut Account,
        key: &'a Pubkey,
        my_program_id: &'a Pubkey,
        is_signer: bool,
        is_writable: bool,
    ) -> AccountInfo<'a> {
        AccountInfo::new(
            key,
            is_signer,
            is_writable,
            &mut account.lamports,
            &mut account.data,
            my_program_id,
            account.executable,
            account.rent_epoch,
        )
    }

    /// Places a limit bid order on the market.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `max_bid` - The maximum bid value for the order.
    ///
    /// # Returns
    ///
    /// A `Result` containing the transaction signature if successful,
    /// or an error if placing the limit bid fails.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with creating or sending the transaction.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// let max_bid = 1;
    /// let r = market.place_limit_bid(max_bid).expect("Failed to place limit bid");
    /// ```
    pub fn place_limit_bid(&self, max_bid: u64) -> anyhow::Result<Signature, ClientError> {
        assert!(max_bid > 0, "Max bid must be greater than zero");
        let limit_price = NonZeroU64::new(max_bid).unwrap();
        let max_coin_qty = NonZeroU64::new(self.coin_lot_size).unwrap();
        let target_usdc_lots_w_fee = (1.0 * 1e6 * 1.1) as u64;

        let place_order_ix = openbook_dex::instruction::new_order(
            &self.market_address,
            &self.orders_key,
            &self.request_queue,
            &self.event_queue,
            &self.bids_address,
            &self.asks_address,
            &self.usdc_ata,
            &self.keypair.pubkey(),
            &self.coin_vault,
            &self.pc_vault,
            &anchor_spl::token::ID,
            &solana_program::sysvar::rent::ID,
            None,
            &self.program_id,
            Side::Bid,
            limit_price,
            max_coin_qty,
            OrderType::PostOnly,
            random::<u64>(),
            SelfTradeBehavior::AbortTransaction,
            u16::MAX,
            NonZeroU64::new(target_usdc_lots_w_fee).unwrap(),
            (get_unix_secs() + 30) as i64,
        )
        .unwrap();

        let instructions = vec![place_order_ix];

        let recent_hash = self.rpc_client.0.get_latest_blockhash()?;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        self.rpc_client.0.send_transaction_with_config(&txn, config)
    }

    /// Cancels an existing order in the market.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `order_id` - The identifier of the order to be canceled.
    ///
    /// # Returns
    ///
    /// A `Result` containing the transaction signature if successful,
    /// or an error if canceling the order fails.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with creating or sending the transaction.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// let order_id_to_cancel = 2;
    /// let c = market.cancel_order(order_id_to_cancel).expect("Failed to cancel order");
    /// ```
    pub fn cancel_order(&self, order_id: u64) -> anyhow::Result<Signature, ClientError> {
        assert!(order_id > 0, "Order ID must be greater than zero");

        let ix = openbook_dex::instruction::cancel_order(
            &self.program_id,
            &self.market_address,
            &self.bids_address,
            &self.asks_address,
            &self.orders_key,
            &self.keypair.pubkey(),
            &self.event_queue,
            Side::Bid,
            order_id as u128,
        )
        .unwrap();

        let instructions = vec![ix];

        let recent_hash = self.rpc_client.0.get_latest_blockhash()?;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        self.rpc_client.0.send_transaction_with_config(&txn, config)
    }

    /// Settles the balance for a user in the market.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    ///
    /// # Returns
    ///
    /// A `Result` containing the transaction signature if successful,
    /// or an error if settling the balance fails.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with creating or sending the transaction.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// let s = market.settle_balance().expect("Failed to settle balance");
    /// ```
    pub fn settle_balance(&self) -> anyhow::Result<Signature, ClientError> {
        let ix = openbook_dex::instruction::settle_funds(
            &self.program_id,
            &self.market_address,
            &anchor_spl::token::ID,
            &self.orders_key,
            &self.keypair.pubkey(),
            &self.coin_vault,
            &self.wsol_ata,
            &self.pc_vault,
            &self.usdc_ata,
            None,
            &self.vault_signer_key,
        )
        .unwrap();

        let instructions = vec![ix];

        let recent_hash = self.rpc_client.0.get_latest_blockhash()?;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        self.rpc_client.0.send_transaction_with_config(&txn, config)
    }

    /// Creates a new transaction to match orders in the market.
    ///
    /// # Arguments
    ///
    /// * `limit` - The maximum number of orders to match.
    ///
    /// # Returns
    ///
    /// A transaction for matching orders.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with transaction creation or sending.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// let tx = market.make_match_orders_transaction(100)?;
    /// ```
    pub fn make_match_orders_transaction(
        &self,
        limit: u16,
    ) -> anyhow::Result<Signature, ClientError> {
        let tx = Transaction::new_with_payer(&[], Some(&self.keypair.pubkey()));

        let _match_orders_ix = openbook_dex::instruction::match_orders(
            &self.program_id,
            &self.market_address,
            &self.request_queue,
            &self.bids_address,
            &self.asks_address,
            &self.event_queue,
            &self.coin_vault,
            &self.pc_vault,
            limit,
        )
        .unwrap();

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        self.rpc_client.0.send_transaction_with_config(&tx, config)
    }

    /// Loads the bids from the market.
    ///
    /// # Returns
    ///
    /// The bids stored in the market.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with loading the bids.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let bids = market.load_bids()?;
    /// ```
    pub fn load_bids(&self) -> Result<Vec<u8>, ProgramError> {
        self.load_orders(self.bids_address)
    }

    /// Loads the asks from the market.
    ///
    /// # Returns
    ///
    /// The asks stored in the market.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with loading the asks.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// let asks = market.load_asks()?;
    /// ```
    pub fn load_asks(&self) -> Result<Vec<u8>, ProgramError> {
        self.load_orders(self.asks_address)
    }

    /// Loads orders from the specified address.
    ///
    /// # Arguments
    ///
    /// * `address` - The address from which to load orders.
    ///
    /// # Returns
    ///
    /// The orders stored at the specified address.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with loading the orders.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// let orders = market.load_orders(bids_address)?;
    /// ```
    pub fn load_orders(&self, address: Pubkey) -> Result<Vec<u8>, ProgramError> {
        let account_info: Vec<u8> = self.rpc_client.0.get_account_data(&address).unwrap();

        // TODO: decode Vec<u8>

        Ok(account_info)
    }

    /// Consumes events from the market for specified open orders accounts.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `open_orders_accounts` - A vector of `Pubkey` representing the open orders accounts.
    /// * `limit` - The maximum number of events to consume.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Signature` of the transaction or a `ClientError` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// let open_orders_accounts = vec![Pubkey::new_from_array([0; 32])];
    /// let limit = 10;
    /// let result = market.make_consume_events_instruction(open_orders_accounts, limit);
    /// ```
    pub fn make_consume_events_instruction(
        &self,
        open_orders_accounts: Vec<Pubkey>,
        limit: u16,
    ) -> Result<Signature, ClientError> {
        let consume_events_ix = openbook_dex::instruction::consume_events(
            &self.program_id,
            open_orders_accounts.iter().collect(),
            &self.market_address,
            &self.event_queue,
            &self.coin_vault,
            &self.pc_vault,
            limit,
        )
        .unwrap();

        let tx =
            Transaction::new_with_payer(&[consume_events_ix.clone()], Some(&self.keypair.pubkey()));

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        self.rpc_client.0.send_transaction_with_config(&tx, config)
    }

    /// Consumes permissioned events from the market for specified open orders accounts.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `open_orders_accounts` - A vector of `Pubkey` representing the open orders accounts.
    /// * `limit` - The maximum number of events to consume.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Signature` of the transaction or a `ClientError` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::market::read_keypair;
    ///
    /// dotenv::dotenv().ok();
    /// let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    /// let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    /// let market_address = std::env::var("SOL_USDC_MARKET_ID")
    ///     .expect("SOL_USDC_MARKET_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
    ///     .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
    ///     .parse()
    ///     .unwrap();
    /// let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    /// let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    /// let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");
    ///
    /// let rpc_client = RpcClient::new(rpc_url);
    /// let keypair = read_keypair(&key_path);
    ///
    /// let market = Market::new(rpc_client, program_id, market_address, keypair);
    /// let limit = 10;
    /// let result = market.make_consume_events_permissioned_instruction(open_orders_accounts, limit);
    /// ```
    pub fn make_consume_events_permissioned_instruction(
        &self,
        open_orders_accounts: Vec<Pubkey>,
        limit: u16,
    ) -> Result<Signature, ClientError> {
        let consume_events_permissioned_ix =
            openbook_dex::instruction::consume_events_permissioned(
                &self.program_id,
                open_orders_accounts.iter().collect(),
                &self.market_address,
                &self.event_queue,
                &self.event_queue, // TODO: Update to consume_events_authority
                limit,
            )
            .unwrap();

        let tx = Transaction::new_with_payer(
            &[consume_events_permissioned_ix.clone()],
            Some(&self.keypair.pubkey()),
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        self.rpc_client.0.send_transaction_with_config(&tx, config)
    }

    pub fn load_orders_for_owner(
        &mut self,
        cache_duration_ms: u64,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let bids = self.load_bids()?;
        let asks = self.load_asks()?;
        let open_orders_accounts =
            self.find_open_orders_accounts_for_owner(&self.keypair.pubkey(), cache_duration_ms)?;

        Ok(self.filter_for_open_orders(bids, asks, open_orders_accounts))
    }

    pub fn filter_for_open_orders(
        &self,
        bids: Vec<u8>,
        asks: Vec<u8>,
        open_orders_accounts: Vec<OpenOrders>,
    ) -> Vec<u8> {
        // Implementation of filter_for_open_orders function
        let all_orders = bids.into_iter().chain(asks);
        let orders = all_orders
            .filter(|_order| {
                open_orders_accounts.iter().any(|_oo| true) // todo fix order.address == oo.address
            })
            .collect();
        orders
    }

    pub fn find_open_orders_accounts_for_owner(
        &mut self,
        owner_address: &Pubkey,
        cache_duration_ms: u64,
    ) -> Result<Vec<OpenOrders>, Box<dyn std::error::Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();
        if let Some(cache_entry) = self.open_orders_accounts_cache.get(owner_address) {
            if now - cache_entry.ts < cache_duration_ms.into() {
                return Ok(cache_entry.accounts.clone());
            }
        }

        let open_orders_accounts_for_owner = OpenOrders::find_for_market_and_owner(
            &self.rpc_client.0,
            self.keypair.pubkey(),
            *owner_address,
            self.program_id,
            false,
        )?;
        self.open_orders_accounts_cache.insert(
            *owner_address,
            OpenOrdersCacheEntry {
                accounts: open_orders_accounts_for_owner.clone(),
                ts: now,
            },
        );

        Ok(open_orders_accounts_for_owner)
    }
}
