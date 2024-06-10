use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use fixed::types::I80F48;
use openbook_v2::{
    accounts_zerocopy::KeyedAccountReader,
    error::OpenBookError,
    pubkey_option::NonZeroPubkeyOption,
    state::{oracle, OracleConfig, OracleState, OracleType, Side, FEES_SCALE_FACTOR},
};
use solana_sdk::pubkey::Pubkey;
use std::fmt::{Debug, Formatter, Result as FmtResult};

#[derive(Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct CreateMarketArgs {
    pub name: String,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_lot_size: i64,
    pub quote_lot_size: i64,
    pub maker_fee: i64,
    pub taker_fee: i64,
    pub oracle_a: Option<Pubkey>,
    pub oracle_b: Option<Pubkey>,
    pub open_orders_admin: Option<Pubkey>,
    pub collect_fee_admin: Pubkey,
    pub consume_events_admin: Option<Pubkey>,
    pub close_market_admin: Option<Pubkey>,
    pub time_expiry: i64,
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct MarketInfo {
    pub name: String,
    pub base_decimals: u8,
    pub quote_decimals: u8,
    pub market_authority: Pubkey,
    pub collect_fee_admin: Pubkey,
    pub open_orders_admin: NonZeroPubkeyOption,
    pub consume_events_admin: NonZeroPubkeyOption,
    pub close_market_admin: NonZeroPubkeyOption,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub event_heap: Pubkey,
    pub oracle_a: NonZeroPubkeyOption,
    pub oracle_b: NonZeroPubkeyOption,
    pub oracle_config: OracleConfig,
    pub quote_lot_size: i64,
    pub base_lot_size: i64,
    pub seq_num: u64,
    pub registration_time: i64,
    pub maker_fee: i64,
    pub taker_fee: i64,
    pub fees_accrued: u128,
    pub fees_to_referrers: u128,
    pub referrer_rebates_accrued: u64,
    pub fees_available: u64,
    pub maker_volume: u128,
    pub taker_volume_wo_oo: u128,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub market_base_vault: Pubkey,
    pub base_deposit_total: u64,
    pub market_quote_vault: Pubkey,
    pub quote_deposit_total: u64,
}

impl Debug for MarketInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "MarketInfo {{")?;
        writeln!(f, "    name: {:?}", self.name)?;
        writeln!(f, "    base_decimals: {:?}", self.base_decimals)?;
        writeln!(f, "    quote_decimals: {:?}", self.quote_decimals)?;
        writeln!(f, "    market_authority: {:?}", self.market_authority)?;
        writeln!(f, "    collect_fee_admin: {:?}", self.collect_fee_admin)?;
        writeln!(f, "    open_orders_admin: {:?}", self.open_orders_admin)?;
        writeln!(
            f,
            "    consume_events_admin: {:?}",
            self.consume_events_admin
        )?;
        writeln!(f, "    close_market_admin: {:?}", self.close_market_admin)?;
        writeln!(f, "    bids: {:?}", self.bids)?;
        writeln!(f, "    asks: {:?}", self.asks)?;
        writeln!(f, "    event_heap: {:?}", self.event_heap)?;
        writeln!(f, "    oracle_a: {:?}", self.oracle_a)?;
        writeln!(f, "    oracle_b: {:?}", self.oracle_b)?;
        writeln!(
            f,
            "    oracle_conf_filter: {:?}",
            self.oracle_config.conf_filter
        )?;
        writeln!(f, "    quote_lot_size: {:?}", self.quote_lot_size)?;
        writeln!(f, "    base_lot_size: {:?}", self.base_lot_size)?;
        writeln!(f, "    seq_num: {:?}", self.seq_num)?;
        writeln!(f, "    registration_time: {:?}", self.registration_time)?;
        writeln!(f, "    maker_fee: {:?}", self.maker_fee)?;
        writeln!(f, "    taker_fee: {:?}", self.taker_fee)?;
        writeln!(f, "    fees_accrued: {:?}", self.fees_accrued)?;
        writeln!(f, "    fees_to_referrers: {:?}", self.fees_to_referrers)?;
        writeln!(
            f,
            "    referrer_rebates_accrued: {:?}",
            self.referrer_rebates_accrued
        )?;
        writeln!(f, "    fees_available: {:?}", self.fees_available)?;
        writeln!(f, "    maker_volume: {:?}", self.maker_volume)?;
        writeln!(f, "    taker_volume_wo_oo: {:?}", self.taker_volume_wo_oo)?;
        writeln!(f, "    base_mint: {:?}", self.base_mint)?;
        writeln!(f, "    quote_mint: {:?}", self.quote_mint)?;
        writeln!(f, "    market_base_vault: {:?}", self.market_base_vault)?;
        writeln!(f, "    base_deposit_total: {:?}", self.base_deposit_total)?;
        writeln!(f, "    market_quote_vault: {:?}", self.market_quote_vault)?;
        writeln!(f, "    quote_deposit_total: {:?}", self.quote_deposit_total)?;
        writeln!(f, "}}")
    }
}

