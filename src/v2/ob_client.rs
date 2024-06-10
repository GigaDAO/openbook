use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    str::FromStr,
    sync::Arc,
};

use anchor_lang::{prelude::System, Id};
use anchor_spl::{associated_token::AssociatedToken, token::Token};
use anyhow::{Context, Error, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use fixed::types::I80F48;
use rand::random;
use serde::{Deserialize, Serialize};
use spl_associated_token_account::get_associated_token_address;

use openbookdex_v2::{
    state::{
        BookSide, Market, OpenOrdersAccount, OracleConfig, OracleConfigParams, PlaceOrderType,
        SelfTradeBehavior, Side,
    },
    PlaceMultipleOrdersArgs, PlaceOrderArgs, PlaceOrderPeggedArgs,
};

use solana_sdk::{
    clock::Slot,
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};

use crate::{
    rpc::Rpc,
    rpc_client::RpcClient,
    utils::{get_unix_secs, read_keypair},
    v2::{
        account_fetcher::{
            account_fetcher_fetch_openorders_account, AccountFetcherTrait, CachedAccountFetcher,
            RpcAccountFetcher,
        },
        context::MarketContext,
        market::{CreateMarketArgs, MarketInfo},
    },
};

#[derive(Clone, BorshDeserialize, BorshSerialize)]
pub struct OpenOrderNode {
    pub is_buy: bool,
    pub price: f64,
    pub amount: f64,
    pub order_id: u64,
    pub timestamp: u64,
    pub slot: u8,
}

impl Debug for OpenOrderNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(f, "OpenOrders {{")?;
        writeln!(f, "        order_id: {:?}", self.order_id)?;
        writeln!(f, "        is_buy: {:?}", self.is_buy)?;
        writeln!(f, "        price: {:?}", self.price)?;
        writeln!(f, "        amount: {:?}", self.amount)?;
        writeln!(f, "        timestamp: {:?}", self.timestamp)?;
        writeln!(f, "        slot: {:?}", self.slot)?;
        writeln!(f, "}}")
    }
}

#[derive(Clone, Default, BorshDeserialize, BorshSerialize)]
pub struct OpenOrderState {
    pub asks_base_in_oos: f64,
    pub bids_base_in_oos: f64,
    pub base_free_in_oos: f64,
    pub quote_free_in_oos: f64,
}

impl Debug for OpenOrderState {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(f, "OpenOrderState {{")?;
        writeln!(f, "    asks base in oos: {:?}", self.asks_base_in_oos)?;
        writeln!(f, "    bids base in oos: {:?}", self.bids_base_in_oos)?;
        writeln!(f, "    base free in oos: {:?}", self.base_free_in_oos)?;
        writeln!(f, "    quote free in oos: {:?}", self.quote_free_in_oos)?;
        writeln!(f, "}}")
    }
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct BestQuotes {
    pub highest_bid: f64,
    pub lowest_ask: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PricePoint {
    pub id: String,
    pub mint_symbol: String,
    pub vs_token: String,
    pub vs_token_symbol: String,
    pub price: f64,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PriceData {
    pub data: HashMap<String, PricePoint>,
}

#[derive(Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct AtaBalances {
    pub quote_balance: f64,
    pub base_balance: f64,
    pub total_balance: f64,
    pub price: f64,
}

impl Debug for AtaBalances {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(f, "AtaBalances {{")?;
        writeln!(f, "    quote balance: {:?}", self.quote_balance)?;
        writeln!(f, "    base balance: {:?}", self.base_balance)?;
        writeln!(f, "    total balance: {:?}", self.total_balance)?;
        writeln!(f, "    price: {:?}", self.price)?;
        writeln!(f, "}}")
    }
}

/// OpenBook v2 Client to interact with the OpenBook market and perform actions.
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

    /// The public key of the index account.
    pub index_account: Pubkey,

    /// The public key of the market ID.
    pub market_id: Pubkey,

    /// The account fetcher used to retrieve account data.
    pub account_fetcher: Arc<dyn AccountFetcherTrait>,

    /// Account info of the wallet on the market (e.g., open orders).
    pub open_orders_account: Pubkey,

    /// A list of open orders.
    pub open_orders: Vec<OpenOrderNode>,

    /// Information about the OpenBook market.
    pub market_info: MarketInfo,

    /// Context information for the market.
    pub context: MarketContext,

    /// Open Orders Account State.
    pub oo_state: OpenOrderState,

    /// Associated Token Accounts Balances, aka equity.
    pub ata_balances: AtaBalances,
}

