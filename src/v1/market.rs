//! This module contains structs and functions related to the openbook market.
use crate::{
    rpc::Rpc,
    tokens_and_markets::{get_market_name, get_program_id, DexVersion, Token},
    traits::MarketInfo,
    utils::{create_account_info_from_account, u64_slice_to_pubkey},
};
use anyhow::{Error, Result};
use openbook_dex::state::{gen_vault_signer_key, MarketState};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    sysvar::slot_history::{AccountInfo, ProgramError},
};

use std::fmt::{Debug, Formatter};

/// Struct representing a market with associated state and information.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Market {
    /// The public key of the program associated with the market.
    pub program_id: Pubkey,

    /// The public key of the market.
    pub market_address: Pubkey,

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

    /// The public key of the event queue associated with the market.
    pub event_queue: Pubkey,

    /// The public key of the request queue associated with the market.
    pub request_queue: Pubkey,

    /// The public key of the bids associated with the market.
    pub bids_address: Pubkey,

    /// The public key of the asks associated with the market.
    pub asks_address: Pubkey,
}

impl Debug for Market {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(f, "Market {{")?;
        writeln!(f, "        program_id: {:?}", self.program_id)?;
        writeln!(f, "        market_address: {:?}", self.market_address)?;
        writeln!(f, "        coin_decimals: {:?}", self.coin_decimals)?;
        writeln!(f, "        pc_decimals: {:?}", self.pc_decimals)?;
        writeln!(f, "        coin_lot_size: {:?}", self.coin_lot_size)?;
        writeln!(f, "        account_flags: {:?}", self.account_flags)?;
        writeln!(f, "        pc_lot_size: {:?}", self.pc_lot_size)?;
        writeln!(f, "        quote_mint: {:?}", self.quote_mint)?;
        writeln!(f, "        base_mint: {:?}", self.base_mint)?;
        writeln!(f, "        coin_vault: {:?}", self.coin_vault)?;
        writeln!(f, "        pc_vault: {:?}", self.pc_vault)?;
        writeln!(f, "        vault_signer_key: {:?}", self.vault_signer_key)?;
        writeln!(f, "        event_queue: {:?}", self.event_queue)?;
        writeln!(f, "        request_queue: {:?}", self.request_queue)?;
        writeln!(f, "        bids_address: {:?}", self.bids_address)?;
        writeln!(f, "        asks_address: {:?}", self.asks_address)?;
        writeln!(f, "    }}")
    }
}

impl MarketInfo for Market {
    /// Initializes a new instance of the `Market` struct.
    ///
    /// This method creates a new `Market` instance with the provided parameters.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - RPC client for interacting with the Solana blockchain.
    ///                 This client allows the crate to communicate with the Solana network,
    ///                 enabling actions like fetching account information and sending transactions.
    /// * `program_version` - The version of the program representing the market.
    ///                       This indicates the specific version of the decentralized exchange (DEX) program
    ///                       running on the blockchain. Different versions may have different features or improvements.
    /// * `base_mint` - The symbol of the base mint.
    ///                 The base mint represents the primary currency traded in the market,
    ///                 such as USDC or JLP.
    /// * `quote_mint` - The symbol of the quote mint.
    ///                  The quote mint represents the secondary currency used for pricing in the market.
    ///                  For example, in a JLP/USDC market, USDC is the quote currency.
    /// * `load` - A boolean indicating whether to load market data immediately.
    ///            If set to `true`, the method will fetch and initialize the market's data from the blockchain.
    ///
    /// # Returns
    ///
    /// Returns a new instance of the `Market` struct on success.
    /// This instance contains all the necessary information about the specified market,
    /// including its address, associated mints, and other relevant details.
    ///
    /// # Errors
    ///
    /// This function returns an error if initialization fails for any reason,
    /// such as invalid input parameters or failure to fetch market data from the blockchain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::rpc_client::RpcClient;
    /// use openbook::market::Market;
    /// use crate::openbook::traits::MarketInfo;
    /// use openbook::rpc::Rpc;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///
    ///     let rpc_client = Rpc::new(RpcClient::new(rpc_url));
    ///
    ///     let market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, true).await?;
    ///
    ///     println!("Initialized Market: {:?}", market);
    ///
    ///     Ok(())
    /// }
    /// ```
    async fn new(
        rpc_client: Rpc,
        program_version: DexVersion,
        base_mint: Token,
        quote_mint: Token,
        load: bool,
    ) -> Result<Self, Error> {
        let (market_address, base_mint, quote_mint, program_id) =
            Self::parse_market_params(program_version, base_mint, quote_mint)?;

        let mut market = Self {
            program_id,
            market_address,
            coin_decimals: 9,
            pc_decimals: 6,
            coin_lot_size: 1_000_000,
            pc_lot_size: 1,
            quote_mint,
            base_mint,
            bids_address: Default::default(),
            asks_address: Default::default(),
            coin_vault: Default::default(),
            pc_vault: Default::default(),
            vault_signer_key: Default::default(),
            event_queue: Default::default(),
            request_queue: Default::default(),
            account_flags: 0,
        };

        if load {
            market.load(&rpc_client).await?;
        }

        market.init_vault_signer_key().await?;

        Ok(market)
    }

