use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anchor_client::Cluster;

use anchor_lang::prelude::System;
use anchor_lang::{AccountDeserialize, Id};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::Token;

use itertools::Itertools;

use openbook_v2::state::OracleConfigParams;
use openbook_v2::{
    state::{Market, OpenOrdersAccount, PlaceOrderType, SelfTradeBehavior, Side},
    PlaceMultipleOrdersArgs, PlaceOrderArgs, PlaceOrderPeggedArgs,
};

use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::address_lookup_table_account::AddressLookupTableAccount;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::hash::Hash;
use solana_sdk::signer::keypair;
use solana_sdk::transaction::TransactionError;

use crate::v2::account_fetcher::*;
use crate::v2::context::MarketContext;
use crate::v2::gpa::{fetch_anchor_account, fetch_openbook_accounts};

use anyhow::Context;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signer::Signer};

use crate::rpc::Rpc;
use rand::random;
use solana_sdk::clock::Slot;

use crate::rpc_client::RpcClient;
use crate::utils::get_unix_secs;
use fixed::types::I80F48;
use openbook_v2::state::BookSide;
use serde::{Deserialize, Serialize};
use solana_sdk::account::Account;
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
    pub jlp_in_oos: f64,
    pub usdc_in_oos: f64,
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
    pub usdc_balance: f64,
    pub sol_balance: f64,
    pub total_balance: f64,
    pub price: f64,
}

/// OpenBook v2 Client to interact with the OpenBook market and perform actions.
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
    /// Account info of the wallet on the market (e.g., open orders).
    pub open_orders: OpenOrderNode,
    /// Information about the OpenBook market.
    pub market_info: Market,
    pub context: MarketContext,
}

impl Clone for OBClient {
    fn clone(&self) -> Self {
        OBClient {
            keypair: Keypair::from_bytes(&self.keypair.to_bytes())
                .expect("Failed to clone Keypair"),
            index_account: self.index_account.clone(),
            open_orders_account: self.open_orders_account.clone(),
            market_id: self.market_id.clone(),
            client: self.client.clone(),
            ob_client: self.clone(),
            market: self.market.clone(),
            base_ata: self.base_ata.clone(),
            quote_ata: self.quote_ata.clone(),
            context: self.context.clone(),
        }
    }
}

