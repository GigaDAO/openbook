//! This module contains structs and functions related to the openbook market.
use crate::{
    orders::{MarketInfo, OpenOrders, OpenOrdersCacheEntry},
    rpc::Rpc,
    rpc_client::RpcClient,
    tokens_and_markets::{get_market_name, get_program_id, DexVersion, Token},
    utils::{get_unix_secs, u64_slice_to_bytes},
};
use anchor_spl::token::spl_token;
use anyhow::{Error, Result};
use openbook_dex::{
    critbit::Slab,
    instruction::SelfTradeBehavior,
    matching::{OrderType, Side},
    state::{gen_vault_signer_key, MarketState},
};
use rand::random;
use solana_client::client_error::ClientError;
use solana_program::{account_info::AccountInfo, instruction::Instruction, pubkey::Pubkey};
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::sysvar::slot_history::ProgramError;
use solana_sdk::{
    account::Account,
    compute_budget::ComputeBudgetInstruction,
    signature::Signature,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address,
};
use std::{
    str::FromStr,
    cell::RefMut,
    collections::HashMap,
    num::NonZeroU64,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{debug, error};

#[derive(Debug)]
pub enum OrderReturnType {
    Instructions(Vec<Instruction>),
    Signature(Signature),
}

/// Struct representing a market with associated state and information.
#[derive(Debug)]
pub struct Market {
    /// The RPC client for interacting with the Solana blockchain.
    pub rpc_client: Rpc,

    /// The public key of the program associated with the market.
    pub program_id: Pubkey,

    /// The public key of the market.
    pub market_address: Pubkey,

    /// The keypair of the owner used for signing transactions related to the market.
    pub owner: Keypair,

    /// The number of decimal places for the base currency (coin) in the market.
    pub coin_decimals: u8,

    /// The number of decimal places for the quote currency (pc) in the market.
    pub pc_decimals: u8,

    /// The lot size for the base currency (coin) in the market.
    pub coin_lot_size: u64,

    /// The account flags associated with the market.
    pub account_flags: u64,

    /// The lot size for the quote currency (pc) in the market.
    pub pc_lot_size: u64,

    /// The public key of the associated account holding the quote quote tokens.
    pub quote_ata: Pubkey,

    /// The public key of the associated account holding the base quote tokens.
    pub base_ata: Pubkey,

    /// The public key of the market quote mint.
    pub quote_mint: Pubkey,

    /// The public key of the market base mint.
    pub base_mint: Pubkey,

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

    /// A HashMap containing open orders cache entries associated with their public keys.
    pub open_orders_accounts_cache: HashMap<Pubkey, OpenOrdersCacheEntry>,

    /// Account info of the wallet on the market (e.g. open orders).
    pub market_info: MarketInfo,
}

impl Market {
    /// Initializes a new instance of the `Market` struct, representing an OpenBook market on the Solana blockchain.
    ///
    /// This method initializes the `Market` struct, containing information about the requested market,
    /// having the base and quote mints. It fetches and stores all data about this OpenBook market.
    /// Additionally, it includes information about the account associated with the wallet on the OpenBook market
    /// (e.g., open orders, bids, asks, etc.).
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - RPC client for interacting with Solana blockchain.
    /// * `program_version` - Program dex version representing the market.
    /// * `base_mint` - Base mint symbol.
    /// * `quote_mint` - Quote mint symbol.
    /// * `owner` - Keypair of the owner used for signing transactions.
    /// * `load` - Boolean indicating whether to load market data immediately.
    ///
    /// # Returns
    ///
    /// Returns an instance of the `Market` struct with default values and configurations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let owner = read_keypair(&key_path);
    ///
    ///     let market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, owner, true).await?;
    ///
    ///     println!("Initialized Market: {:?}", market);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(
        rpc_client: RpcClient,
        program_version: DexVersion,
        base_mint: Token,
        quote_mint: Token,
        owner: Keypair,
        load: bool,
    ) -> Result<Self, Error> {
        let market_address = get_market_name(base_mint).0.parse()?;
        let quote_mint = get_market_name(quote_mint).1.parse()?;
        let base_mint = get_market_name(base_mint).1.parse()?;
        let orders_key = Default::default();
        let coin_vault = Default::default();
        let quote_ata = Default::default();
        let base_ata = Default::default();
        let pc_vault = Default::default();
        let vault_signer_key = Default::default();
        let event_queue = Default::default();
        let request_queue = Default::default();
        let market_info = Default::default();

        let program_id = get_program_id(program_version).parse()?;
        let rpc_client = Rpc::new(rpc_client);
        let pub_owner_key = owner.pubkey().clone();
        let owner_bytes = owner.to_bytes();
        let cloned_owner = Keypair::from_bytes(&owner_bytes)?;
        let open_orders =
            OpenOrders::new(rpc_client.clone(), program_id, cloned_owner, market_address).await?;
        let mut open_orders_accounts_cache = HashMap::new();

        let open_orders_cache_entry = OpenOrdersCacheEntry {
            accounts: vec![open_orders],
            ts: 123456789,
        };

        open_orders_accounts_cache.insert(pub_owner_key, open_orders_cache_entry.clone());

        let mut market = Self {
            rpc_client,
            program_id,
            market_address,
            owner,
            coin_decimals: 9,
            pc_decimals: 6,
            coin_lot_size: 1_000_000,
            pc_lot_size: 1,
            quote_ata,
            base_ata,
            quote_mint,
            base_mint,
            coin_vault,
            pc_vault,
            vault_signer_key,
            orders_key,
            event_queue,
            request_queue,
            account_flags: 0,
            open_orders_accounts_cache,
            market_info,
        };

        market.orders_key = open_orders_cache_entry.accounts[0].address;

        if load {
            market.load().await?;
        }

        let (_, vault_signer_key) = {
            let mut i = 0;
            loop {
                assert!(i < 100);
                if let Ok(pk) = gen_vault_signer_key(i, &market_address, &program_id) {
                    break (i, pk);
                }
                i += 1;
            }
        };
        market.vault_signer_key = vault_signer_key;

        let oos_key_str = std::env::var("OOS_KEY").unwrap_or("".to_string());

        let orders_key = Pubkey::from_str(oos_key_str.as_str());

        if !orders_key.is_err() {
            market.orders_key = orders_key.unwrap();
        }

        market.base_ata = get_associated_token_address(&pub_owner_key.clone(), &market.base_mint);
        market.quote_ata = get_associated_token_address(&pub_owner_key.clone(), &market.quote_mint);

        Ok(market)
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
    pub async fn load(&mut self) -> Result<MarketState> {
        let mut account = self
            .rpc_client
            .inner()
            .get_account(&self.market_address)
            .await?;
        let owner = account.owner;
        let program_id_binding = self.program_id;
        let market_account_binding = self.market_address;
        let account_info;
        {
            account_info = self.create_account_info_from_account(
                &mut account,
                &market_account_binding,
                &program_id_binding,
                false,
                false,
            );
        }
        if self.program_id != owner {
            return Err(ProgramError::InvalidArgument.into());
        }

        let market_state = self.load_market_state_bids_info(&account_info).await?;

        Ok(*market_state)
    }

    /// Loads the market state and bids information from the provided account information.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the `Market` struct.
    /// * `account_info` - A reference to the account information used to load the market state.
    ///
    /// # Returns
    ///
    /// A `Result` containing a mutable reference to the loaded `MarketState` if successful,
    /// or an error if loading the market state fails.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with loading the market state.
    pub async fn load_market_state_bids_info<'a>(
        &'a mut self,
        account_info: &'a AccountInfo<'_>,
    ) -> Result<RefMut<MarketState>> {
        let mut market_state = MarketState::load(account_info, &self.program_id, false)?;

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
        let _result = self.load_bids_asks_info(&mut market_state).await?;

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
    pub async fn load_bids_asks_info(
        &mut self,
        market_state: &RefMut<'_, MarketState>,
    ) -> Result<(Pubkey, Pubkey, MarketInfo)> {
        let (bids_address, asks_address) = self.get_bids_asks_addresses(market_state);

        let mut bids_account = self.rpc_client.inner().get_account(&bids_address).await?;
        let bids_info = self.create_account_info_from_account(
            &mut bids_account,
            &bids_address,
            &self.program_id,
            false,
            false,
        );
        let mut bids = market_state.load_bids_mut(&bids_info)?;
        let (open_bids, open_bids_prices, max_bid) = self.process_bids(&mut bids)?;

        let mut asks_account = self.rpc_client.inner().get_account(&asks_address).await?;
        let asks_info = self.create_account_info_from_account(
            &mut asks_account,
            &asks_address,
            &self.program_id,
            false,
            false,
        );
        let mut asks = market_state.load_asks_mut(&asks_info)?;
        let (open_asks, open_asks_prices, min_ask) = self.process_asks(&mut asks)?;

        self.market_info = MarketInfo {
            min_ask,
            max_bid,
            open_asks,
            open_bids,
            bids_address,
            asks_address,
            open_asks_prices,
            open_bids_prices,
            base_total: 0.,
            quote_total: 0.,
        };

        Ok((bids_address, asks_address, self.market_info.clone()))
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
    pub fn process_bids(&self, bids: &mut RefMut<Slab>) -> Result<(Vec<u128>, Vec<f64>, u64)> {
        let mut max_bid = 0;
        let mut open_bids = Vec::new();
        let mut open_bids_prices = Vec::new();
        let node = bids.remove_max();
        match node {
            Some(node) => {
                let owner = node.owner();
                let bytes = u64_slice_to_bytes(owner);
                let owner_address = Pubkey::from(bytes);

                let order_id = node.order_id();
                let price_raw = node.price().get();
                let ui_price = price_raw as f64 / 1e4;

                debug!("[*] Bid: {price_raw}");

                if max_bid == 0 {
                    max_bid = price_raw;
                }

                if owner_address == self.orders_key {
                    open_bids.push(order_id);
                    open_bids_prices.push(ui_price);
                }
            }
            None => {}
        }
        Ok((open_bids, open_bids_prices, max_bid))
    }

    /// Processes asks information to fetch asks info.
    ///
    /// This function iteratively removes asks from the provided `Slab` until
    /// it finds the all asks.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `asks` - A mutable reference to the `Slab` containing asks information.
    ///
    /// # Returns
    ///
    /// A `Result` containing the maximum bid price if successful, or an error if processing asks fails.
    pub fn process_asks(&self, asks: &mut RefMut<Slab>) -> Result<(Vec<u128>, Vec<f64>, u64)> {
        let mut min_ask = 0;
        let mut open_asks = Vec::new();
        let mut open_asks_prices = Vec::new();
        loop {
            let node = asks.remove_min();
            match node {
                Some(node) => {
                    let owner = node.owner();
                    let bytes = u64_slice_to_bytes(owner);
                    let owner_address = Pubkey::from(bytes);

                    let order_id = node.order_id();
                    let price_raw = node.price().get();
                    let ui_price = price_raw as f64 / 1e4;

                    debug!("[*] Ask: {price_raw}");

                    if min_ask == 0 {
                        min_ask = price_raw;
                    }

                    if owner_address == self.orders_key {
                        open_asks.push(order_id);
                        open_asks_prices.push(ui_price);
                    }
                }
                None => {
                    break;
                }
            }
        }
        Ok((open_asks, open_asks_prices, min_ask))
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
    /// use openbook::utils::read_keypair;
    /// use openbook::state::MarketState;
    /// use openbook::tokens_and_markets::{get_market_name, get_program_id};
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let program_id = get_program_id(DexVersion::default()).parse()?;
    ///     let market_address = get_market_name(Token::USDC).0.parse()?;
    ///
    ///     let rpc_client1 = RpcClient::new(rpc_url.clone());
    ///     let rpc_client2 = RpcClient::new(rpc_url.clone());
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let market = Market::new(rpc_client1, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let mut account = rpc_client2.get_account(&market_address).await?;
    ///
    ///     let account_info = market.create_account_info_from_account(
    ///         &mut account,
    ///         &market_address,
    ///         &program_id,
    ///         false,
    ///         false,
    ///     );
    ///
    ///     let mut market_state = MarketState::load(&account_info, &program_id, false)?;
    ///     let (bids_address, asks_address) = market.get_bids_asks_addresses(&market_state);
    ///
    ///     assert_eq!(&bids_address.to_string(), "5jWUncPNBMZJ3sTHKmMLszypVkoRK6bfEQMQUHweeQnh");
    ///     assert_eq!(&asks_address.to_string(), "EaXdHx7x3mdGA38j5RSmKYSXMzAFzzUXCLNBEDXDn1d5");
    ///
    ///     Ok(())
    /// }
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
    /// use openbook::tokens_and_markets::{get_market_name, get_program_id};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::state::MarketState;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let program_id = get_program_id(DexVersion::default()).parse()?;
    ///     let market_address = get_market_name(Token::USDC).0.parse()?;
    ///
    ///     let rpc_client1 = RpcClient::new(rpc_url.clone());
    ///     let rpc_client2 = RpcClient::new(rpc_url.clone());
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let market = Market::new(rpc_client1, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let mut account = rpc_client2.get_account(&market_address).await?;
    ///
    ///     let account_info = market.create_account_info_from_account(
    ///         &mut account,
    ///         &market_address,
    ///         &program_id,
    ///         false,
    ///         false,
    ///     );
    ///
    ///     println!("{:?}", account_info);
    ///
    ///     Ok(())
    /// }
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

    /// Places a limit order on the market.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `target_amount_quote` - The target amount in quote currency for the order.
    /// * `side` - The side of the order (buy or sell).
    /// * `best_offset_usdc` - The best offset in USDC for the order.
    /// * `execute` - A boolean indicating whether to execute the order immediately.
    /// * `target_price` - The target price for the order.
    ///
    /// # Returns
    ///
    /// A `Result` containing the transaction signature if successful,
    /// or an error if placing the limit order fails.
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
    /// use openbook::utils::read_keypair;
    /// use openbook::matching::Side;
    /// use openbook::market::OrderReturnType;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let mut market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let target_amount_quote = 5.0;
    ///     let side = Side::Bid; // or Side::Ask
    ///     let best_offset_usdc = 5.0;
    ///     let execute = true;
    ///     let target_price = 2.1;
    ///
    ///     if let Some(ord_ret_type) = market.place_limit_order(
    ///         target_amount_quote,
    ///         side,
    ///         best_offset_usdc,
    ///         execute,
    ///         target_price
    ///     ).await?
    ///     {
    ///         match ord_ret_type {
    ///             OrderReturnType::Instructions(insts) => {
    ///                 println!("[*] Got Instructions: {:?}", insts);
    ///             }
    ///             OrderReturnType::Signature(sign) => {
    ///                 println!("[*] Transaction successful, signature: {:?}", sign);
    ///             }
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn place_limit_order(
        &self,
        target_amount_quote: f64,
        side: Side,
        best_offset_usdc: f64,
        execute: bool,
        target_price: f64,
    ) -> Result<Option<OrderReturnType>, Error> {
        // coin: base
        // pc: quote
        let base_d_factor = 10u32.pow(self.coin_decimals as u32) as f64;
        let quote_d_factor = 10u32.pow(self.pc_decimals as u32) as f64;
        let base_lot_factor = self.coin_lot_size as f64;
        let quote_lot_factor = self.pc_lot_size as f64;

        let price_factor = quote_d_factor * base_lot_factor / base_d_factor / quote_lot_factor;

        let (input_ata, price) = match side {
            Side::Bid => {
                let mut price = self.market_info.max_bid as f64 / price_factor - best_offset_usdc;
                if execute {
                    price = target_price;
                }

                (&self.quote_ata, price)
            }
            Side::Ask => {
                let mut price = self.market_info.min_ask as f64 / price_factor + best_offset_usdc;
                if execute {
                    price = target_price;
                }

                (&self.base_ata, price)
            }
        };

        let limit_price_lots = (price * price_factor) as u64;
        let target_amount_base = target_amount_quote / price;

        let target_base_lots = (target_amount_base * base_d_factor / base_lot_factor) as u64;
        let target_quote_lots_w_fee =
            (target_base_lots as f64 * quote_lot_factor * limit_price_lots as f64) as u64;

        debug!("[*] Using limit price lots: {:?}", limit_price_lots);
        debug!("[*] Using target base lots: {:?}", target_base_lots);

        if target_base_lots == 0 {
            debug!(
                "[*] Got zero base lots, and quote: {:?}",
                target_amount_quote
            );
            return Ok(None);
        }

        let limit_price = NonZeroU64::new(limit_price_lots).unwrap();
        let max_coin_qty = NonZeroU64::new(target_base_lots).unwrap(); // max wsol lots
        let max_native_pc_qty_including_fees = NonZeroU64::new(target_quote_lots_w_fee).unwrap(); // max usdc lots + fees

        let place_order_ix = openbook_dex::instruction::new_order(
            &self.market_address,
            &self.orders_key,
            &self.request_queue,
            &self.event_queue,
            &self.market_info.bids_address,
            &self.market_info.asks_address,
            input_ata,
            &self.owner.pubkey(),
            &self.coin_vault,
            &self.pc_vault,
            &spl_token::ID,
            &solana_program::sysvar::rent::ID,
            None,
            &self.program_id,
            side,
            limit_price,
            max_coin_qty,
            OrderType::PostOnly,
            random::<u64>(),
            SelfTradeBehavior::AbortTransaction,
            u16::MAX,
            max_native_pc_qty_including_fees,
            (get_unix_secs() + 30) as i64,
        )?;

        let instructions = vec![place_order_ix];

        if !execute {
            return Ok(Some(OrderReturnType::Instructions(instructions)));
        }

        let recent_hash = self
            .rpc_client
            .inner()
            .get_latest_blockhash_with_commitment(self.rpc_client.inner().commitment())
            .await?
            .0;

        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.owner.pubkey()),
            &[&self.owner],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = false;
        config.preflight_commitment = Some(self.rpc_client.inner().commitment().commitment);
        let signature = self
            .rpc_client
            .inner()
            .send_transaction_with_config(&txn, config)
            .await?;

        Ok(Some(OrderReturnType::Signature(signature)))
    }

    /// Cancels all limit orders in the market.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `execute` - A boolean indicating whether to execute the order immediately.
    ///
    /// # Returns
    ///
    /// A `Result` containing the transaction signature if successful,
    /// or an error if canceling all orders fails.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with creating or sending the transaction.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::OrderReturnType;
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     if let Some(ord_ret_type) = market
    ///         .cancel_orders(true)
    ///         .await?
    ///     {
    ///         match ord_ret_type {
    ///             OrderReturnType::Instructions(insts) => {
    ///                 println!("[*] Got Instructions: {:?}", insts);
    ///             }
    ///             OrderReturnType::Signature(sign) => {
    ///                 println!("[*] Transaction successful, signature: {:?}", sign);
    ///             }
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn cancel_orders(&self, execute: bool) -> Result<Option<OrderReturnType>, Error> {
        let mut ixs = Vec::new();

        for oid in &self.market_info.open_bids {
            let ix = openbook_dex::instruction::cancel_order(
                &self.program_id,
                &self.market_address,
                &self.market_info.bids_address,
                &self.market_info.asks_address,
                &self.orders_key,
                &self.owner.pubkey(),
                &self.event_queue,
                Side::Bid,
                *oid,
            )?;
            ixs.push(ix);
        }

        for oid in &self.market_info.open_asks {
            let ix = openbook_dex::instruction::cancel_order(
                &self.program_id,
                &self.market_address,
                &self.market_info.bids_address,
                &self.market_info.asks_address,
                &self.orders_key,
                &self.owner.pubkey(),
                &self.event_queue,
                Side::Ask,
                *oid,
            )?;
            ixs.push(ix);
        }

        if ixs.is_empty() {
            return Ok(None);
        }

        if !execute {
            return Ok(Some(OrderReturnType::Instructions(ixs)));
        }

        let recent_hash = self
            .rpc_client
            .inner()
            .get_latest_blockhash_with_commitment(self.rpc_client.inner().commitment())
            .await?
            .0;
        let txn = Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.owner.pubkey()),
            &[&self.owner],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        config.preflight_commitment = Some(self.rpc_client.inner().commitment().commitment);

        Ok(Some(OrderReturnType::Signature(
            self.rpc_client
                .inner()
                .send_transaction_with_config(&txn, config)
                .await?,
        )))
    }

    /// Settles the balance for a user in the market.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `execute` - A boolean indicating whether to execute the order immediately.
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
    /// use openbook::utils::read_keypair;
    /// use openbook::market::OrderReturnType;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     if let Some(ord_ret_type) = market
    ///         .settle_balance(true)
    ///         .await?
    ///     {
    ///         match ord_ret_type {
    ///             OrderReturnType::Instructions(insts) => {
    ///                 println!("[*] Got Instructions: {:?}", insts);
    ///             }
    ///             OrderReturnType::Signature(sign) => {
    ///                 println!("[*] Transaction successful, signature: {:?}", sign);
    ///             }
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn settle_balance(&self, execute: bool) -> Result<Option<OrderReturnType>, Error> {
        let ix = openbook_dex::instruction::settle_funds(
            &self.program_id,
            &self.market_address,
            &anchor_spl::token::ID,
            &self.orders_key,
            &self.owner.pubkey(),
            &self.coin_vault,
            &self.base_ata,
            &self.pc_vault,
            &self.quote_ata,
            None,
            &self.vault_signer_key,
        )?;

        let instructions = vec![ix];

        if !execute {
            return Ok(Some(OrderReturnType::Instructions(instructions)));
        }

        let recent_hash = self
            .rpc_client
            .inner()
            .get_latest_blockhash_with_commitment(self.rpc_client.inner().commitment())
            .await?
            .0;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.owner.pubkey()),
            &[&self.owner],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        config.preflight_commitment = Some(self.rpc_client.inner().commitment().commitment);
        Ok(Some(OrderReturnType::Signature(
            self.rpc_client
                .inner()
                .send_transaction_with_config(&txn, config)
                .await?,
        )))
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
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let result = market.make_match_orders_transaction(100).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn make_match_orders_transaction(
        &self,
        limit: u16,
    ) -> Result<Signature, ClientError> {
        let ix = openbook_dex::instruction::match_orders(
            &self.program_id,
            &self.market_address,
            &self.request_queue,
            &self.market_info.bids_address,
            &self.market_info.asks_address,
            &self.event_queue,
            &self.coin_vault,
            &self.pc_vault,
            limit,
        )
        .unwrap();

        let instructions = vec![ix];

        let recent_hash = self
            .rpc_client
            .inner()
            .get_latest_blockhash_with_commitment(self.rpc_client.inner().commitment())
            .await?
            .0;

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.owner.pubkey()),
            &[&self.owner],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        config.preflight_commitment = Some(self.rpc_client.inner().commitment().commitment);
        self.rpc_client
            .inner()
            .send_transaction_with_config(&tx, config)
            .await
    }

    /// Executes a combination of canceling all limit orders, settling balance, and placing new bid and ask orders.
    ///
    /// # Arguments
    ///
    /// * `target_size_usdc_ask` - The target size in USDC for the ask order.
    /// * `target_size_usdc_bid` - The target size in USDC for the bid order.
    /// * `bid_price_jlp_usdc` - The bid price in JLP/USDC.
    /// * `ask_price_jlp_usdc` - The ask price in JLP/USDC.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let mut market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let target_size_usdc_ask = 0.5;
    ///     let target_size_usdc_bid = 1.0;
    ///     let bid_price_jlp_usdc = 1.5;
    ///     let ask_price_jlp_usdc = 2.5;
    ///
    ///     let result = market.cancel_settle_place(target_size_usdc_ask, target_size_usdc_bid, bid_price_jlp_usdc, ask_price_jlp_usdc).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn cancel_settle_place(
        &mut self,
        target_size_usdc_ask: f64,
        target_size_usdc_bid: f64,
        bid_price_jlp_usdc: f64,
        ask_price_jlp_usdc: f64,
    ) -> Result<Option<Signature>> {
        let mut instructions = Vec::new();

        // Fetch recent prioritization fees
        let r = self
            .rpc_client
            .inner()
            .get_recent_prioritization_fees(&[])
            .await?;
        let mut max_fee = 1;
        for f in r {
            if f.prioritization_fee > max_fee {
                max_fee = f.prioritization_fee;
            }
        }

        // Set compute budget and fee instructions
        let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);
        let fee_ix = ComputeBudgetInstruction::set_compute_unit_price(max_fee);
        instructions.push(budget_ix);
        instructions.push(fee_ix);

        // Cancel all limit orders
        if let Some(ord_ret_type) = self.cancel_orders(false).await? {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Settle balance
        if let Some(ord_ret_type) = self.settle_balance(false).await? {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Place bid order
        if let Some(ord_ret_type) = self
            .place_limit_order(
                target_size_usdc_bid,
                Side::Bid,
                0.,
                false,
                bid_price_jlp_usdc,
            )
            .await?
        {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Place ask order
        if let Some(ord_ret_type) = self
            .place_limit_order(
                target_size_usdc_ask,
                Side::Ask,
                0.,
                false,
                ask_price_jlp_usdc,
            )
            .await?
        {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Build and send transaction
        let recent_hash = self
            .rpc_client
            .inner()
            .get_latest_blockhash_with_commitment(self.rpc_client.inner().commitment())
            .await?
            .0;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.owner.pubkey()),
            &[&self.owner],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        config.preflight_commitment = Some(self.rpc_client.inner().commitment().commitment);
        let kp_str = self.owner.pubkey().to_string().clone();
        match self
            .rpc_client
            .inner()
            .send_transaction_with_config(&txn, config)
            .await
        {
            Ok(sign) => Ok(Some(sign)),
            Err(err) => {
                error!("[*] err combo'ing: {err}, {}", kp_str);
                Ok(None)
            }
        }
    }

    /// Executes a combination of canceling all limit orders, settling balance, and placing a bid order.
    ///
    /// # Arguments
    ///
    /// * `target_size_usdc_bid` - The target size in USDC for the bid order.
    /// * `bid_price_jlp_usdc` - The bid price in JLP/USDC.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let mut market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let result = market.cancel_settle_place_bid(1.5, 1.0).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn cancel_settle_place_bid(
        &mut self,
        target_size_usdc_bid: f64,
        bid_price_jlp_usdc: f64,
    ) -> Result<Option<Signature>> {
        let mut instructions = Vec::new();

        // Fetch recent prioritization fees
        let r = self
            .rpc_client
            .inner()
            .get_recent_prioritization_fees(&[])
            .await?;
        let mut max_fee = 1;
        for f in r {
            if f.prioritization_fee > max_fee {
                max_fee = f.prioritization_fee;
            }
        }

        // Set compute budget and fee instructions
        let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(800_000);
        let fee_ix = ComputeBudgetInstruction::set_compute_unit_price(max_fee);
        instructions.push(budget_ix);
        instructions.push(fee_ix);

        // Cancel all limit orders
        if let Some(ord_ret_type) = self.cancel_orders(false).await? {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Settle balance
        if let Some(ord_ret_type) = self.settle_balance(false).await? {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Place bid order
        if let Some(ord_ret_type) = self
            .place_limit_order(
                target_size_usdc_bid,
                Side::Bid,
                0.,
                false,
                bid_price_jlp_usdc,
            )
            .await?
        {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Build and send transaction
        let recent_hash = self
            .rpc_client
            .inner()
            .get_latest_blockhash_with_commitment(self.rpc_client.inner().commitment())
            .await?
            .0;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.owner.pubkey()),
            &[&self.owner],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        config.preflight_commitment = Some(self.rpc_client.inner().commitment().commitment);
        let kp_str = self.owner.pubkey().to_string().clone();
        match self
            .rpc_client
            .inner()
            .send_transaction_with_config(&txn, config)
            .await
        {
            Ok(sign) => Ok(Some(sign)),
            Err(err) => {
                error!("[*] err bidding: {err}, {}", kp_str);
                let e = err.get_transaction_error();
                if let Some(e) = e {
                    error!("[*] got tx err: {e}");
                }
                Ok(None)
            }
        }
    }

    /// Executes a combination of canceling all limit orders, settling balance, and placing an ask order.
    ///
    /// # Arguments
    ///
    /// * `target_size_usdc_ask` - The target size in USDC for the ask order.
    /// * `ask_price_jlp_usdc` - The ask price in JLP/USDC.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let mut market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let result = market.cancel_settle_place_ask(1.5, 1.0).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn cancel_settle_place_ask(
        &mut self,
        target_size_usdc_ask: f64,
        ask_price_jlp_usdc: f64,
    ) -> Result<Option<Signature>> {
        let mut instructions = Vec::new();

        // Fetch recent prioritization fees
        let r = self
            .rpc_client
            .inner()
            .get_recent_prioritization_fees(&[])
            .await?;

        let mut max_fee = 1;
        for f in r {
            if f.prioritization_fee > max_fee {
                max_fee = f.prioritization_fee;
            }
        }

        // Set compute budget and fee instructions
        let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(800_000);
        let fee_ix = ComputeBudgetInstruction::set_compute_unit_price(max_fee);
        instructions.push(budget_ix);
        instructions.push(fee_ix);

        // Cancel all limit orders
        if let Some(ord_ret_type) = self.cancel_orders(false).await? {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Settle balance
        if let Some(ord_ret_type) = self.settle_balance(false).await? {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Place ask order
        if let Some(ord_ret_type) = self
            .place_limit_order(
                target_size_usdc_ask,
                Side::Ask,
                0.,
                false,
                ask_price_jlp_usdc,
            )
            .await?
        {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Build and send transaction
        let recent_hash = self
            .rpc_client
            .inner()
            .get_latest_blockhash_with_commitment(self.rpc_client.inner().commitment())
            .await?
            .0;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.owner.pubkey()),
            &[&self.owner],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        config.preflight_commitment = Some(self.rpc_client.inner().commitment().commitment);
        let kp_str = self.owner.pubkey().to_string().clone();
        match self
            .rpc_client
            .inner()
            .send_transaction_with_config(&txn, config)
            .await
        {
            Ok(sign) => Ok(Some(sign)),
            Err(err) => {
                error!("[*] err asking: {err}, {}", kp_str);
                Ok(None)
            }
        }
    }

    /// Executes a combination of canceling all limit orders and settling balance.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let mut market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let result = market.cancel_settle().await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn cancel_settle(&mut self) -> Result<Option<Signature>> {
        let mut instructions = Vec::new();

        // Fetch recent prioritization fees
        let r = self
            .rpc_client
            .inner()
            .get_recent_prioritization_fees(&[])
            .await?;
        let mut max_fee = 1;
        for f in r {
            if f.prioritization_fee > max_fee {
                max_fee = f.prioritization_fee;
            }
        }

        // Set compute budget and fee instructions
        let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(800_000);
        let fee_ix = ComputeBudgetInstruction::set_compute_unit_price(max_fee);
        instructions.push(budget_ix);
        instructions.push(fee_ix);

        // Cancel all limit orders
        if let Some(ord_ret_type) = self.cancel_orders(false).await? {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Settle balance
        if let Some(ord_ret_type) = self.settle_balance(false).await? {
            match ord_ret_type {
                OrderReturnType::Instructions(insts) => {
                    instructions.extend(insts);
                }
                OrderReturnType::Signature(_) => {}
            }
        }

        // Build and send transaction
        let recent_hash = self
            .rpc_client
            .inner()
            .get_latest_blockhash_with_commitment(self.rpc_client.inner().commitment())
            .await?
            .0;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.owner.pubkey()),
            &[&self.owner],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        config.preflight_commitment = Some(self.rpc_client.inner().commitment().commitment);
        let kp_str = self.owner.pubkey().to_string().clone();

        match self
            .rpc_client
            .inner()
            .send_transaction_with_config(&txn, config)
            .await
        {
            Ok(sign) => Ok(Some(sign)),
            Err(err) => {
                error!("[*] err canceling: {err}\npk: {}", kp_str);
                Ok(None)
            }
        }
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
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let mut market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let result = market.load_bids()?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn load_bids(&mut self) -> Result<Vec<u128>, ProgramError> {
        Ok(self.market_info.open_bids.clone())
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
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let mut market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let result = market.load_asks()?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn load_asks(&mut self) -> Result<Vec<u128>, ProgramError> {
        Ok(self.market_info.open_asks.clone())
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
    /// A `Result` containing the `Signature` of the transaction or a `Error` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let open_orders_accounts = vec![Pubkey::new_from_array([0; 32])];
    ///     let limit = 10;
    ///     let result = market.make_consume_events_instruction(open_orders_accounts, limit).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn make_consume_events_instruction(
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
            Transaction::new_with_payer(&[consume_events_ix.clone()], Some(&self.owner.pubkey()));

        let mut config = RpcSendTransactionConfig::default();
        config.preflight_commitment = Some(self.rpc_client.inner().commitment().commitment);
        config.skip_preflight = true;
        self.rpc_client
            .inner()
            .send_transaction_with_config(&tx, config)
            .await
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
    /// A `Result` containing the `Signature` of the transaction or a `Error` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let open_orders_accounts = vec![Pubkey::new_from_array([0; 32])];
    ///     let limit = 10;
    ///     let result = market.make_consume_events_permissioned_instruction(open_orders_accounts, limit).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn make_consume_events_permissioned_instruction(
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
            Some(&self.owner.pubkey()),
        );

        let mut config = RpcSendTransactionConfig::default();
        config.preflight_commitment = Some(self.rpc_client.inner().commitment().commitment);
        config.skip_preflight = true;
        self.rpc_client
            .inner()
            .send_transaction_with_config(&tx, config)
            .await
    }

    /// Loads open orders accounts for the owner, filtering them based on bids and asks.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the `Market` struct.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `Account` representing open orders accounts or a boxed `Error` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let mut market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let result = market.load_orders_for_owner().await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn load_orders_for_owner(&mut self) -> Result<Vec<u128>, Box<dyn std::error::Error>> {
        let mut bids = self.load_bids()?;
        let asks = self.load_asks()?;
        bids.extend(asks);
        // let open_orders_accounts = self
        //     .find_open_orders_accounts_for_owner(
        //         &self.orders_key.clone(),
        //         5000,
        //     )
        //     .await?;

        Ok(bids)
    }

    /// Filters open orders accounts based on bids and asks.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `bids` - A `MarketInfo` struct representing bids information.
    /// * `asks` - A `MarketInfo` struct representing asks information.
    /// * `open_orders_accounts` - A vector of `OpenOrders` representing open orders accounts.
    ///
    /// # Returns
    ///
    /// A filtered vector of `OpenOrders` based on bids and asks addresses.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///
    ///     let bids = market.market_info.clone();
    ///     let asks = market.market_info.clone();
    ///
    ///     let open_orders_accounts = vec![];
    ///     let result = market.filter_for_open_orders(bids, asks, open_orders_accounts);
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn filter_for_open_orders(
        &self,
        bids: MarketInfo,
        asks: MarketInfo,
        open_orders_accounts: Vec<OpenOrders>,
    ) -> Vec<OpenOrders> {
        let bids_address = bids.bids_address;
        let asks_address = asks.asks_address;

        open_orders_accounts
            .into_iter()
            .filter(|open_orders| {
                open_orders.address == bids_address || open_orders.address == asks_address
            })
            .collect()
    }

    /// Finds open orders accounts for a specified owner and caches them based on the specified duration.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the `Market` struct.
    /// * `owner_address` - A reference to the owner's `Pubkey`.
    /// * `cache_duration_ms` - The duration in milliseconds for which to cache open orders accounts.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `Account` representing open orders accounts or a boxed `Error` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
    /// use openbook::market::Market;
    /// use openbook::utils::read_keypair;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    ///
    ///     let rpc_client = RpcClient::new(rpc_url);
    ///
    ///     let keypair = read_keypair(&key_path);
    ///
    ///     let mut market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, keypair, true).await?;
    ///     let owner_address = &Pubkey::new_from_array([0; 32]);
    ///
    ///     let result = market.find_open_orders_accounts_for_owner(&owner_address, 5000).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn find_open_orders_accounts_for_owner(
        &mut self,
        owner_address: &Pubkey,
        cache_duration_ms: u64,
    ) -> Result<Vec<(Pubkey, Account)>, Box<dyn std::error::Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();
        if let Some(cache_entry) = self.open_orders_accounts_cache.get(owner_address) {
            if now - cache_entry.ts < cache_duration_ms.into() {
                // return Ok(cache_entry.accounts.clone());
            }
        }

        let open_orders_accounts_for_owner = OpenOrders::find_for_market_and_owner(
            &self.rpc_client,
            self.owner.pubkey(),
            *owner_address,
            false,
        )
        .await?;
        // self.open_orders_accounts_cache.insert(
        //     *owner_address,
        //     OpenOrdersCacheEntry {
        //         accounts: open_orders_accounts_for_owner.clone(),
        //         ts: now,
        //     },
        // );

        Ok(open_orders_accounts_for_owner)
    }
}