    /// Loads market information from the provided RPC client.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - RPC client for interacting with the Solana blockchain.
    ///                  This client allows fetching account information and submitting transactions.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success or an error if loading fails.
    ///
    /// # Errors
    ///
    /// This function returns an error if loading the market fails for any reason,
    /// such as the absence of the market account or permission issues.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::rpc_client::RpcClient;
    /// use openbook::market::Market;
    /// use crate::openbook::traits::MarketInfo;
    /// use openbook::rpc::Rpc;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///
    ///     let rpc_client = Rpc::new(RpcClient::new(rpc_url));
    ///
    ///     let mut market = Market::new(rpc_client.clone(), DexVersion::default(), Token::JLP, Token::USDC, true).await?;
    ///
    ///     market.load(&rpc_client.clone()).await?;
    ///
    ///     println!("Initialized Market: {:?}", market);
    ///
    ///     Ok(())
    /// }
    /// ```
    async fn load(&mut self, rpc_client: &Rpc) -> Result<(), Error> {
        let mut account = rpc_client.inner().get_account(&self.market_address).await?;
        let owner = account.owner;
        let program_id_binding = self.program_id;
        let market_account_binding = self.market_address;
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
        if self.program_id != owner {
            return Err(ProgramError::InvalidArgument.into());
        }

        self.load_market_state_info(&account_info).await?;

        Ok(())
    }