impl MarketInfo {
    pub fn is_expired(&self, timestamp: i64) -> bool {
        self.registration_time != 0 && self.registration_time < timestamp
    }

    pub fn is_empty(&self) -> bool {
        self.base_deposit_total == 0
            && self.quote_deposit_total == 0
            && self.fees_available == 0
            && self.referrer_rebates_accrued == 0
    }

    pub fn is_market_vault(&self, pubkey: Pubkey) -> bool {
        pubkey == self.market_quote_vault || pubkey == self.market_base_vault
    }

    pub fn get_vault_by_side(&self, side: Side) -> Pubkey {
        match side {
            Side::Ask => self.market_base_vault,
            Side::Bid => self.market_quote_vault,
        }
    }

    pub fn max_base_lots(&self) -> i64 {
        i64::MAX / self.base_lot_size
    }

    pub fn max_quote_lots(&self) -> i64 {
        i64::MAX / self.quote_lot_size
    }

    pub fn max_base_lots_from_lamports(&self, lamports: u64) -> i64 {
        let base_lots = lamports / self.base_lot_size as u64;
        std::cmp::min(self.max_base_lots() as u64, base_lots)
            .try_into()
            .unwrap()
    }

    pub fn max_quote_lots_from_lamports(&self, lamports: u64) -> i64 {
        let quote_lots = lamports / self.quote_lot_size as u64;
        std::cmp::min(self.max_quote_lots() as u64, quote_lots)
            .try_into()
            .unwrap()
    }

    pub fn lot_to_native_price(&self, price: i64) -> I80F48 {
        I80F48::from_num(price) * I80F48::from_num(self.quote_lot_size)
            / I80F48::from_num(self.base_lot_size)
    }

    pub fn native_price_to_lot(&self, price: I80F48) -> Result<i64, OpenBookError> {
        price
            .checked_mul(I80F48::from_num(self.base_lot_size))
            .and_then(|x| x.checked_div(I80F48::from_num(self.quote_lot_size)))
            .and_then(|x| x.checked_to_num())
            .ok_or_else(|| OpenBookError::InvalidOraclePrice.into())
    }

    pub fn oracle_price_lots(
        &self,
        oracle_a_acc: Option<&impl KeyedAccountReader>,
        oracle_b_acc: Option<&impl KeyedAccountReader>,
        slot: u64,
    ) -> Result<Option<i64>, OpenBookError> {
        let oracle_price = self.oracle_price(oracle_a_acc, oracle_b_acc, slot)?;
        match oracle_price {
            Some(p) => Ok(Some(self.native_price_to_lot(p)?)),
            None => Ok(None),
        }
    }

    pub fn oracle_price(
        &self,
        oracle_a_acc: Option<&impl KeyedAccountReader>,
        oracle_b_acc: Option<&impl KeyedAccountReader>,
        slot: u64,
    ) -> Result<Option<I80F48>, OpenBookError> {
        if self.oracle_a != Pubkey::default() && self.oracle_b != Pubkey::default() {
            self.oracle_price_from_a_and_b(oracle_a_acc.unwrap(), oracle_b_acc.unwrap(), slot)
        } else if self.oracle_a != Pubkey::default() {
            self.oracle_price_from_a(oracle_a_acc.unwrap(), slot)
        } else {
            Ok(None)
        }
    }

