use crate::v1::{
    market::Market,
    orders::{OpenOrders, OpenOrdersCacheEntry, OrderReturnType},
};
use crate::{
    rpc::Rpc,
    rpc_client::RpcClient,
    utils::{create_account_info_from_account, get_unix_secs, read_keypair, u64_slice_to_pubkey},
    v1::traits::{MarketInfo, OpenOrdersT},
};

use anyhow::{Error, Result};
use openbook_dex::{
    critbit::Slab,
    instruction::SelfTradeBehavior,
    matching::{OrderType, Side},
    state::{Market as MarketAuth, MarketState},
};
use rand::random;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    signer::keypair::Keypair,
    sysvar::{rent, slot_history::ProgramError},
};
use spl_associated_token_account::get_associated_token_address;
use std::{
    cell::RefMut,
    collections::HashMap,
    fmt::{Debug, Formatter},
    num::NonZeroU64,
    str::FromStr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::debug;

pub static SPL_TOKEN_ID: &'static str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub static SRM_PROGRAM_ID: &'static str = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX";

/// OpenBook v1 Client to interact with the OpenBook market and perform actions.
#[derive(Clone)]
pub struct OBClient {
    /// The keypair of the owner used for signing transactions related to the market.
    pub owner: Arc<Keypair>,
    /// The RPC client for interacting with the Solana blockchain.
    pub rpc_client: Rpc,
    /// The public key of the associated account holding the quote tokens.
    pub quote_ata: Pubkey,
    /// The public key of the associated account holding the base tokens.
    pub base_ata: Pubkey,
    /// Account info of the wallet on the market (e.g., open orders).
    pub open_orders: OpenOrders,
    /// Information about the OpenBook market.
    pub market_info: Market,
    /// A HashMap containing open orders cache entries associated with their public keys.
    pub open_orders_cache: HashMap<Pubkey, OpenOrdersCacheEntry>,
}

impl Debug for OBClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(f, "OB_V1_Client {{")?;
        writeln!(f, "    owner: {:?}", self.owner.pubkey())?;
        writeln!(f, "    rpc_client: {:?}", self.rpc_client)?;
        writeln!(f, "    quote_ata: {:?}", self.quote_ata)?;
        writeln!(f, "    base_ata: {:?}", self.base_ata)?;
        writeln!(f, "    open_orders: {:?}", self.open_orders)?;
        writeln!(f, "    market_info: {:?}", self.market_info)?;
        writeln!(f, "}}")
    }
}