impl OBClient {
    pub async fn new() -> Self {
        let keypair = load_env_keypair("SECRET");
        let index_account = load_env_pubkey("INDEX_ACCOUNT");
        let open_orders_account = load_env_pubkey("SOL_USDC_OO_ACCOUNT");
        let market_id = load_env_pubkey("SOL_USDC_MARKET_ID");
        let base_ata = load_env_pubkey("BASE_ATA");
        let quote_ata = load_env_pubkey("QUOTE_ATA");
        let rpc_url = load_env_url("RPC_URL");
        let client = RpcClient::new();
        let market = client
            .rpc_anchor_account::<Market>(&market_id)
            .await
            .unwrap();
        let ob_client = OBClient::new_for_existing_account(
            client.clone(),
            open_orders_account,
            Arc::new(keypair.insecure_clone()),
        )
        .await
        .unwrap();
        let context = MarketContext {
            market,
            address: market_id,
        };
        Self {
            keypair: keypair.insecure_clone(),
            index_account,
            open_orders_account,
            client,
            ob_client,
            market_id,
            market,
            base_ata,
            quote_ata,
            context,
        }
    }
    pub async fn settle_balance(&self) -> anyhow::Result<Signature> {
        let r = self
            .settle_funds(
                self.market,
                self.market_id,
                self.base_ata,
                self.quote_ata,
                self.market.market_base_vault,
                self.market.market_quote_vault,
                None,
            )
            .await;
        r
    }
    pub async fn place_market_order(
        &mut self,
        limit_price: f64,
        quote_size_usd: u64,
        side: Side,
    ) -> anyhow::Result<Signature> {
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
        let vault = self.market.get_vault_by_side(side);

        tracing::debug!("base: {max_base_lots}, quote: {max_quote_lots}");
        let oid = random::<u64>();
        let r = self
            .place_order(
                self.market,
                self.market_id,
                side,
                price_lots,
                max_base_lots as i64,
                max_quote_lots as i64,
                oid,
                PlaceOrderType::PostOnly,
                current_time + 86_400,
                12,
                ata,
                vault,
                SelfTradeBehavior::AbortTransaction,
            )
            .await;
        r
    }
    pub async fn place_limit_order(
        &mut self,
        limit_price: f64,
        quote_size_usd: u64,
        side: Side,
    ) -> anyhow::Result<(u64, Signature, Slot)> {
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
        let vault = self.market.get_vault_by_side(side);

        tracing::debug!("base: {max_base_lots}, quote: {max_quote_lots}");
        let oid = random::<u64>();
        let sig = self
            .place_order(
                self.market,
                self.market_id,
                side,
                price_lots,
                max_base_lots as i64,
                max_quote_lots as i64,
                oid,
                PlaceOrderType::PostOnly,
                current_time + 86_400,
                12,
                ata,
                vault,
                SelfTradeBehavior::AbortTransaction,
            )
            .await?;
        // get slot
        let max_slot: Slot = self
            .account_fetcher
            .transaction_max_slot(sig)
            .await?
            .unwrap_or(0);
        Ok((oid, sig, max_slot))
    }
    pub async fn cancel_limit_order(&self, order_id: u128) -> anyhow::Result<Signature> {
        self.cancel_order(self.market, self.market_id, order_id)
            .await
    }
    pub async fn cancel_all(&self) -> anyhow::Result<Signature> {
        let r = self
            .cancel_all_orders(self.market, self.market_id, None, 255)
            .await;
        r
    }
    pub async fn cancel_all_and_place_orders(
        &self,
        bids: Vec<PlaceMultipleOrdersArgs>,
        asks: Vec<PlaceMultipleOrdersArgs>,
    ) -> anyhow::Result<Signature> {
        let orders_type = PlaceOrderType::PostOnly;

        self.cancel_all_and_place_orders(
            self.market,
            self.market_id,
            self.base_ata.clone(),
            self.quote_ata.clone(),
            orders_type,
            bids,
            asks,
            255,
        )
        .await
    }

    pub async fn find_accounts(
        &self,
        owner: &Keypair,
    ) -> anyhow::Result<Vec<(Pubkey, OpenOrdersAccount)>> {
        fetch_openbook_accounts(&self.rpc_client, openbook_v2::ID, owner.pubkey()).await
    }

