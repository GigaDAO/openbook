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

/// Represents openbook market ids, and base mints associated with the tokens.
///
/// This static array contains tuples where the first element represents the market ID,
/// the second element represents the associated base mint, and the third element represents
/// the associated token.
pub static MARKET_IDS_TO_NAMES: [(&str, &str, &str); 6] = [
    (
        "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6",
        "So11111111111111111111111111111111111111112",
        "sol",
    ),
    (
        "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "usdc",
    ),
    (
        "HTHMfoxePjcXFhrV74pfCUNoWGe374ecFwiDjPGTkzHr",
        "SLNDpmoWTVADgEdndyvWzroNL7zSi1dF9PC3xHGtPwp",
        "slnd",
    ),
    (
        "DZjbn4XC8qoHKikZqzmhemykVzmossoayV9ffbsUqxVj",
        "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
        "ray",
    ),
    (
        "BbJgE7HZMaDp5NTYvRh5jZSkQPVDTU8ubPFtpogUkEj4",
        "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs",
        "eth",
    ),
    (
        "CC9VYJprbxacpiS94tPJ1GyBhfvrLQbUiUSVMWvFohNW",
        "MNDEFzGvMt87ueuHvVU9VcTqsAP5b3fTGPsHuuPA5ey",
        "mnde",
    ),
    // TODO: add all markets
];

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
/// let market_id = get_market_name("usdc").0;
///
/// assert_eq!(&market_id, "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6");
/// ```
pub fn get_market_name(market_name: &str) -> (String, String) {
    MARKET_IDS_TO_NAMES
        .iter()
        .find(|(_, _, val)| *val == market_name)
        .map(|(id, base, _)| (id.to_string(), base.to_string()))
        .unwrap_or_else(|| {
            (
                "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6".to_string(),
                "So11111111111111111111111111111111111111112".to_string(),
            )
        })
}