impl OBClient {
    /// Initializes a new instance of the `OBClient` struct, representing an OpenBook V1 program client.
    ///
    /// This method initializes the `OBClient` struct, containing information about the requested market id.
    /// It fetches and stores all data about this OpenBook market. Additionally, it includes information about
    /// the account associated with the wallet on the OpenBook market (e.g., open orders, bids, asks, etc.).
    ///
    /// # Arguments
    ///
    /// * `commitment` - Commitment configuration for transactions, determining the level of finality required.
    /// * `market_id` - Public key (ID) of the market to fetch information about.
    /// * `load` - Boolean indicating whether to load market data immediately after initialization.
    /// * `cache_ts` - Timestamp for caching current open orders, used to manage the cache validity.
    ///
    /// # Returns
    ///
    /// Returns a `Result` wrapping a new instance of the `OBClient` struct initialized with the provided parameters,
    /// or an `Error` if the initialization process fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     println!("Initialized OBClient: {:?}", ob_client);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Business Logic
    ///
    /// 1. Retrieve necessary env vars, such as the `RPC_URL` and `KEY_PATH` path.
    /// 2. Read the owner's keypair from the specified key path.
    /// 3. Initialize the RPC client with the given commitment configuration.
    /// 4. Fetch the market account information on chain.
    /// 5. Load the market state and extract base and quote mints.
    /// 6. Initialize the `Market` struct with fetched market information.
    /// 7. Fetche associated token accounts (ATA) for the base and quote tokens.
    /// 8. Initialize the open orders for the client.
    /// 9. Populate the open orders cache.
    /// 10. Load bids and asks information if the `load` parameter is set to `true`.
    ///
    pub async fn new(
        commitment: CommitmentConfig,
        market_id: Pubkey,
        load: bool,
        cache_ts: u128,
    ) -> Result<Self, Error> {
        let rpc_url =
            std::env::var("RPC_URL").unwrap_or("https://api.mainnet-beta.solana.com".to_string());
        let key_path = std::env::var("KEY_PATH").unwrap_or("".to_string());

        let owner = read_keypair(&key_path);
        let rpc_client = RpcClient::new_with_commitment(rpc_url, commitment);
        let oos_key_str = std::env::var("OOS_KEY").unwrap_or("".to_string());

        let orders_key = Pubkey::from_str(oos_key_str.as_str());

        let pub_owner_key = owner.pubkey().clone();

        let rpc_client = Rpc::new(rpc_client);

        let mut account_1 = rpc_client.inner().get_account(&market_id).await?;
        let mut account_2 = rpc_client.inner().get_account(&market_id).await?;
        let account_info_1;
        let account_info_2;
        let program_id = SRM_PROGRAM_ID.parse().unwrap();
        {
            account_info_1 = create_account_info_from_account(
                &mut account_1,
                &market_id,
                &program_id,
                false,
                false,
            );
            account_info_2 = create_account_info_from_account(
                &mut account_2,
                &market_id,
                &program_id,
                false,
                false,
            );
        }
        let market = MarketState::load(&account_info_1, &SRM_PROGRAM_ID.parse().unwrap(), false)?;
        let market_auth =
            MarketAuth::load(&account_info_2, &SRM_PROGRAM_ID.parse().unwrap(), false)?;
        let default_auth = Default::default();
        let events_authority = market_auth
            .consume_events_authority()
            .unwrap_or(&default_auth);

        let base_mint = Pubkey::from(u64_slice_to_pubkey(market.coin_mint));
        let quote_mint = Pubkey::from(u64_slice_to_pubkey(market.pc_mint));

        let market_info = Market::new(
            rpc_client.clone(),
            SRM_PROGRAM_ID.parse().unwrap(),
            market_id,
            base_mint,
            quote_mint,
            *events_authority,
            load,
        )
        .await?;

        let base_ata = get_associated_token_address(&pub_owner_key.clone(), &market_info.base_mint);
        let quote_ata =
            get_associated_token_address(&pub_owner_key.clone(), &market_info.quote_mint);

        let cloned_owner = owner.insecure_clone();
        let open_orders = OpenOrders::new(
            rpc_client.clone(),
            SRM_PROGRAM_ID.parse().unwrap(),
            cloned_owner,
            market_info.market_address,
        )
        .await?;
        let mut open_orders_cache = HashMap::new();

        let open_orders_cache_entry = OpenOrdersCacheEntry {
            open_orders: open_orders.clone(),
            ts: cache_ts,
        };

        open_orders_cache.insert(pub_owner_key, open_orders_cache_entry.clone());

        let mut ob_client = Self {
            rpc_client,
            market_info,
            owner: owner.into(),
            quote_ata,
            base_ata,
            open_orders,
            open_orders_cache,
        };

        if !orders_key.is_err() {
            ob_client.open_orders.oo_key = orders_key.unwrap();
        }

        if load {
            ob_client.load_bids_asks_info().await?;
        }

        if let Some(entry) = ob_client.open_orders_cache.get_mut(&pub_owner_key) {
            entry.open_orders = ob_client.open_orders.clone();
        }

        Ok(ob_client)
    }