    /// Loads the market state information from the provided account information.
    ///
    /// # Arguments
    ///
    /// * `account_info` - A reference to the account information used to load the market state.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if loading the market state is successful.
    ///
    /// # Errors
    ///
    /// This function returns an error if loading the market state fails for any reason,
    /// such as invalid account data or parsing issues.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::rpc_client::RpcClient;
    /// use openbook::market::Market;
    /// use crate::openbook::traits::MarketInfo;
    /// use openbook::rpc::Rpc;
    /// use openbook::utils::create_account_info_from_account;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///
    ///     let rpc_client = Rpc::new(RpcClient::new(rpc_url));
    ///
    ///     let mut market = Market::new(rpc_client.clone(), DexVersion::default(), Token::JLP, Token::USDC, true).await?;
    ///
    ///     let mut account = rpc_client.clone().inner().get_account(&market.market_address).await?;
    ///     let program_id_binding = market.program_id;
    ///     let market_account_binding = market.market_address;
    ///
    ///     let account_info;
    ///     {
    ///         account_info = create_account_info_from_account(
    ///             &mut account,
    ///             &market_account_binding,
    ///             &program_id_binding,
    ///             false,
    ///             false,
    ///         );
    ///     }
    ///
    ///     market.load_market_state_info(&account_info).await?;
    ///
    ///     println!("Initialized Market: {:?}", market);
    ///
    ///     Ok(())
    /// }
    /// ```
    async fn load_market_state_info(
        &mut self,
        account_info: &AccountInfo<'_>,
    ) -> Result<(), Error> {
        let market_state = MarketState::load(account_info, &self.program_id, false)?;

        // Extract relevant information from the loaded market state.
        let coin_vault_array: [u8; 32] = u64_slice_to_pubkey(market_state.coin_vault);
        let pc_vault_array: [u8; 32] = u64_slice_to_pubkey(market_state.pc_vault);
        let request_queue_array: [u8; 32] = u64_slice_to_pubkey(market_state.req_q);
        let event_queue_array: [u8; 32] = u64_slice_to_pubkey(market_state.event_q);
        let own_address_array: [u8; 32] = u64_slice_to_pubkey(market_state.own_address);
        let bids_array: [u8; 32] = u64_slice_to_pubkey(market_state.bids);
        let asks_array: [u8; 32] = u64_slice_to_pubkey(market_state.asks);

        self.coin_vault = Pubkey::new_from_array(coin_vault_array);
        self.pc_vault = Pubkey::new_from_array(pc_vault_array);
        self.request_queue = Pubkey::new_from_array(request_queue_array);
        self.event_queue = Pubkey::new_from_array(event_queue_array);
        self.bids_address = Pubkey::new_from_array(bids_array);
        self.asks_address = Pubkey::new_from_array(asks_array);

        let own_address = Pubkey::new_from_array(own_address_array);
        assert_eq!(self.market_address, own_address);

        self.account_flags = market_state.account_flags;
        self.coin_lot_size = market_state.coin_lot_size;
        self.pc_lot_size = market_state.pc_lot_size;

        Ok(())
    }

    /// Parses market parameters.
    ///
    /// # Arguments
    ///
    /// * `program_version` - Program dex version representing the market.
    /// * `base_mint` - Base mint symbol.
    /// * `quote_mint` - Quote mint symbol.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing the parsed market parameters:
    /// `(market_address, base_mint, quote_mint, program_id)`.
    ///
    /// # Errors
    ///
    /// This function returns an error if parsing the market parameters fails,
    /// such as invalid mint symbols or program versions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::market::Market;
    /// use crate::openbook::traits::MarketInfo;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///
    ///     let (market_address, base_mint, quote_mint, program_id) =
    ///         Market::parse_market_params(DexVersion::default(), Token::JLP, Token::USDC)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    fn parse_market_params(
        program_version: DexVersion,
        base_mint: Token,
        quote_mint: Token,
    ) -> Result<(Pubkey, Pubkey, Pubkey, Pubkey), Error> {
        // Parse market parameters and obtain relevant Pubkey values.
        let market_address = get_market_name(base_mint).0.parse()?;
        let quote_mint = get_market_name(quote_mint).1.parse()?;
        let base_mint = get_market_name(base_mint).1.parse()?;
        let program_id = get_program_id(program_version).parse()?;
        Ok((market_address, base_mint, quote_mint, program_id))
    }

    /// Initializes the vault signer key.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initialization is successful, or an error otherwise.
    ///
    /// # Errors
    ///
    /// This function returns an error if generating the vault signer key fails,
    /// such as reaching the maximum number of attempts or encountering cryptographic errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::rpc_client::RpcClient;
    /// use openbook::market::Market;
    /// use crate::openbook::traits::MarketInfo;
    /// use openbook::rpc::Rpc;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    ///
    ///     let rpc_client = Rpc::new(RpcClient::new(rpc_url));
    ///
    ///     let mut market = Market::new(rpc_client, DexVersion::default(), Token::JLP, Token::USDC, true).await?;
    ///
    ///     market.init_vault_signer_key().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    async fn init_vault_signer_key(&mut self) -> Result<(), Error> {
        for i in 0..100 {
            if let Ok(pk) = gen_vault_signer_key(i, &self.market_address, &self.program_id) {
                self.vault_signer_key = pk;
                return Ok(());
            }
        }
        Ok(())
    }
}