impl Debug for OBClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(f, "OB_V2_Client {{")?;
        writeln!(f, "    owner: {:?}", self.owner.pubkey())?;
        writeln!(f, "    rpc_client: {:?}", self.rpc_client)?;
        writeln!(f, "    quote_ata: {:?}", self.quote_ata)?;
        writeln!(f, "    base_ata: {:?}", self.base_ata)?;
        writeln!(f, "    open_orders: {:?}", self.open_orders)?;
        writeln!(f, "    market_info: {:?}", self.market_info)?;
        writeln!(f, "    market_id: {:?}", self.market_id)?;
        writeln!(f, "    index_account: {:?}", self.index_account)?;
        writeln!(f, "    open_orders_account: {:?}", self.open_orders_account)?;
        writeln!(f, "    oo_state: {:?}", self.oo_state)?;
        writeln!(f, "    ata_balances: {:?}", self.ata_balances)?;
        writeln!(f, "}}")
    }
}

impl OBClient {
    /// Initializes a new instance of the `OBClient` struct, representing an OpenBook V2 program client.
    ///
    /// This method initializes the `OBClient` struct, containing information about the requested market id,
    /// It fetches and stores all data about this OpenBook market. Additionally, it includes information about
    /// the account associated with the wallet on the OpenBook market (e.g., open orders, bids, asks, etc.).
    ///
    /// # Arguments
    ///
    /// * `commitment` - Commitment configuration for transactions, determining the level of finality required.
    /// * `market_id` - Public key (ID) of the market to fetch information about.
    /// * `new` - Boolean indicating whether to create new open orders and index accounts.
    /// * `load` - Boolean indicating whether to load market data immediately after initialization.
    ///
    /// # Returns
    ///
    /// Returns a `Result` wrapping a new instance of the `OBClient` struct initialized with the provided parameters,
    /// or an `Error` if the initialization process fails.
    ///
    /// # Example
    ///
    /// ```rust , ignore
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v2::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "gQN1TNHiqj5x82ZQd7JZ8rm8WD4xwWtXxd4onReWZNK".parse()?;
    ///
    ///     let ob_client = OBClient::new(commitment, market_id, false, true).await?;
    ///
    ///     println!("Initialized OBClient: {:?}", ob_client);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Business Logic
    ///
    /// 1. Retrieve necessary environment variables, such as the `RPC_URL`, `KEY_PATH`, open orders key `OOS_KEY`, and index key `INDEX_KEY`.
    /// 2. Read the owner's keypair from the specified key path.
    /// 3. Initialize the RPC client with the given commitment configuration.
    /// 4. Fetch the market information from the Solana blockchain.
    /// 5. Generate associated token accounts (ATA) for the base and quote tokens.
    /// 6. Initialize the context with market information.
    /// 7. Initialize the account fetcher for fetching account data.
    /// 8. Populate the initial fields of the `OBClient` struct.
    /// 9. Load open orders and bids/asks information if the `load` parameter is set to `true`.
    /// 10. Create new open orders and index accounts if the `new` parameter is set to `true`.
    ///
    pub async fn new(
        commitment: CommitmentConfig,
        market_id: Pubkey,
        new: bool,
        load: bool,
    ) -> Result<Self, Error> {
        let rpc_url =
            std::env::var("RPC_URL").unwrap_or("https://api.mainnet-beta.solana.com".to_string());
        let key_path = std::env::var("KEY_PATH").unwrap_or("".to_string());

        let owner = read_keypair(&key_path);
        let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), commitment);
        let oos_key_str = std::env::var("OOS_KEY").unwrap_or("".to_string());
        let index_key_str = std::env::var("INDEX_KEY").unwrap_or("".to_string());

        let orders_key = Pubkey::from_str(oos_key_str.as_str());
        let index_key = Pubkey::from_str(index_key_str.as_str());

        let pub_owner_key = owner.pubkey().clone();

        let rpc = Rpc::new(rpc_client);

        let market = rpc.fetch_anchor_account::<Market>(&market_id).await?;

        let base_ata = get_associated_token_address(&pub_owner_key.clone(), &market.base_mint);
        let quote_ata = get_associated_token_address(&pub_owner_key.clone(), &market.quote_mint);

        let index_account = Default::default();
        let open_orders_account = Default::default();
        let oo_state = Default::default();
        let ata_balances = Default::default();

        let context = MarketContext {
            market: market,
            address: market_id,
        };
        let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), commitment);

        let account_fetcher = Arc::new(CachedAccountFetcher::new(Arc::new(RpcAccountFetcher {
            rpc: rpc_client,
        })));

        let oracle_config = OracleConfig {
            conf_filter: market.oracle_config.conf_filter,
            max_staleness_slots: market.oracle_config.max_staleness_slots,
            reserved: market.oracle_config.reserved,
        };

        let market_info = MarketInfo {
            name: market.name().to_string(),
            base_decimals: market.base_decimals,
            quote_decimals: market.quote_decimals,
            market_authority: market.market_authority,
            collect_fee_admin: market.collect_fee_admin,
            open_orders_admin: market.open_orders_admin,
            consume_events_admin: market.consume_events_admin,
            close_market_admin: market.close_market_admin,
            bids: market.bids,
            asks: market.asks,
            event_heap: market.event_heap,
            oracle_a: market.oracle_a,
            oracle_b: market.oracle_b,
            oracle_config: oracle_config,
            quote_lot_size: market.quote_lot_size,
            base_lot_size: market.base_lot_size,
            seq_num: market.seq_num,
            registration_time: market.registration_time,
            maker_fee: market.maker_fee,
            taker_fee: market.taker_fee,
            fees_accrued: market.fees_accrued,
            fees_to_referrers: market.fees_to_referrers,
            referrer_rebates_accrued: market.referrer_rebates_accrued,
            fees_available: market.fees_available,
            maker_volume: market.maker_volume,
            taker_volume_wo_oo: market.taker_volume_wo_oo,
            base_mint: market.base_mint,
            quote_mint: market.quote_mint,
            market_base_vault: market.market_base_vault,
            base_deposit_total: market.base_deposit_total,
            market_quote_vault: market.market_quote_vault,
            quote_deposit_total: market.quote_deposit_total,
        };

        let mut ob_client = Self {
            rpc_client: rpc,
            market_info,
            owner: owner.into(),
            quote_ata,
            base_ata,
            index_account,
            account_fetcher,
            market_id,
            open_orders_account,
            context,
            open_orders: vec![],
            oo_state,
            ata_balances,
        };

        if new {
            ob_client.index_account = ob_client.create_open_orders_indexer(true).await?.3;
            ob_client.open_orders_account = ob_client.find_or_create_account().await?;
        }

        if load {
            if !orders_key.is_err() {
                ob_client.open_orders_account = orders_key.unwrap();
            } else {
                ob_client.open_orders_account = ob_client.find_or_create_account().await?;
            }
            if !index_key.is_err() {
                ob_client.index_account = index_key.unwrap();
            } else {
                ob_client.index_account = ob_client.create_open_orders_indexer(false).await?.3;
            }
            let (open_orders, _best_quotes) = ob_client.load_bids_asks_info().await?;
            ob_client.open_orders = open_orders;
            ob_client.oo_state = ob_client.load_oo_state().await?;
            ob_client.ata_balances = ob_client.get_base_quote_total().await?;
        }

        Ok(ob_client)
    }

    /// # Example
    ///
    /// ```rust , ignore
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v2::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "gQN1TNHiqj5x82ZQd7JZ8rm8WD4xwWtXxd4onReWZNK".parse()?;
    ///
    ///     let ob_client = OBClient::new(commitment, market_id, false, true).await?;
    ///
    ///     let (confirmed, sig) = ob_client.settle_funds().await?;
    ///
    ///     println!("Got Signature: {:?}", sig);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn settle_funds(&self) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbookdex_v2::accounts::SettleFunds {
                        owner: self.owner(),
                        penalty_payer: self.owner(),
                        open_orders_account: self.open_orders_account,
                        market: self.market_id,
                        market_authority: self.market_info.market_authority,
                        user_base_account: self.base_ata,
                        user_quote_account: self.quote_ata,
                        market_base_vault: self.market_info.market_base_vault,
                        market_quote_vault: self.market_info.market_quote_vault,
                        referrer_account: None,
                        system_program: System::id(),
                        token_program: Token::id(),
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbookdex_v2::instruction::SettleFunds {}),
        };
        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    /// # Example
    ///
    /// ```rust , ignore
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v2::ob_client::OBClient;
    /// use openbook::v2_state::Side;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "gQN1TNHiqj5x82ZQd7JZ8rm8WD4xwWtXxd4onReWZNK".parse()?;
    ///
    ///     let ob_client = OBClient::new(commitment, market_id, false, true).await?;
    ///
    ///     let (confirmed, sig, order_id, slot) = ob_client.place_limit_order(165.2, 1000, Side::Bid).await?;
    ///
    ///     println!("Got Order ID: {:?}", order_id);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn place_limit_order(
        &mut self,
        limit_price: f64,
        quote_size: u64,
        side: Side,
    ) -> Result<(bool, Signature, u64, Slot)> {
        let current_time = get_unix_secs();
        let price_lots = self.native_price_to_lots_price(limit_price);
        let max_quote_lots = self
            .context
            .max_quote_lots_including_maker_fees_from_usd(quote_size);
        let base_size = self.get_base_size_from_quote(quote_size, limit_price);
        let max_base_lots = self.context.max_base_lots_from_usd(base_size);
        let ata = self.get_ata_by_side(side);
        let vault = self.market_info.get_vault_by_side(side);

        tracing::debug!("base: {max_base_lots}, quote: {max_quote_lots}");
        let oid = self.gen_order_id();

        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbookdex_v2::accounts::PlaceOrder {
                        open_orders_account: self.open_orders_account,
                        open_orders_admin: None,
                        signer: self.owner(),
                        market: self.market_id,
                        bids: self.market_info.bids,
                        asks: self.market_info.asks,
                        event_heap: self.market_info.event_heap,
                        oracle_a: self.market_info.oracle_a.into(),
                        oracle_b: self.market_info.oracle_b.into(),
                        user_token_account: ata,
                        market_vault: vault,
                        token_program: Token::id(),
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbookdex_v2::instruction::PlaceOrder {
                args: PlaceOrderArgs {
                    side,
                    price_lots,
                    max_base_lots: max_base_lots as i64,
                    max_quote_lots_including_fees: max_quote_lots as i64,
                    client_order_id: oid,
                    order_type: PlaceOrderType::PostOnly,
                    expiry_timestamp: current_time + 86_400,
                    self_trade_behavior: SelfTradeBehavior::AbortTransaction,
                    limit: 12,
                },
            }),
        };

        let (confirmed, sig) = self
            .rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await?;

        // get slot
        let max_slot: Slot = self
            .account_fetcher
            .transaction_max_slot(sig)
            .await?
            .unwrap_or(0);

        Ok((confirmed, sig, oid, max_slot))
    }

    pub async fn place_market_order(
        &mut self,
        limit_price: f64,
        quote_size: u64,
        side: Side,
    ) -> Result<(bool, Signature)> {
        let current_time = get_unix_secs();
        let price_lots = self.native_price_to_lots_price(limit_price);
        let max_quote_lots = self
            .context
            .max_quote_lots_including_maker_fees_from_usd(quote_size);
        let base_size = self.get_base_size_from_quote(quote_size, limit_price);
        let max_base_lots = self.context.max_base_lots_from_usd(base_size);
        let ata = self.get_ata_by_side(side);
        let vault = self.market_info.get_vault_by_side(side);

        tracing::debug!("base: {max_base_lots}, quote: {max_quote_lots}");
        let oid = self.gen_order_id();

        // TODO: update to market order inst
        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbookdex_v2::accounts::PlaceOrder {
                        open_orders_account: self.open_orders_account,
                        open_orders_admin: None,
                        signer: self.owner(),
                        market: self.market_id,
                        bids: self.market_info.bids,
                        asks: self.market_info.asks,
                        event_heap: self.market_info.event_heap,
                        oracle_a: self.market_info.oracle_a.into(),
                        oracle_b: self.market_info.oracle_b.into(),
                        user_token_account: ata,
                        market_vault: vault,
                        token_program: Token::id(),
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbookdex_v2::instruction::PlaceOrder {
                args: PlaceOrderArgs {
                    side,
                    price_lots,
                    max_base_lots: max_base_lots as i64,
                    max_quote_lots_including_fees: max_quote_lots as i64,
                    client_order_id: oid,
                    order_type: PlaceOrderType::PostOnly,
                    expiry_timestamp: current_time + 86_400,
                    self_trade_behavior: SelfTradeBehavior::AbortTransaction,
                    limit: 12,
                },
            }),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    /// # Example
    ///
    /// ```rust , ignore
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v2::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "gQN1TNHiqj5x82ZQd7JZ8rm8WD4xwWtXxd4onReWZNK".parse()?;
    ///
    ///     let ob_client = OBClient::new(commitment, market_id, false, true).await?;
    ///
    ///     let (confirmed, sig) = ob_client.cancel_limit_order(12345678123578).await?;
    ///
    ///     println!("Got Sig: {:?}", sig);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn cancel_limit_order(&self, order_id: u128) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbookdex_v2::accounts::CancelOrder {
                        open_orders_account: self.open_orders_account,
                        signer: self.owner(),
                        market: self.market_id,
                        bids: self.market_info.bids,
                        asks: self.market_info.asks,
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbookdex_v2::instruction::CancelOrder {
                order_id,
            }),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    /// # Example
    ///
    /// ```rust , ignore
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v2::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "gQN1TNHiqj5x82ZQd7JZ8rm8WD4xwWtXxd4onReWZNK".parse()?;
    ///
    ///     let ob_client = OBClient::new(commitment, market_id, false, true).await?;
    ///
    ///     let (confirmed, sig) = ob_client.cancel_all().await?;
    ///
    ///     println!("Got Sig: {:?}", sig);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn cancel_all(&self) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbookdex_v2::accounts::CancelOrder {
                        open_orders_account: self.open_orders_account,
                        signer: self.owner(),
                        market: self.market_id,
                        bids: self.market_info.bids,
                        asks: self.market_info.asks,
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbookdex_v2::instruction::CancelAllOrders {
                side_option: None,
                limit: 255,
            }),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    pub async fn cancel_all_and_place_orders(
        &self,
        bids: Vec<PlaceMultipleOrdersArgs>,
        asks: Vec<PlaceMultipleOrdersArgs>,
    ) -> Result<(bool, Signature)> {
        let orders_type = PlaceOrderType::PostOnly;

        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbookdex_v2::accounts::CancelAllAndPlaceOrders {
                        open_orders_account: self.open_orders_account,
                        signer: self.owner(),
                        open_orders_admin: self.market_info.open_orders_admin.into(),
                        user_quote_account: self.quote_ata,
                        user_base_account: self.base_ata,
                        market: self.market_id,
                        bids: self.market_info.bids,
                        asks: self.market_info.asks,
                        event_heap: self.market_info.event_heap,
                        market_quote_vault: self.market_info.market_quote_vault,
                        market_base_vault: self.market_info.market_base_vault,
                        oracle_a: self.market_info.oracle_a.into(),
                        oracle_b: self.market_info.oracle_b.into(),
                        token_program: Token::id(),
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(
                &openbookdex_v2::instruction::CancelAllAndPlaceOrders {
                    orders_type,
                    bids,
                    asks,
                    limit: 255,
                },
            ),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    /// # Example
    ///
    /// ```rust , ignore
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v2::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "gQN1TNHiqj5x82ZQd7JZ8rm8WD4xwWtXxd4onReWZNK".parse()?;
    ///
    ///     let ob_client = OBClient::new(commitment, market_id, false, true).await?;
    ///
    ///     let account = ob_client.find_or_create_account().await?;
    ///
    ///     println!("Got Account: {:?}", account);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn find_or_create_account(&self) -> Result<Pubkey> {
        let program = openbookdex_v2::id();

        let openbook_account_name = "random";

        let mut openbook_account_tuples = self
            .rpc_client
            .fetch_openbook_accounts(program, self.owner())
            .await?;
        let openbook_account_opt = openbook_account_tuples
            .iter()
            .find(|(_, account)| account.name() == openbook_account_name);
        if openbook_account_opt.is_none() {
            openbook_account_tuples
                .sort_by(|a, b| a.1.account_num.partial_cmp(&b.1.account_num).unwrap());
            let account_num = match openbook_account_tuples.last() {
                Some(tuple) => tuple.1.account_num + 1,
                None => 0u32,
            };
            self.create_open_orders_account(account_num, openbook_account_name)
                .await
                .context("Failed to create account...")?;
        }
        let openbook_account_tuples = self
            .rpc_client
            .fetch_openbook_accounts(program, self.owner())
            .await?;

        let index = openbook_account_tuples
            .iter()
            .position(|tuple| tuple.1.name() == openbook_account_name)
            .unwrap_or(0);

        Ok(openbook_account_tuples[index].0)
    }

    /// # Example
    ///
    /// ```rust , ignore
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v2::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "gQN1TNHiqj5x82ZQd7JZ8rm8WD4xwWtXxd4onReWZNK".parse()?;
    ///
    ///     let ob_client = OBClient::new(commitment, market_id, false, true).await?;
    ///
    ///     let indexer = ob_client.create_open_orders_indexer().await?.3;
    ///
    ///     println!("Got Indexer Account: {:?}", indexer);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_open_orders_indexer(
        &self,
        execute: bool,
    ) -> Result<(bool, Instruction, Signature, Pubkey)> {
        let owner = &self.owner;
        let payer = &self.owner;

        let open_orders_indexer = Pubkey::find_program_address(
            &[b"OpenOrdersIndexer".as_ref(), owner.pubkey().as_ref()],
            &openbookdex_v2::id(),
        )
        .0;

        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &openbookdex_v2::accounts::CreateOpenOrdersIndexer {
                    owner: owner.pubkey(),
                    open_orders_indexer,
                    payer: payer.pubkey(),
                    system_program: System::id(),
                },
                None,
            ),
            data: anchor_lang::InstructionData::data(
                &openbookdex_v2::instruction::CreateOpenOrdersIndexer {},
            ),
        };

        let mut sig = Signature::default();
        let mut confirmed = false;
        if execute {
            (confirmed, sig) = self
                .rpc_client
                .send_and_confirm((*self.owner).insecure_clone(), vec![ix.clone()])
                .await?;
        }

        Ok((confirmed, ix, sig, open_orders_indexer))
    }

    /// # Example
    ///
    /// ```rust , ignore
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v2::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "gQN1TNHiqj5x82ZQd7JZ8rm8WD4xwWtXxd4onReWZNK".parse()?;
    ///
    ///     let ob_client = OBClient::new(commitment, market_id, false, true).await?;
    ///
    ///     let (confirmed, sig, account) = ob_client.create_open_orders_account(2, "Sol-USDC-OO-Account").await?;
    ///
    ///     println!("Got New OO Account: {:?}", account);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_open_orders_account(
        &self,
        account_num: u32,
        name: &str,
    ) -> Result<(bool, Signature, Pubkey)> {
        let owner = &self.owner;
        let payer = &self.owner;
        let market = self.market_id;

        let delegate = None;

        let open_orders_indexer = Pubkey::find_program_address(
            &[b"OpenOrdersIndexer".as_ref(), owner.pubkey().as_ref()],
            &openbookdex_v2::id(),
        )
        .0;

        let account = Pubkey::find_program_address(
            &[
                b"OpenOrders".as_ref(),
                owner.pubkey().as_ref(),
                &account_num.to_le_bytes(),
            ],
            &openbookdex_v2::id(),
        )
        .0;

        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &openbookdex_v2::accounts::CreateOpenOrdersAccount {
                    owner: owner.pubkey(),
                    open_orders_indexer,
                    open_orders_account: account,
                    payer: payer.pubkey(),
                    delegate_account: delegate,
                    market,
                    system_program: System::id(),
                },
                None,
            ),
            data: anchor_lang::InstructionData::data(
                &openbookdex_v2::instruction::CreateOpenOrdersAccount {
                    name: name.to_string(),
                },
            ),
        };

        let (confirmed, sig) = self
            .rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await?;

        Ok((confirmed, sig, account))
    }

    pub fn owner(&self) -> Pubkey {
        self.owner.pubkey()
    }

    pub async fn openorders_account(&self) -> Result<OpenOrdersAccount> {
        account_fetcher_fetch_openorders_account(&*self.account_fetcher, &self.open_orders_account)
            .await
    }

    pub async fn create_market(
        &self,
        market_args: CreateMarketArgs,
    ) -> Result<(bool, Signature, Pubkey)> {
        let program_id = openbookdex_v2::id();

        let market = Keypair::new().pubkey();

        let oracle_config = OracleConfigParams {
            conf_filter: 0.069,
            max_staleness_slots: Some(69),
        };

        let event_authority_slice = &[b"__event_authority".as_ref()];

        let (event_authority, _bump_seed) =
            Pubkey::find_program_address(event_authority_slice, &program_id);

        let market_seeds = &[b"Market".as_ref(), &market.to_bytes()];

        let (market_authority, _bump_seed) =
            Pubkey::find_program_address(market_seeds, &program_id);

        let market_base_vault =
            get_associated_token_address(&market_authority, &market_args.base_mint);
        let market_quote_vault =
            get_associated_token_address(&market_authority, &market_args.quote_mint);

        let event_heap = Keypair::new().pubkey();
        let asks = Keypair::new().pubkey();
        let bids = Keypair::new().pubkey();

        let ix = Instruction {
            program_id: program_id,
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbookdex_v2::accounts::CreateMarket {
                        market,
                        market_authority,
                        bids,
                        asks,
                        event_heap,
                        payer: self.owner(),
                        market_base_vault,
                        market_quote_vault,
                        base_mint: market_args.base_mint,
                        quote_mint: market_args.quote_mint,
                        system_program: solana_sdk::system_program::id(),
                        oracle_a: market_args.oracle_a,
                        oracle_b: market_args.oracle_b,
                        collect_fee_admin: market_args.collect_fee_admin,
                        open_orders_admin: market_args.open_orders_admin,
                        consume_events_admin: market_args.consume_events_admin,
                        close_market_admin: market_args.close_market_admin,
                        event_authority,
                        program: openbookdex_v2::id(),
                        token_program: Token::id(),
                        associated_token_program: AssociatedToken::id(),
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbookdex_v2::instruction::CreateMarket {
                name: market_args.name,
                oracle_config,
                base_lot_size: market_args.base_lot_size,
                quote_lot_size: market_args.quote_lot_size,
                maker_fee: market_args.maker_fee,
                taker_fee: market_args.taker_fee,
                time_expiry: market_args.time_expiry,
            }),
        };

        let (confirmed, sig) = self
            .rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await?;

        Ok((confirmed, sig, market))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn place_order_pegged(
        &self,
        market: Market,
        market_address: Pubkey,
        side: Side,
        price_offset_lots: i64,
        peg_limit: i64,
        max_base_lots: i64,
        max_quote_lots_including_fees: i64,
        client_order_id: u64,
        order_type: PlaceOrderType,
        expiry_timestamp: u64,
        limit: u8,
        user_token_account: Pubkey,
        market_vault: Pubkey,
        self_trade_behavior: SelfTradeBehavior,
    ) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbookdex_v2::accounts::PlaceOrder {
                        open_orders_account: self.open_orders_account,
                        open_orders_admin: None,
                        signer: self.owner(),
                        market: market_address,
                        bids: market.bids,
                        asks: market.asks,
                        event_heap: market.event_heap,
                        oracle_a: market.oracle_a.into(),
                        oracle_b: market.oracle_b.into(),
                        user_token_account,
                        market_vault,
                        token_program: Token::id(),
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbookdex_v2::instruction::PlaceOrderPegged {
                args: PlaceOrderPeggedArgs {
                    side,
                    price_offset_lots,
                    peg_limit,
                    max_base_lots,
                    max_quote_lots_including_fees,
                    client_order_id,
                    order_type,
                    expiry_timestamp,
                    self_trade_behavior,
                    limit,
                },
            }),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit(
        &self,
        market_address: Pubkey,
        base_amount: u64,
        quote_amount: u64,
        user_base_account: Pubkey,
        user_quote_account: Pubkey,
        market_base_vault: Pubkey,
        market_quote_vault: Pubkey,
    ) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbookdex_v2::accounts::Deposit {
                        open_orders_account: self.open_orders_account,
                        owner: self.owner(),
                        market: market_address,
                        user_base_account,
                        user_quote_account,
                        market_base_vault,
                        market_quote_vault,
                        token_program: Token::id(),
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbookdex_v2::instruction::Deposit {
                base_amount,
                quote_amount,
            }),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    /// # Example
    ///
    /// ```rust , ignore
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v2::ob_client::OBClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let market_id = "gQN1TNHiqj5x82ZQd7JZ8rm8WD4xwWtXxd4onReWZNK".parse()?;
    ///
    ///     let ob_client = OBClient::new(commitment, market_id, false, true).await?;
    ///
    ///     let (confirmed, sig) = ob_client.consume_events(255).await?;
    ///
    ///     println!("Tx Signature: {:?}", sig);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn consume_events(&self, limit: usize) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbookdex_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbookdex_v2::accounts::ConsumeEvents {
                        consume_events_admin: self.market_info.consume_events_admin.into(),
                        market: self.market_id,
                        event_heap: self.market_info.event_heap,
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbookdex_v2::instruction::ConsumeEvents {
                limit,
            }),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    pub fn native_price_to_lots_price(&self, limit_price: f64) -> i64 {
        let base_decimals = self.market_info.base_decimals as u32;
        let quote_decimals = self.market_info.quote_decimals as u32;
        let base_factor = 10_u64.pow(base_decimals);
        let quote_factor = 10_u64.pow(quote_decimals);
        let price_factor = (base_factor / quote_factor) as f64;
        let price_lots = (limit_price * price_factor) as i64;
        price_lots
    }

    pub fn get_base_size_from_quote(&self, quote_size: u64, limit_price: f64) -> u64 {
        let base_decimals = self.market_info.base_decimals as u32;
        let base_factor = 10_u64.pow(base_decimals) as f64;
        let base_size = ((quote_size as f64 / limit_price) * base_factor) as u64;
        base_size
    }

    pub async fn load_bids_asks_info(&self) -> Result<(Vec<OpenOrderNode>, BestQuotes)> {
        let mut best_quotes = BestQuotes {
            highest_bid: 0.,
            lowest_ask: 0.,
        };
        let mut open_orders = Vec::new();
        let _current_time = get_unix_secs();
        let orders_key = self.open_orders_account;

        let bids_book_side = self
            .rpc_client
            .fetch_anchor_account::<BookSide>(&self.market_info.bids)
            .await?;

        for (i, bid_book_side) in bids_book_side.iter_valid(0, None).enumerate() {
            let node = bid_book_side.node;
            let slot = node.owner_slot;
            let timestamp = node.timestamp;
            let owner_address = node.owner;
            let order_id = node.client_order_id;
            let lot_price = bid_book_side.price_lots;
            let native_price = self.market_info.lot_to_native_price(lot_price);
            let ui_price: f64 = I80F48::to_num::<f64>(native_price) * 1000.;
            let ui_amount = node.quantity as f64 / 1e1;
            if i == 0 {
                best_quotes.highest_bid = ui_amount;
            }
            if owner_address == orders_key {
                open_orders.push(OpenOrderNode {
                    is_buy: true,
                    price: ui_price,
                    amount: ui_amount,
                    order_id: order_id,
                    timestamp: timestamp,
                    slot: slot,
                });
            }
        }

        let asks_book_side = self
            .rpc_client
            .fetch_anchor_account::<BookSide>(&self.market_info.asks)
            .await
            .unwrap();

        for (i, ask_book_side) in asks_book_side.iter_valid(0, None).enumerate() {
            let node = ask_book_side.node;
            let slot = node.owner_slot;
            let timestamp = node.timestamp;
            let owner_address = node.owner;
            let order_id = node.client_order_id;
            let lot_price = ask_book_side.price_lots;
            let native_price = self.market_info.lot_to_native_price(lot_price);
            let ui_price: f64 = I80F48::to_num::<f64>(native_price) * 1000.;
            if i == 0 {
                best_quotes.lowest_ask = ui_price;
            }
            let ui_amount = node.quantity as f64 / 1e1;
            if owner_address == orders_key {
                open_orders.push(OpenOrderNode {
                    is_buy: false,
                    price: ui_price,
                    amount: ui_amount,
                    order_id: order_id,
                    timestamp: timestamp,
                    slot: slot,
                });
            }
        }

        Ok((open_orders, best_quotes))
    }

    pub async fn load_oo_state(&self) -> Result<OpenOrderState> {
        let open_orders_account = self.openorders_account().await?;

        let asks_base_lots = open_orders_account.position.asks_base_lots;
        let base_decimals = self.market_info.base_decimals;
        let base_decimals_factor = 10_i64.pow(base_decimals as u32) as f64;
        let base_lots_factor = self.market_info.base_lot_size as f64;
        let lots_2_native_factor = base_lots_factor / base_decimals_factor;
        let asks_base_ui_amount = asks_base_lots as f64 * lots_2_native_factor;

        let bids_quote_lots = open_orders_account.position.bids_quote_lots;
        let quote_decimals = self.market_info.quote_decimals;
        let quote_decimals_factor = 10_i64.pow(quote_decimals as u32) as f64;
        let quote_lots_factor = self.market_info.quote_lot_size as f64;
        let lots_2_native_factor = quote_lots_factor / quote_decimals_factor;
        let bids_base_ui_amount = bids_quote_lots as f64 * lots_2_native_factor;

        let base_free_native = open_orders_account.position.base_free_native;
        let base_free_ui = base_free_native as f64 / base_decimals_factor;

        let quote_free_native = open_orders_account.position.quote_free_native;
        let quote_free_ui = quote_free_native as f64 / quote_decimals_factor;

        Ok(OpenOrderState {
            asks_base_in_oos: asks_base_ui_amount,
            bids_base_in_oos: bids_base_ui_amount,
            base_free_in_oos: base_free_ui,
            quote_free_in_oos: quote_free_ui,
        })
    }

    pub async fn get_token_balance(&self, ata: &Pubkey) -> Result<f64> {
        let r = self
            .rpc_client
            .inner()
            .get_token_account_balance(&ata)
            .await?;
        Ok(r.ui_amount.unwrap())
    }

    pub async fn get_base_quote_total(&self) -> Result<AtaBalances> {
        let base_ata = self.base_ata.clone();
        let quote_ata = self.quote_ata.clone();
        let base_balance = self.get_token_balance(&base_ata).await?;
        let quote_balance = self.get_token_balance(&quote_ata).await?;
        let price = get_base_price(&self.market_info.quote_mint.to_string()).await?;

        Ok(AtaBalances {
            quote_balance,
            base_balance,
            total_balance: quote_balance + (price * base_balance),
            price,
        })
    }

    pub fn get_ata_by_side(&self, side: Side) -> Pubkey {
        match side {
            Side::Bid => self.quote_ata,
            Side::Ask => self.base_ata,
        }
    }

    pub fn gen_order_id(&self) -> u64 {
        random::<u64>()
    }
}

pub async fn get_base_price(quote_mint: &str) -> Result<f64> {
    let base_url = "https://price.jup.ag/v4/price?ids=";
    let url = format!("{base_url}{quote_mint}");
    let result = reqwest::get(url).await?;
    let prices = result.json::<PriceData>().await?;
    let base_quote_price = prices.data.get(quote_mint).unwrap().price;
    Ok(base_quote_price)
}