    pub async fn find_or_create_account(
        &self,
        owner: &Keypair,
        payer: &Keypair, // pays the SOL for the new account
        market: Pubkey,
        openbook_account_name: &str,
    ) -> anyhow::Result<Pubkey> {
        let program = openbook_v2::ID;

        let mut openbook_account_tuples =
            fetch_openbook_accounts(&self.rpc_client, program, owner.pubkey()).await?;
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
            Self::create_open_orders_account(
                self.rpc_client,
                market,
                owner,
                payer,
                None,
                account_num,
                openbook_account_name,
            )
            .await
            .context("Failed to create account...")?;
        }
        let openbook_account_tuples =
            fetch_openbook_accounts(&self.rpc_client, program, owner.pubkey()).await?;
        let index = openbook_account_tuples
            .iter()
            .position(|tuple| tuple.1.name() == openbook_account_name)
            .unwrap();
        Ok(openbook_account_tuples[index].0)
    }

    pub async fn create_open_orders_indexer(
        &self,
        owner: &Keypair,
        payer: &Keypair, // pays the SOL for the new account
    ) -> anyhow::Result<(Pubkey, Signature)> {
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

        let txsig = TransactionBuilder {
            instructions: vec![ix],
            address_lookup_tables: vec![],
            payer: payer.pubkey(),
            signers: vec![owner, payer],
            config: self.rpc_client.transaction_builder_config,
        }
        .send_and_confirm(self.rpc_client)
        .await?;

        Ok((open_orders_indexer, txsig))
    }

    pub async fn create_open_orders_account(
        &self,
        market: Pubkey,
        owner: &Keypair,
        payer: &Keypair, // pays the SOL for the new account
        delegate: Option<Pubkey>,
        account_num: u32,
        name: &str,
    ) -> anyhow::Result<(Pubkey, Signature)> {
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

        let txsig = TransactionBuilder {
            instructions: vec![ix],
            address_lookup_tables: vec![],
            payer: payer.pubkey(),
            signers: vec![owner, payer],
            config: self.rpc_client.transaction_builder_config,
        }
        .send_and_confirm(self.rpc_client)
        .await?;

        Ok((account, txsig))
    }

    /// Conveniently creates a RPC based client
    pub async fn new_for_existing_account(
        &self,
        account: Pubkey,
        owner: Arc<Keypair>,
    ) -> anyhow::Result<Self> {
        let account_fetcher = Arc::new(CachedAccountFetcher::new(Arc::new(RpcAccountFetcher {
            rpc: self.rpc_client,
        })));
        let openbook_account =
            account_fetcher_fetch_openorders_account(&*account_fetcher, &account).await?;
        if openbook_account.owner != owner.pubkey() {
            anyhow::bail!(
                "bad owner for account: expected {} got {}",
                openbook_account.owner,
                owner.pubkey()
            );
        }

        Self::new_detail(self.rpc_client, account, owner, account_fetcher)
    }

    /// Allows control of AccountFetcher and externally created MangoGroupContext
    pub fn new_detail(
        client: Client,
        account: Pubkey,
        owner: Arc<Keypair>,
        // future: maybe pass Arc<MangoGroupContext>, so it can be extenally updated?
        account_fetcher: Arc<dyn AccountFetcherTrait>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            client,
            account_fetcher,
            owner,
            open_orders_account: account,
            http_client: reqwest::Client::new(),
        })
    }

    pub fn owner(&self) -> Pubkey {
        self.owner.pubkey()
    }

    pub async fn openorders_account(&self) -> anyhow::Result<OpenOrdersAccount> {
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
    ) -> anyhow::Result<Signature> {
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
                        market_base_vault:
                            spl_associated_token_account::get_associated_token_address(
                                &market_authority,
                                &base_mint,
                            ),
                        market_quote_vault:
                            spl_associated_token_account::get_associated_token_address(
                                &market_authority,
                                &quote_mint,
                            ),
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
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn place_order(
        &self,
        market: Market,
        market_address: Pubkey,
        side: Side,
        price_lots: i64,
        max_base_lots: i64,
        max_quote_lots_including_fees: i64,
        client_order_id: u64,
        order_type: PlaceOrderType,
        expiry_timestamp: u64,
        limit: u8,
        user_token_account: Pubkey,
        market_vault: Pubkey,
        self_trade_behavior: SelfTradeBehavior,
    ) -> anyhow::Result<Signature> {
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
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::PlaceOrder {
                args: PlaceOrderArgs {
                    side,
                    price_lots,
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
        self.send_and_confirm_owner_tx(vec![ix]).await
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
    ) -> anyhow::Result<Signature> {
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
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn cancel_order(
        &self,
        market: Market,
        market_address: Pubkey,
        order_id: u128,
    ) -> anyhow::Result<Signature> {
        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::CancelOrder {
                        open_orders_account: self.open_orders_account,
                        signer: self.owner(),
                        market: market_address,
                        bids: market.bids,
                        asks: market.asks,
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::CancelOrder {
                order_id,
            }),
        };
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn cancel_all_orders(
        &self,
        market: Market,
        market_address: Pubkey,
        side_option: Option<Side>,
        limit: u8,
    ) -> anyhow::Result<Signature> {
        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::CancelOrder {
                        open_orders_account: self.open_orders_account,
                        signer: self.owner(),
                        market: market_address,
                        bids: market.bids,
                        asks: market.asks,
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::CancelAllOrders {
                side_option,
                limit,
            }),
        };
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn cancel_all_and_place_orders(
        &self,
        market: Market,
        market_address: Pubkey,
        user_base_account: Pubkey,
        user_quote_account: Pubkey,
        orders_type: PlaceOrderType,
        bids: Vec<PlaceMultipleOrdersArgs>,
        asks: Vec<PlaceMultipleOrdersArgs>,
        limit: u8,
    ) -> anyhow::Result<Signature> {
        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::CancelAllAndPlaceOrders {
                        open_orders_account: self.open_orders_account,
                        signer: self.owner(),
                        open_orders_admin: market.open_orders_admin.into(),
                        user_quote_account: user_quote_account,
                        user_base_account: user_base_account,
                        market: market_address,
                        bids: market.bids,
                        asks: market.asks,
                        event_heap: market.event_heap,
                        market_quote_vault: market.market_quote_vault,
                        market_base_vault: market.market_base_vault,
                        oracle_a: market.oracle_a.into(),
                        oracle_b: market.oracle_b.into(),
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
                    limit,
                },
            ),
        };
        self.send_and_confirm_owner_tx(vec![ix]).await
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
    ) -> anyhow::Result<Signature> {
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
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn settle_funds(
        &self,
        market: Market,
        market_address: Pubkey,
        user_base_account: Pubkey,
        user_quote_account: Pubkey,
        market_base_vault: Pubkey,
        market_quote_vault: Pubkey,
        referrer_account: Option<Pubkey>,
    ) -> anyhow::Result<Signature> {
        let ix = Instruction {
            program_id: openbook_v2::id(),
            accounts: {
                anchor_lang::ToAccountMetas::to_account_metas(
                    &openbook_v2::accounts::SettleFunds {
                        owner: self.owner(),
                        penalty_payer: self.owner(),
                        open_orders_account: self.open_orders_account,
                        market: market_address,
                        market_authority: market.market_authority,
                        user_base_account,
                        user_quote_account,
                        market_base_vault,
                        market_quote_vault,
                        referrer_account,
                        system_program: System::id(),
                        token_program: Token::id(),
                    },
                    None,
                )
            },
            data: anchor_lang::InstructionData::data(&openbook_v2::instruction::SettleFunds {}),
        };
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn consume_events(
        &self,
        market: Market,
        market_address: Pubkey,
        limit: usize,
    ) -> anyhow::Result<Signature> {
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
        self.send_and_confirm_owner_tx(vec![ix]).await
    }

    pub async fn send_and_confirm_owner_tx(
        &self,
        instructions: Vec<Instruction>,
    ) -> anyhow::Result<Signature> {
        TransactionBuilder {
            instructions,
            address_lookup_tables: vec![],
            payer: self.client.fee_payer.pubkey(),
            signers: vec![&*self.owner, &*self.client.fee_payer],
            config: self.client.transaction_builder_config,
        }
        .send_and_confirm(&self.client)
        .await
    }

    pub async fn send_and_confirm_permissionless_tx(
        &self,
        instructions: Vec<Instruction>,
    ) -> anyhow::Result<Signature> {
        TransactionBuilder {
            instructions,
            address_lookup_tables: vec![],
            payer: self.client.fee_payer.pubkey(),
            signers: vec![&*self.client.fee_payer],
            config: self.client.transaction_builder_config,
        }
        .send_and_confirm(&self.client)
        .await
    }

    pub async fn get_equity(&self) -> anyhow::Result<(f64, f64)> {
        let ba = &self.base_ata;
        let qa = &self.quote_ata;
        let rpc_client = self.client.rpc_async();

        let bb = rpc_client
            .get_token_account_balance(ba)
            .await?
            .ui_amount
            .unwrap();
        let qb = rpc_client
            .get_token_account_balance(qa)
            .await?
            .ui_amount
            .unwrap();

        Ok((bb, qb))
    }
    pub fn native_price_to_lots_price(&self, limit_price: f64) -> i64 {
        let base_decimals = self.market.base_decimals as u32;
        let quote_decimals = self.market.quote_decimals as u32;
        let base_factor = 10_u64.pow(base_decimals);
        let quote_factor = 10_u64.pow(quote_decimals);
        let price_factor = (base_factor / quote_factor) as f64;
        let price_lots = (limit_price * price_factor) as i64;
        price_lots
    }

    pub fn get_base_size_from_quote(&self, quote_size_usd: u64, limit_price: f64) -> u64 {
        let base_decimals = self.market.base_decimals as u32;
        let base_factor = 10_u64.pow(base_decimals) as f64;
        let base_size = ((quote_size_usd as f64 / limit_price) * base_factor) as u64;
        base_size
    }

    pub async fn load_bids_asks(&self) -> anyhow::Result<(Vec<OpenOrderNode>, BestQuotes)> {
        let mut best_quotes = BestQuotes {
            highest_bid: 0.,
            lowest_ask: 0.,
        };
        let mut open_orders = Vec::new();
        let _current_time = get_unix_secs();
        let orders_key = self.open_orders_account;

        let bids_book_side = self
            .client
            .rpc_anchor_account::<BookSide>(&self.market.bids)
            .await?;

        for (i, bid_book_side) in bids_book_side.iter_valid(0, None).enumerate() {
            let node = bid_book_side.node;
            let slot = node.owner_slot;
            let timestamp = node.timestamp;
            let owner_address = node.owner;
            let order_id = node.client_order_id;
            let lot_price = bid_book_side.price_lots;
            let native_price = self.market.lot_to_native_price(lot_price);
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
            .client
            .rpc_anchor_account::<BookSide>(&self.market.asks)
            .await
            .unwrap();

        for (i, ask_book_side) in asks_book_side.iter_valid(0, None).enumerate() {
            let node = ask_book_side.node;
            let slot = node.owner_slot;
            let timestamp = node.timestamp;
            let owner_address = node.owner;
            let order_id = node.client_order_id;
            let lot_price = ask_book_side.price_lots;
            let native_price = self.market.lot_to_native_price(lot_price);
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

    pub async fn load_oo_state(&self) -> anyhow::Result<OpenOrderState> {
        let result = self.load_oo_balances().await?;
        println!("got (jlp, usdc) bals: {:?}", result);

        Ok(OpenOrderState {
            jlp_in_oos: result.0,
            usdc_in_oos: result.1,
        })
    }

    pub async fn load_oo_balances(&self) -> anyhow::Result<(f64, f64)> {
        let open_orders_account = self.openorders_account().await?;
        let asks_native_price = self
            .market
            .lot_to_native_price(open_orders_account.position.asks_base_lots);
        let asks_base_total: f64 = I80F48::to_num::<f64>(asks_native_price) * 1000.;

        let bids_native_price = self
            .market
            .lot_to_native_price(open_orders_account.position.bids_base_lots);
        let bids_base_total: f64 = I80F48::to_num::<f64>(bids_native_price) * 1000.;

        Ok((bids_base_total, asks_base_total))
    }

    pub async fn load_open_orders(&self) -> anyhow::Result<Vec<OpenOrderNode>> {
        let open_orders_account = self.openorders_account().await?;
        let mut open_orders = Vec::new();
        for order in open_orders_account.all_orders_in_use() {
            let native_price = self.market.lot_to_native_price(order.locked_price);
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

    // each keypair must have exactly one indexer before it can create open orders accounts
    #[allow(dead_code)]
    async fn create_indexer(&self) -> anyhow::Result<(Pubkey, Signature)> {
        let r =
            OBClient::create_open_orders_indexer(&self.client, &self.keypair, &self.keypair).await;
        println!("Created indexer {:?}", r);
        r
    }

    // you can create multiple oo accounts per indexer, each with a dedicated market id
    #[allow(dead_code)]
    async fn create_open_orders_acc(&self) -> anyhow::Result<(Pubkey, Signature)> {
        let result = OBClient::create_open_orders_account(
            &self.client,
            self.market_id,
            &self.keypair,
            &self.keypair,
            None,
            1,
            "somename1",
        )
        .await;
        println!("got result: {:?}", result);
        result
    }

    pub async fn get_token_balance(&self, ata: &Pubkey) -> anyhow::Result<f64> {
        let rpc_client = self.client.rpc_async();
        let r = rpc_client.get_token_account_balance(&ata).await?;
        Ok(r.ui_amount.unwrap())
    }

    pub async fn get_sol_usdc_total(&self) -> anyhow::Result<AtaBalances> {
        let sol_ata = self.base_ata.clone();
        let usdc_ata = self.quote_ata.clone();
        let sol_balance = self.get_token_balance(&sol_ata).await?;
        let usdc_balance = self.get_token_balance(&usdc_ata).await?;
        let price = get_sol_price().await?;

        Ok(AtaBalances {
            usdc_balance,
            sol_balance,
            total_balance: usdc_balance + (price * sol_balance),
            price,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OBClientError {
    #[error("Transaction simulation error. Error: {err:?}, Logs: {}",
    .logs.iter().join("; ")
    )]
    SendTransactionPreflightFailure {
        err: Option<TransactionError>,
        logs: Vec<String>,
    },
}

#[derive(Copy, Clone, Debug)]
pub struct TransactionBuilderConfig {
    // adds a SetComputeUnitPrice instruction in front
    pub prioritization_micro_lamports: Option<u64>,
}

pub struct TransactionBuilder<'a> {
    pub instructions: Vec<Instruction>,
    pub address_lookup_tables: Vec<AddressLookupTableAccount>,
    pub signers: Vec<&'a Keypair>,
    pub payer: Pubkey,
    pub config: TransactionBuilderConfig,
}

impl<'a> TransactionBuilder<'a> {
    pub async fn transaction(
        self,
        rpc: &RpcClientAsync,
    ) -> anyhow::Result<solana_sdk::transaction::VersionedTransaction> {
        let latest_blockhash = rpc.get_latest_blockhash().await?;
        self.transaction_with_blockhash(latest_blockhash)
    }

    pub fn transaction_with_blockhash(
        mut self,
        blockhash: Hash,
    ) -> anyhow::Result<solana_sdk::transaction::VersionedTransaction> {
        if let Some(prio_price) = self.config.prioritization_micro_lamports {
            self.instructions.insert(
                0,
                solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(
                    prio_price,
                ),
            )
        }
        let v0_message = solana_sdk::message::v0::Message::try_compile(
            &self.payer,
            &self.instructions,
            &self.address_lookup_tables,
            blockhash,
        )?;
        let versioned_message = solana_sdk::message::VersionedMessage::V0(v0_message);
        let signers = self
            .signers
            .into_iter()
            .unique_by(|s| s.pubkey())
            .collect::<Vec<_>>();
        let tx =
            solana_sdk::transaction::VersionedTransaction::try_new(versioned_message, &signers)?;
        Ok(tx)
    }
}

pub async fn get_sol_price() -> anyhow::Result<f64> {
    const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
    let base_url = "https://price.jup.ag/v4/price?ids=";
    let url = format!("{base_url}{SOL_MINT}");
    let result = reqwest::get(url).await?;
    let prices = result.json::<PriceData>().await?;
    let sol_usdc_price = prices.data.get(SOL_MINT).unwrap().price;
    Ok(sol_usdc_price)
}
