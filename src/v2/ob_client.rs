use std::str::FromStr;
use std::sync::Arc;

use anchor_lang::prelude::System;
use anchor_lang::Id;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::Token;

use openbook_v2::state::OracleConfigParams;
use openbook_v2::{
    state::{Market, OpenOrdersAccount, PlaceOrderType, SelfTradeBehavior, Side},
    PlaceMultipleOrdersArgs, PlaceOrderArgs, PlaceOrderPeggedArgs,
};

use crate::v2::account_fetcher::*;
use crate::v2::context::MarketContext;
use spl_associated_token_account::get_associated_token_address;

use anyhow::{Error, Result};
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signer::Signer};

use crate::rpc::Rpc;
use rand::random;
use solana_sdk::clock::Slot;

use crate::rpc_client::RpcClient;
use crate::utils::get_unix_secs;
use crate::utils::read_keypair;
use anyhow::Context;
use fixed::types::I80F48;
use openbook_v2::state::BookSide;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenOrderNode {
    pub is_buy: bool,
    pub price: f64,
    pub amount: f64,
    pub order_id: u64,
    pub timestamp: u64,
    pub slot: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenOrderState {
    pub base_in_oos: f64,
    pub quote_in_oos: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PriceData {
    pub data: HashMap<String, PricePoint>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AtaBalances {
    pub quote_balance: f64,
    pub base_balance: f64,
    pub total_balance: f64,
    pub price: f64,
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
    pub index_account: Pubkey,
    pub market_id: Pubkey,
    pub account_fetcher: Arc<dyn AccountFetcherTrait>,

    /// Account info of the wallet on the market (e.g., open orders).
    pub open_orders_account: Pubkey,
    /// open orders.
    pub open_orders: Vec<OpenOrderNode>,
    /// Information about the OpenBook market.
    pub market_info: Market,
    pub context: MarketContext,
}

impl OBClient {
    /// Initializes a new instance of the `OBClient` struct, representing an OpenBook client.
    ///
    /// This method initializes the `OBClient` struct, containing information about the requested market,
    /// having the base and quote mints. It fetches and stores all data about this OpenBook market.
    /// Additionally, it includes information about the account associated with the wallet on the OpenBook market
    /// (e.g., open orders, bids, asks, etc.).
    ///
    /// # Arguments
    ///
    /// * `commitment` - Commitment configuration for transactions.
    /// * `program_version` - Program dex version representing the market.
    /// * `base_mint` - Base mint symbol.
    /// * `quote_mint` - Quote mint symbol.
    /// * `load` - Boolean indicating whether to load market data immediately.
    /// * `cache_ts` - Timestamp for caching current open orders.
    ///
    /// # Returns
    ///
    /// Returns a new instance of the `OBClient` struct initialized with the provided parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::commitment_config::CommitmentConfig;
    /// use openbook::v1::ob_client::OBClient;
    /// use openbook::tokens_and_markets::{DexVersion, Token};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let commitment = CommitmentConfig::confirmed();
    ///
    ///     let mut ob_client = OBClient::new(commitment, DexVersion::default(), Token::JLP, Token::USDC, true, 1000).await?;
    ///
    ///     println!("Initialized OBClient: {:?}", ob_client);
    ///
    ///     Ok(())
    /// }
    /// ```
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

        let market_info = rpc
            .fetch_anchor_account::<Market>(&market_id)
            .await
            .unwrap();

        let base_ata = get_associated_token_address(&pub_owner_key.clone(), &market_info.base_mint);
        let quote_ata =
            get_associated_token_address(&pub_owner_key.clone(), &market_info.quote_mint);

        let index_account = Default::default();
        let open_orders_account = Default::default();

        let context = MarketContext {
            market: market_info,
            address: market_id,
        };
        let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), commitment);

        let account_fetcher = Arc::new(CachedAccountFetcher::new(Arc::new(RpcAccountFetcher {
            rpc: rpc_client,
        })));

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
        };

        if !orders_key.is_err() {
            ob_client.open_orders_account = orders_key.unwrap();
        }

        if !index_key.is_err() {
            ob_client.index_account = index_key.unwrap();
        }

        if load {
            let (open_orders, _best_quotes) = ob_client.load_bids_asks_info().await?;
            ob_client.open_orders = open_orders;
        }

        if new {
            ob_client.index_account = ob_client.create_open_orders_indexer().await?.2;
            ob_client.open_orders_account = ob_client.find_or_create_account().await?;
        }

        Ok(ob_client)
    }

    pub async fn settle_funds(&self) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::SettleFunds {
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
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::SettleFunds {}),
        };
        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    pub async fn place_market_order(
        &mut self,
        limit_price: f64,
        quote_size_usd: u64,
        side: Side,
    ) -> Result<(bool, Signature)> {
        let current_time = get_unix_secs();
        let price_lots = self.native_price_to_lots_price(limit_price);
        let max_quote_lots = self
            .context
            .max_quote_lots_including_maker_fees_from_usd(quote_size_usd);
        let base_size = self.get_base_size_from_quote(quote_size_usd, limit_price);
        let max_base_lots = self.context.max_base_lots_from_usd(base_size);
        let ata = match side {
            Side::Bid => self.quote_ata.clone(),
            Side::Ask => self.base_ata.clone(),
        };
        let vault = self.market_info.get_vault_by_side(side);

        tracing::debug!("base: {max_base_lots}, quote: {max_quote_lots}");
        let oid = random::<u64>();

        // TODO: update to market order inst
        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::PlaceOrder {
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
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::PlaceOrder {
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

    pub async fn place_limit_order(
        &mut self,
        limit_price: f64,
        quote_size_usd: u64,
        side: Side,
    ) -> Result<(bool, Signature, u64, Slot)> {
        let current_time = get_unix_secs();
        let price_lots = self.native_price_to_lots_price(limit_price);
        let max_quote_lots = self
            .context
            .max_quote_lots_including_maker_fees_from_usd(quote_size_usd);
        let base_size = self.get_base_size_from_quote(quote_size_usd, limit_price);
        let max_base_lots = self.context.max_base_lots_from_usd(base_size);
        let ata = match side {
            Side::Bid => self.quote_ata.clone(),
            Side::Ask => self.base_ata.clone(),
        };
        let vault = self.market_info.get_vault_by_side(side);

        tracing::debug!("base: {max_base_lots}, quote: {max_quote_lots}");
        let oid = random::<u64>();

        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::PlaceOrder {
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
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::PlaceOrder {
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

    pub async fn cancel_limit_order(&self, order_id: u128) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::CancelOrder {
                        open_orders_account: self.open_orders_account,
                        signer: self.owner(),
                        market: self.market_id,
                        bids: self.market_info.bids,
                        asks: self.market_info.asks,
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::CancelOrder {
                order_id,
            }),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    pub async fn cancel_all(&self) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::CancelOrder {
                        open_orders_account: self.open_orders_account,
                        signer: self.owner(),
                        market: self.market_id,
                        bids: self.market_info.bids,
                        asks: self.market_info.asks,
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::CancelAllOrders {
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
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::CancelAllAndPlaceOrders {
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
                &openbook_v2::instruction::CancelAllAndPlaceOrders {
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

    pub async fn find_or_create_account(&self) -> Result<Pubkey> {
        let program = openbook_v2::id();

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
            .unwrap();

        Ok(openbook_account_tuples[index].0)
    }

    pub async fn create_open_orders_indexer(&self) -> Result<(bool, Signature, Pubkey)> {
        let owner = &self.owner;
        let payer = &self.owner;

        let open_orders_indexer = Pubkey::find_program_address(
            &[b"OpenOrdersIndexer".as_ref(), owner.pubkey().as_ref()],
            &openbook_v2::id(),
        )
        .0;

        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &openbook_v2::accounts::CreateOpenOrdersIndexer {
                    owner: owner.pubkey(),
                    open_orders_indexer,
                    payer: payer.pubkey(),
                    system_program: System::id(),
                },
                None,
            ),
            data: anchor_lang::InstructionData::data(
                &openbook_v2::instruction::CreateOpenOrdersIndexer {},
            ),
        };

        let (confirmed, sig) = self
            .rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await?;

        Ok((confirmed, sig, open_orders_indexer))
    }

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
            &openbook_v2::id(),
        )
        .0;

        let account = Pubkey::find_program_address(
            &[
                b"OpenOrders".as_ref(),
                owner.pubkey().as_ref(),
                &account_num.to_le_bytes(),
            ],
            &openbook_v2::id(),
        )
        .0;

        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: anchor_lang::ToAccountMetas::to_account_metas(
                &openbook_v2::accounts::CreateOpenOrdersAccount {
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
                &openbook_v2::instruction::CreateOpenOrdersAccount {
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

    #[allow(clippy::too_many_arguments)]
    pub async fn create_market(
        &self,
        market: Pubkey,
        market_authority: Pubkey,
        bids: Pubkey,
        asks: Pubkey,
        event_heap: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
        oracle_a: Option<Pubkey>,
        oracle_b: Option<Pubkey>,
        collect_fee_admin: Pubkey,
        open_orders_admin: Option<Pubkey>,
        consume_events_admin: Option<Pubkey>,
        close_market_admin: Option<Pubkey>,
        event_authority: Pubkey,
        name: String,
        oracle_config: OracleConfigParams,
        base_lot_size: i64,
        quote_lot_size: i64,
        maker_fee: i64,
        taker_fee: i64,
        time_expiry: i64,
    ) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::CreateMarket {
                        market,
                        market_authority,
                        bids,
                        asks,
                        event_heap,
                        payer: self.owner(),
                        market_base_vault: self.base_ata,
                        market_quote_vault: self.quote_ata,
                        base_mint,
                        quote_mint,
                        system_program: solana_sdk::system_program::id(),
                        oracle_a,
                        oracle_b,
                        collect_fee_admin,
                        open_orders_admin,
                        consume_events_admin,
                        close_market_admin,
                        event_authority,
                        program: openbook_v2::id(),
                        token_program: Token::id(),
                        associated_token_program: AssociatedToken::id(),
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::CreateMarket {
                name,
                oracle_config,
                base_lot_size,
                quote_lot_size,
                maker_fee,
                taker_fee,
                time_expiry,
            }),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
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
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::PlaceOrder {
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
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::PlaceOrderPegged {
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
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::Deposit {
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
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::Deposit {
                base_amount,
                quote_amount,
            }),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn consume_events(
        &self,
        market: Market,
        market_address: Pubkey,
        limit: usize,
    ) -> Result<(bool, Signature)> {
        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::ConsumeEvents {
                        consume_events_admin: market.consume_events_admin.into(),
                        market: market_address,
                        event_heap: market.event_heap,
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::ConsumeEvents {
                limit,
            }),
        };

        self.rpc_client
            .send_and_confirm((*self.owner).insecure_clone(), vec![ix])
            .await
    }

    pub async fn get_equity(&self) -> Result<(f64, f64)> {
        let ba = &self.base_ata;
        let qa = &self.quote_ata;

        let bb = self
            .rpc_client
            .inner()
            .get_token_account_balance(ba)
            .await?
            .ui_amount
            .unwrap();
        let qb = self
            .rpc_client
            .inner()
            .get_token_account_balance(qa)
            .await?
            .ui_amount
            .unwrap();

        Ok((bb, qb))
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

    pub fn get_base_size_from_quote(&self, quote_size_usd: u64, limit_price: f64) -> u64 {
        let base_decimals = self.market_info.base_decimals as u32;
        let base_factor = 10_u64.pow(base_decimals) as f64;
        let base_size = ((quote_size_usd as f64 / limit_price) * base_factor) as u64;
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
        let result = self.load_oo_balances().await?;
        println!("got (base, quote) bals: {:?}", result);

        Ok(OpenOrderState {
            base_in_oos: result.0,
            quote_in_oos: result.1,
        })
    }

    pub async fn load_oo_balances(&self) -> Result<(f64, f64)> {
        let open_orders_account = self.openorders_account().await?;
        let asks_native_price = self
            .market_info
            .lot_to_native_price(open_orders_account.position.asks_base_lots);
        let asks_base_total: f64 = I80F48::to_num::<f64>(asks_native_price) * 1000.;

        let bids_native_price = self
            .market_info
            .lot_to_native_price(open_orders_account.position.bids_base_lots);
        let bids_base_total: f64 = I80F48::to_num::<f64>(bids_native_price) * 1000.;

        Ok((bids_base_total, asks_base_total))
    }

    pub async fn load_open_orders(&self) -> Result<Vec<OpenOrderNode>> {
        let open_orders_account = self.openorders_account().await?;
        let mut open_orders = Vec::new();
        for order in open_orders_account.all_orders_in_use() {
            let native_price = self.market_info.lot_to_native_price(order.locked_price);
            let ui_price: f64 = I80F48::to_num::<f64>(native_price) * 1000.;
            open_orders.push(OpenOrderNode {
                is_buy: false, // Order object doesn't contain is_buy
                price: ui_price,
                amount: 0.0, // Order object doesn't contain amount
                order_id: order.client_id,
                timestamp: 0, // Order object doesn't contain timestamp
                slot: 0,      // Order object doesn't contain slot
            });
        }

        Ok(open_orders)
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
}

pub async fn get_base_price(quote_mint: &str) -> Result<f64> {
    let base_url = "https://price.jup.ag/v4/price?ids=";
    let url = format!("{base_url}{quote_mint}");
    let result = reqwest::get(url).await?;
    let prices = result.json::<PriceData>().await?;
    let base_quote_price = prices.data.get(quote_mint).unwrap().price;
    Ok(base_quote_price)
}
