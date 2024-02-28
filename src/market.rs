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
use solana_sdk::bs58;
use solana_sdk::signature::Signature;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::cell::RefMut;
use std::fmt;
use std::fs;
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
    /// use solana_program::pubkey::Pubkey;
    /// use solana_sdk::signature::Keypair;
    /// use solana_rpc_client::rpc_client::RpcClient;
    /// use openbook_v1_sdk::market::Market;
    /// use openbook_v1_sdk::market::read_keypair;
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Assuming `market` is an initialized instance of the `Market` struct
    /// market.load().expect("Failed to load market information");
    /// ```
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

        let market_state = self.load_market_state(&account_info)?;
        let (bids_address, asks_address, max_bid) = self.load_bids_info(&market_state)?;

        drop(market_state);

        self.bids_address = bids_address.clone();
        self.asks_address = asks_address.clone();
        self.max_bid = max_bid.clone();

        Ok(())
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Assuming `market` is an initialized instance of the `Market` struct
    /// let market_state = market.load_market_state(&account_info).expect("Failed to load market state");
    /// let (bids_address, asks_address, max_bid) = market.load_bids_info(&market_state)
    ///     .expect("Failed to load bids information");
    /// ```
    pub fn load_bids_info(
        &self,
        market_state: &RefMut<MarketState>,
    ) -> anyhow::Result<(Pubkey, Pubkey, f64)> {
        let (bids_address, asks_address) = self.get_bids_asks_addresses(&market_state);

        let mut bids_account = self.rpc_client.0.get_account(&bids_address)?;
        let mut bids_info = self.create_account_info_from_account(
            &mut bids_account,
            &bids_address,
            &self.program_id,
            false,
            false,
        );
        let mut bids = market_state.load_bids_mut(&mut bids_info)?;

        let max_bid = self.process_bids(&mut bids)?;

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
    /// // Assuming `market` is an initialized instance of the `Market` struct
    /// let max_bid = market.process_bids(&mut bids).expect("Failed to process bids");
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
    /// // Assuming `market` is an initialized instance of the `Market` struct
    /// let (bids_address, asks_address) = market.get_bids_asks_addresses(&market_state);
    /// ```
    pub fn get_bids_asks_addresses(&self, market_state: &MarketState) -> (Pubkey, Pubkey) {
        let bids = market_state.bids;
        let asks = market_state.asks;
        let bids_bytes = self.u64_slice_to_bytes(&bids);
        let asks_bytes = self.u64_slice_to_bytes(&asks);

        let bids_address = Pubkey::new_from_array(bids_bytes);
        let asks_address = Pubkey::new_from_array(asks_bytes);

        (bids_address, asks_address)
    }

    /// Converts a slice of `u64` values into a fixed-size byte array.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
    /// * `slice` - A reference to a slice of `u64` values to be converted.
    ///
    /// # Returns
    ///
    /// A fixed-size array of bytes containing the serialized `u64` values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Assuming `market` is an initialized instance of the `Market` struct
    /// let slice = [1, 2, 3, 4];
    /// let bytes_array = market.u64_slice_to_bytes(&slice);
    /// ```
    pub fn u64_slice_to_bytes(&self, slice: &[u64]) -> [u8; 32] {
        let mut bytes_array: [u8; 32] = [0; 32];
        for i in 0..4 {
            bytes_array[i * 8..i * 8 + 8].copy_from_slice(&slice[i].to_le_bytes());
        }
        bytes_array
    }

    /// Loads the market state from the provided account information.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the `Market` struct.
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Assuming `market` is an initialized instance of the `Market` struct
    /// let market_state = market.load_market_state(&account_info).expect("Failed to load market state");
    /// ```
    pub fn load_market_state<'a>(
        &'a self,
        binding: &'a AccountInfo,
    ) -> anyhow::Result<RefMut<MarketState>> {
        let market_state = MarketState::load(binding, &self.program_id, false)?;
        Ok(market_state)
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
    /// // Assuming `market` is an initialized instance of the `Market` struct
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
            &my_program_id,
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
    /// // Assuming `market` is an initialized instance of the `Market` struct
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

        let mut instructions = Vec::new();
        instructions.push(place_order_ix);

        let recent_hash = self.rpc_client.0.get_latest_blockhash()?;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        let r = self.rpc_client.0.send_transaction_with_config(&txn, config);

        r
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
    /// // Assuming `market` is an initialized instance of the `Market` struct
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

        let mut instructions = Vec::new();
        instructions.push(ix);

        let recent_hash = self.rpc_client.0.get_latest_blockhash()?;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        let r = self.rpc_client.0.send_transaction_with_config(&txn, config);
        r
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
    /// // Assuming `market` is an initialized instance of the `Market` struct
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

        let mut instructions = Vec::new();
        instructions.push(ix);

        let recent_hash = self.rpc_client.0.get_latest_blockhash()?;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_hash,
        );

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        let r = self.rpc_client.0.send_transaction_with_config(&txn, config);

        r
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
    /// let tx = market.make_match_orders_transaction(100)?;
    /// ```
    pub fn make_match_orders_transaction(
        &self,
        limit: u16,
    ) -> anyhow::Result<Signature, ClientError> {
        let tx = Transaction::new_with_payer(&[], Some(&self.keypair.pubkey()));

        let match_orders_ix = openbook_dex::instruction::match_orders(
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

        let mut instructions = Vec::new();
        instructions.push(match_orders_ix);

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        let r = self.rpc_client.0.send_transaction_with_config(&tx, config);

        r
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
        Ok(self.load_orders(self.bids_address)?)
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
    /// let asks = market.load_asks()?;
    /// ```
    pub fn load_asks(&self) -> Result<Vec<u8>, ProgramError> {
        Ok(self.load_orders(self.asks_address)?)
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
    /// // Assuming `market` is an initialized instance of the `Market` struct
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

        let mut instructions = Vec::new();
        instructions.push(consume_events_ix);

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        let result = self.rpc_client.0.send_transaction_with_config(&tx, config);

        result
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
    ///
    /// let open_orders_accounts = vec![Pubkey::new_from_array([0; 32])];
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

        let mut instructions = Vec::new();
        instructions.push(consume_events_permissioned_ix);

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = true;
        let result = self.rpc_client.0.send_transaction_with_config(&tx, config);

        result
    }
}

/// Reads a keypair from a file.
///
/// # Arguments
///
/// * `path` - The file path containing the keypair information.
///
/// # Returns
///
/// A `Keypair` instance created from the keypair information in the file.
///
/// # Examples
///
/// ```rust
/// use openbook_v1_sdk::market::read_keypair;
///
/// let path = String::from("/path/to/keypair_file.json");
/// let keypair = read_keypair(&path);
/// ```
pub fn read_keypair(path: &String) -> Keypair {
    let secret_string: String = fs::read_to_string(path).expect("Can't find key file");
    let secret_bytes: Vec<u8> = match serde_json::from_str(&secret_string) {
        Ok(bytes) => bytes,
        Err(_) => match bs58::decode(&secret_string.trim()).into_vec() {
            Ok(bytes) => bytes,
            Err(_) => panic!("failed to load secret key from file"),
        },
    };
    let keypair =
        Keypair::from_bytes(&secret_bytes).expect("failed to generate keypair from secret bytes");
    keypair
}

/// Gets the current UNIX timestamp in seconds.
///
/// # Returns
///
/// The current UNIX timestamp in seconds.
///
/// # Examples
///
/// ```rust
/// use openbook_v1_sdk::market::get_unix_secs;
///
/// let timestamp = get_unix_secs();
/// ```
pub fn get_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