    /// Loads information about bids, asks, and the open orders associated with the wallet from the market state.
    ///
    /// This function fetches and processes bids information, including extracting the bids and asks addresses
    /// and loading the open orders. It also determines the maximum bid price and minimum ask price.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the `OBClient` struct.
    ///
    /// # Returns
    ///
    /// A `Result` containing a tuple of `(bids_address, asks_address, open_orders)` if successful,
    /// or an error if loading the bids information fails.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with fetching accounts
    /// or processing the bids information.
    pub async fn load_bids_asks_info(&mut self) -> Result<(Pubkey, Pubkey, OpenOrders)> {
        let mut account = self
            .rpc_client
            .inner()
            .get_account(&self.market_info.market_address)
            .await?;
        let program_id_binding = self.market_info.program_id;
        let market_account_binding = self.market_info.market_address;
        let account_info;
        {
            account_info = create_account_info_from_account(
                &mut account,
                &market_account_binding,
                &program_id_binding,
                false,
                false,
            );
        }
        let market_state = MarketState::load(&account_info, &self.market_info.program_id, false)?;

        let bids_address = self.market_info.bids_address;
        let asks_address = self.market_info.asks_address;

        let mut bids_account = self.rpc_client.inner().get_account(&bids_address).await?;
        let bids_info = create_account_info_from_account(
            &mut bids_account,
            &bids_address,
            &self.market_info.program_id,
            false,
            false,
        );
        let mut bids = market_state.load_bids_mut(&bids_info)?;
        let (open_bids, open_bids_prices, max_bid) = self.process_bids(&mut bids)?;

        let mut asks_account = self.rpc_client.inner().get_account(&asks_address).await?;
        let asks_info = create_account_info_from_account(
            &mut asks_account,
            &asks_address,
            &self.market_info.program_id,
            false,
            false,
        );
        let mut asks = market_state.load_asks_mut(&asks_info)?;
        let (open_asks, open_asks_prices, min_ask) = self.process_asks(&mut asks)?;

        self.open_orders = OpenOrders {
            oo_key: self.open_orders.oo_key,
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

        Ok((bids_address, asks_address, self.open_orders.clone()))
    }

    /// Processes bids information to find the maximum bid price.
    ///
    /// This function removes bids from the provided `Slab` to find the maximum bid price.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `OBClient` struct.
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
                let bytes = u64_slice_to_pubkey(owner);
                let owner_address = Pubkey::from(bytes);

                let order_id = node.order_id();
                let price_raw = node.price().get();
                let ui_price = price_raw as f64 / 1e4;

                debug!("[*] Bid: {price_raw}");

                if max_bid == 0 {
                    max_bid = price_raw;
                }

                if owner_address == self.open_orders.oo_key {
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
    /// it finds all asks.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `OBClient` struct.
    /// * `asks` - A mutable reference to the `Slab` containing asks information.
    ///
    /// # Returns
    ///
    /// A `Result` containing a tuple of `(open_asks, open_asks_prices, min_ask)` if successful,
    /// or an error if processing asks fails.
    pub fn process_asks(&self, asks: &mut RefMut<Slab>) -> Result<(Vec<u128>, Vec<f64>, u64)> {
        let mut min_ask = 0;
        let mut open_asks = Vec::new();
        let mut open_asks_prices = Vec::new();
        loop {
            let node = asks.remove_min();
            match node {
                Some(node) => {
                    let owner = node.owner();
                    let bytes = u64_slice_to_pubkey(owner);
                    let owner_address = Pubkey::from(bytes);

                    let order_id = node.order_id();
                    let price_raw = node.price().get();
                    let ui_price = price_raw as f64 / 1e4;

                    debug!("[*] Ask: {price_raw}");

                    if min_ask == 0 {
                        min_ask = price_raw;
                    }

                    if owner_address == self.open_orders.oo_key {
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

    /// Loads the open bids from the market.
    ///
    /// # Returns
    ///
    /// The open bids stored in the market.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with loading the bids.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let result = ob_client.load_bids()?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn load_bids(&mut self) -> Result<Vec<u128>, ProgramError> {
        Ok(self.open_orders.open_bids.clone())
    }

    /// Loads the open asks from the market.
    ///
    /// # Returns
    ///
    /// The open asks stored in the market.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with loading the asks.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let result = ob_client.load_asks()?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn load_asks(&mut self) -> Result<Vec<u128>, ProgramError> {
        Ok(self.open_orders.open_asks.clone())
    }
    /// Places a limit order on the market.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `OBClient` struct.
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
    /// use openbook::matching::Side;
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let target_amount_quote = 5.0;
    ///     let side = Side::Bid; // or Side::Ask
    ///     let best_offset_usdc = 5.0;
    ///     let execute = true;
    ///     let target_price = 2.1;
    ///
    ///     if let Some(ord_ret_type) = ob_client.place_limit_order(
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
        let base_d_factor = 10u32.pow(self.market_info.coin_decimals as u32) as f64;
        let quote_d_factor = 10u32.pow(self.market_info.pc_decimals as u32) as f64;
        let base_lot_factor = self.market_info.coin_lot_size as f64;
        let quote_lot_factor = self.market_info.pc_lot_size as f64;

        let price_factor = quote_d_factor * base_lot_factor / base_d_factor / quote_lot_factor;

        let (input_ata, price) = match side {
            Side::Bid => {
                let mut price = self.open_orders.max_bid as f64 / price_factor - best_offset_usdc;
                if execute {
                    price = target_price;
                }

                (&self.quote_ata, price)
            }
            Side::Ask => {
                let mut price = self.open_orders.min_ask as f64 / price_factor + best_offset_usdc;
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
        let max_coin_qty = NonZeroU64::new(target_base_lots).unwrap();
        let max_native_pc_qty_including_fees = NonZeroU64::new(target_quote_lots_w_fee).unwrap();

        let place_order_ix = openbook_dex::instruction::new_order(
            &self.market_info.market_address,
            &self.open_orders.oo_key,
            &self.market_info.request_queue,
            &self.market_info.event_queue,
            &self.market_info.bids_address,
            &self.market_info.asks_address,
            input_ata,
            &self.owner.pubkey(),
            &self.market_info.coin_vault,
            &self.market_info.pc_vault,
            &SPL_TOKEN_ID.parse()?,
            &rent::ID,
            None,
            &self.market_info.program_id,
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

        let (_, signature) = self
            .rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), instructions)
            .await?;

        Ok(Some(OrderReturnType::Signature(signature)))
    }

    /// Cancels all limit orders in the market.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `OBClient` struct.
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
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     if let Some(ord_ret_type) = ob_client
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

        for oid in &self.open_orders.open_bids {
            let ix = openbook_dex::instruction::cancel_order(
                &self.market_info.program_id,
                &self.market_info.market_address,
                &self.market_info.bids_address,
                &self.market_info.asks_address,
                &self.open_orders.oo_key,
                &self.owner.pubkey(),
                &self.market_info.event_queue,
                Side::Bid,
                *oid,
            )?;
            ixs.push(ix);
        }

        for oid in &self.open_orders.open_asks {
            let ix = openbook_dex::instruction::cancel_order(
                &self.market_info.program_id,
                &self.market_info.market_address,
                &self.market_info.bids_address,
                &self.market_info.asks_address,
                &self.open_orders.oo_key,
                &self.owner.pubkey(),
                &self.market_info.event_queue,
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

        let (_, signature) = self
            .rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), ixs)
            .await?;

        Ok(Some(OrderReturnType::Signature(signature)))
    }

    /// Settles the balance for a user in the market.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `OBClient` struct.
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
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     if let Some(ord_ret_type) = ob_client
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
            &self.market_info.program_id,
            &self.market_info.market_address,
            &SPL_TOKEN_ID.parse()?,
            &self.open_orders.oo_key,
            &self.owner.pubkey(),
            &self.market_info.coin_vault,
            &self.base_ata,
            &self.market_info.pc_vault,
            &self.quote_ata,
            None,
            &self.market_info.vault_signer_key,
        )?;

        let instructions = vec![ix];

        if !execute {
            return Ok(Some(OrderReturnType::Instructions(instructions)));
        }

        let (_, signature) = self
            .rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), instructions)
            .await?;

        Ok(Some(OrderReturnType::Signature(signature)))
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
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let result = ob_client.match_orders_transaction(100).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn match_orders_transaction(&self, limit: u16) -> Result<(bool, Signature)> {
        let ix = openbook_dex::instruction::match_orders(
            &self.market_info.program_id,
            &self.market_info.market_address,
            &self.market_info.request_queue,
            &self.market_info.bids_address,
            &self.market_info.asks_address,
            &self.market_info.event_queue,
            &self.market_info.coin_vault,
            &self.market_info.pc_vault,
            limit,
        )
        .unwrap();

        let instructions = vec![ix];

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), instructions)
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
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let target_size_usdc_ask = 0.5;
    ///     let target_size_usdc_bid = 1.0;
    ///     let bid_price_jlp_usdc = 1.5;
    ///     let ask_price_jlp_usdc = 2.5;
    ///
    ///     let result = ob_client.cancel_settle_place(target_size_usdc_ask, target_size_usdc_bid, bid_price_jlp_usdc, ask_price_jlp_usdc).await?;
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
    ) -> Result<(bool, Signature)> {
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

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), instructions)
            .await
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
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let result = ob_client.cancel_settle_place_bid(1.5, 1.0).await?;
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
    ) -> Result<(bool, Signature)> {
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

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), instructions)
            .await
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
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let result = ob_client.cancel_settle_place_ask(1.5, 1.0).await?;
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
    ) -> Result<(bool, Signature)> {
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

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), instructions)
            .await
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
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let result = ob_client.cancel_settle().await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn cancel_settle(&mut self) -> Result<(bool, Signature)> {
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

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), instructions)
            .await
    }

    /// Consumes events from the market for specified open orders accounts.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `OBClient` struct.
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
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let open_orders_accounts = vec![ob_client.open_orders.oo_key];
    ///     let limit = 10;
    ///     let result = ob_client.consume_events_instruction(open_orders_accounts, limit).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn consume_events_instruction(
        &self,
        open_orders_accounts: Vec<Pubkey>,
        limit: u16,
    ) -> Result<(bool, Signature)> {
        let ix = openbook_dex::instruction::consume_events(
            &self.market_info.program_id,
            open_orders_accounts.iter().collect(),
            &self.market_info.market_address,
            &self.market_info.event_queue,
            &self.market_info.coin_vault,
            &self.market_info.pc_vault,
            limit,
        )
        .unwrap();

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    /// Consumes permissioned events from the market for specified open orders accounts.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `OBClient` struct.
    /// * `open_orders_accounts` - A vector of `Pubkey` representing the open orders accounts.
    /// * `limit` - The maximum number of events to consume.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Signature` of the transaction or a `Error` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust , ignore
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let open_orders_accounts = vec![ob_client.open_orders.oo_key];
    ///     let limit = 10;
    ///     let result = ob_client.consume_events_permissioned_instruction(open_orders_accounts, limit).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn consume_events_permissioned_instruction(
        &self,
        open_orders_accounts: Vec<Pubkey>,
        limit: u16,
    ) -> Result<(bool, Signature)> {
        let ix = openbook_dex::instruction::consume_events_permissioned(
            &self.market_info.program_id,
            open_orders_accounts.iter().collect(),
            &self.market_info.market_address,
            &self.market_info.event_queue,
            &self.market_info.events_authority,
            limit,
        )
        .unwrap();

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    /// Loads open orders accounts for the owner, filtering them based on bids and asks.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the `OBClient` struct.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `Account` representing open orders accounts or a boxed `Error` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let result = ob_client.load_orders_for_owner().await?;
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
        let _open_orders_accounts = self
            .find_open_orders_accounts_for_owner(self.open_orders.oo_key.clone(), 5000)
            .await?;

        Ok(bids)
    }

    /// Filters open orders accounts based on bids and asks.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `OBClient` struct.
    /// * `bids_address` - A `Pubkey` representing the bids address.
    /// * `asks_address` - A `Pubkey` representing the asks address.
    /// * `open_orders_accounts` - A vector of `OpenOrders` representing open orders accounts.
    ///
    /// # Returns
    ///
    /// A filtered vector of `OpenOrders` based on bids and asks addresses.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::v1::orders::OrderReturnType;
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let open_orders_accounts = vec![];
    ///     let result = ob_client.filter_for_open_orders(ob_client.market_info.bids_address, ob_client.market_info.asks_address, open_orders_accounts);
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn filter_for_open_orders(
        &self,
        bids_address: Pubkey,
        asks_address: Pubkey,
        open_orders_accounts: Vec<OpenOrders>,
    ) -> Vec<OpenOrders> {
        open_orders_accounts
            .into_iter()
            .filter(|open_orders| {
                open_orders.oo_key == bids_address || open_orders.oo_key == asks_address
            })
            .collect()
    }

    /// Finds open orders accounts for a specified owner and caches them based on the specified duration.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - A mutable reference to the `OBClient` struct.
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
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".parse()?;
    ///
    ///     let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    ///
    ///     let result = ob_client.find_open_orders_accounts_for_owner(ob_client.open_orders.oo_key, 5000).await?;
    ///
    ///     println!("{:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn find_open_orders_accounts_for_owner(
        &mut self,
        owner_address: Pubkey,
        cache_duration_ms: u128,
    ) -> Result<OpenOrders, Box<dyn std::error::Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();
        if let Some(cache_entry) = self.open_orders_cache.get(&owner_address) {
            if now - cache_entry.ts < cache_duration_ms as u128 {
                return Ok(cache_entry.open_orders.clone());
            }
        }

        self.load_bids_asks_info().await?;

        self.open_orders_cache.insert(
            owner_address,
            OpenOrdersCacheEntry {
                open_orders: self.open_orders.clone(),
                ts: now,
            },
        );

        Ok(self.open_orders.clone())
    }
}
