//! This module contains utility functions related openbook token and market info.

use solana_sdk::pubkey::Pubkey;

/// Represents the layout versions associated with Solana programs.
///
/// This static array contains tuples where the first element represents the program ID in string format,
/// and the second element represents the associated layout version.
pub static PROGRAM_LAYOUT_VERSIONS: [(&str, u8); 4] = [
    ("4ckmDgGdxQoPDLUkDT3vHgSAkzA3QRdNq5ywwY4sUSJn", 1), // DEX Version 1
    ("BJ3jrUzddfuSrZHXSCxMUUQsjKEyLmuuyZebkcaFp2fg", 1), // DEX Version 1
    ("EUqojwWA2rd19FZrzeBncJsm38Jm1hEhE3zsmX3bRc2o", 2), // DEX Version 2
    ("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX", 3),  // DEX Version 3
];

/// Represents openbook market ids associated with the market names.
///
/// This static array contains tuples where the first element represents the market ID in string format,
/// and the second element represents the associated market name.
pub static MARKET_IDS_TO_NAMES: [(&str, &str); 1] =
    [("8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6", "openbook")];

/// Gets the layout version for the given program ID.
///
/// # Arguments
///
/// * `program_id` - The program ID for which the layout version is requested.
///
/// # Returns
///
/// The layout version associated with the program ID. Returns 3 if not found.
///
/// # Examples
///
/// ```rust
/// use openbook::pubkey::Pubkey;
/// use openbook::tokens_and_markets::get_layout_version;
///
/// let program_id = Pubkey::new_unique();
/// let version = get_layout_version(&program_id);
///
/// assert_eq!(version, 3);
/// ```
pub fn get_layout_version(program_id: &Pubkey) -> u8 {
    PROGRAM_LAYOUT_VERSIONS
        .iter()
        .find(|(id, _)| *id == program_id.to_string())
        .map(|(_, version)| *version)
        .unwrap_or(3)
}

/// Gets the program ID for the given layout version.
///
/// # Arguments
///
/// * `version` - The layout version for which the program ID is requested.
///
/// # Returns
///
/// The program ID associated with the layout version. Returns program ID for version
/// 3 if not found.
///
/// # Examples
///
/// ```rust
/// use openbook::pubkey::Pubkey;
/// use openbook::tokens_and_markets::get_program_id;
///
/// let program_id = get_program_id(3);
///
/// assert_eq!(&program_id, "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX");
/// ```
pub fn get_program_id(version: u8) -> String {
    PROGRAM_LAYOUT_VERSIONS
        .iter()
        .find(|(_, v)| *v == version)
        .map(|(id, _)| id.to_string())
        .unwrap_or("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX".to_string())
}

/// Gets the market id for the given market name.
///
/// # Arguments
///
/// * `market_name` - The market name for which the market id is requested.
///
/// # Returns
///
/// The market id ID associated with the market name. Returns market ID for "openbook" market
/// if not found.
///
/// # Examples
///
/// ```rust
/// use openbook::pubkey::Pubkey;
/// use openbook::tokens_and_markets::get_market_name;
///
/// let market_id = get_market_name("openbook");
///
/// assert_eq!(&market_id, "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6");
/// ```
pub fn get_market_name(market_name: &str) -> String {
    MARKET_IDS_TO_NAMES
        .iter()
        .find(|(_, val)| *val == market_name)
        .map(|(id, _)| id.to_string())
        .unwrap_or_else(|| "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".to_string())
}