    fn oracle_price_from_a(
        &self,
        oracle_acc: &impl KeyedAccountReader,
        now_slot: u64,
    ) -> Result<Option<I80F48>, OpenBookError> {
        assert_eq!(self.oracle_a, *oracle_acc.key());
        let oracle = oracle::oracle_state_unchecked(oracle_acc).unwrap_or(OracleState {
            price: 0.0,
            deviation: 0.0,
            last_update_slot: 0,
            oracle_type: OracleType::Pyth,
        });

        if oracle.is_stale(oracle_acc.key(), &self.oracle_config, now_slot)
            || !oracle.has_valid_confidence(oracle_acc.key(), &self.oracle_config)
        {
            Ok(None)
        } else {
            let decimals = (self.quote_decimals as i8) - (self.base_decimals as i8);
            let decimal_adj = oracle::power_of_ten_float(decimals);
            Ok(I80F48::checked_from_num(oracle.price * decimal_adj))
        }
    }

    fn oracle_price_from_a_and_b(
        &self,
        oracle_a_acc: &impl KeyedAccountReader,
        oracle_b_acc: &impl KeyedAccountReader,
        now_slot: u64,
    ) -> Result<Option<I80F48>, OpenBookError> {
        assert_eq!(self.oracle_a, *oracle_a_acc.key());
        assert_eq!(self.oracle_b, *oracle_b_acc.key());

        let oracle_a = oracle::oracle_state_unchecked(oracle_a_acc).unwrap_or(OracleState {
            price: 0.0,
            deviation: 0.0,
            last_update_slot: 0,
            oracle_type: OracleType::Pyth,
        });
        let oracle_b = oracle::oracle_state_unchecked(oracle_b_acc).unwrap_or(OracleState {
            price: 0.0,
            deviation: 0.0,
            last_update_slot: 0,
            oracle_type: OracleType::Pyth,
        });

        if oracle_a.is_stale(oracle_a_acc.key(), &self.oracle_config, now_slot)
            || oracle_b.is_stale(oracle_b_acc.key(), &self.oracle_config, now_slot)
            || !oracle_a.has_valid_combined_confidence(&oracle_b, &self.oracle_config)
        {
            Ok(None)
        } else {
            let price = oracle_a.price / oracle_b.price;
            let decimals = (self.quote_decimals as i8) - (self.base_decimals as i8);
            let decimal_adj = oracle::power_of_ten_float(decimals);
            Ok(I80F48::checked_from_num(price * decimal_adj))
        }
    }

    pub fn subtract_taker_fees(&self, quote: i64) -> i64 {
        ((quote as i128) * FEES_SCALE_FACTOR / (FEES_SCALE_FACTOR + (self.taker_fee as i128)))
            .try_into()
            .unwrap()
    }

    pub fn maker_fees_floor(&self, amount: u64) -> u64 {
        if self.maker_fee.is_positive() {
            self.unsigned_maker_fees_floor(amount)
        } else {
            0
        }
    }

    pub fn maker_rebate_floor(&self, amount: u64) -> u64 {
        if self.maker_fee.is_positive() {
            0
        } else {
            self.unsigned_maker_fees_floor(amount)
        }
    }

    pub fn maker_fees_ceil<T>(&self, amount: T) -> T
    where
        T: Into<i128> + TryFrom<i128> + From<u8>,
        <T as TryFrom<i128>>::Error: std::fmt::Debug,
    {
        if self.maker_fee.is_positive() {
            self.ceil_fee_division(amount.into() * (self.maker_fee.abs() as i128))
                .try_into()
                .unwrap()
        } else {
            T::from(0)
        }
    }

    pub fn taker_fees_ceil<T>(&self, amount: T) -> T
    where
        T: Into<i128> + TryFrom<i128>,
        <T as TryFrom<i128>>::Error: std::fmt::Debug,
    {
        self.ceil_fee_division(amount.into() * (self.taker_fee as i128))
            .try_into()
            .unwrap()
    }

    fn ceil_fee_division(&self, numerator: i128) -> i128 {
        (numerator + (FEES_SCALE_FACTOR - 1_i128)) / FEES_SCALE_FACTOR
    }

    fn unsigned_maker_fees_floor(&self, amount: u64) -> u64 {
        (i128::from(amount) * i128::from(self.maker_fee.abs()) / FEES_SCALE_FACTOR)
            .try_into()
            .unwrap()
    }
}
