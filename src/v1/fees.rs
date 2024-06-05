//! This module contains functions related to the openbook market fees.

use crate::tokens_and_markets::get_layout_version;
use solana_sdk::pubkey::Pubkey;

/// Checks if a program supports SRM fee discounts based on its layout version.
///
/// # Arguments
///
/// * `program_id` - The program ID to check for SRM fee discounts.
///
/// # Returns
///
/// `true` if the program supports SRM fee discounts, `false` otherwise.
///
/// # Examples
///
/// ```rust
/// use openbook::pubkey::Pubkey;
/// use openbook::fees::supports_srm_fee_discounts;
///
/// let program_id = Pubkey::new_unique();
/// let is_supported = supports_srm_fee_discounts(&program_id);
///
/// assert_eq!(is_supported, true);
/// ```
pub fn supports_srm_fee_discounts(program_id: &Pubkey) -> bool {
    get_layout_version(program_id) as u64 > 0
}

/// Gets the fee rates for a given fee tier.
///
/// # Arguments
///
/// * `fee_tier` - The fee tier for which to retrieve fee rates.
///
/// # Returns
///
/// A tuple containing the taker and maker fee rates.
///
/// # Examples
///
/// ```rust
/// use openbook::fees::get_fee_rates;
///
/// let fee_tier = 3;
/// let (taker_rate, maker_rate) = get_fee_rates(fee_tier);
///
/// assert_eq!((taker_rate, maker_rate), (0.0016, -0.0003));
/// ```
pub fn get_fee_rates(fee_tier: u32) -> (f64, f64) {
    match fee_tier {
        1 => (0.002, -0.0003),  // SRM2
        2 => (0.0018, -0.0003), // SRM3
        3 => (0.0016, -0.0003), // SRM4
        4 => (0.0014, -0.0003), // SRM5
        5 => (0.0012, -0.0003), // SRM6
        6 => (0.001, -0.0005),  // MSRM
        _ => (0.0022, -0.0003), // Base
    }
}

/// Gets the fee tier based on MSRM and SRM balances.
///
/// # Arguments
///
/// * `msrm_balance` - The MSRM (Mega SRM) balance.
/// * `srm_balance` - The SRM (Serum) balance.
///
/// # Returns
///
/// The fee tier corresponding to the balances.
///
/// # Examples
///
/// ```rust
/// use openbook::fees::get_fee_tier;
///
/// let msrm_balance = 1.5;
/// let srm_balance = 0.0;
/// let fee_tier = get_fee_tier(msrm_balance, srm_balance);
///
/// assert_eq!(fee_tier, 6);
/// ```
pub fn get_fee_tier(msrm_balance: f64, srm_balance: f64) -> u32 {
    if msrm_balance >= 1.0 {
        6
    } else if srm_balance >= 1_000_000.0 {
        5
    } else if srm_balance >= 100_000.0 {
        4
    } else if srm_balance >= 10_000.0 {
        3
    } else if srm_balance >= 1_000.0 {
        2
    } else if srm_balance >= 100.0 {
        1
    } else {
        0
    }
}
