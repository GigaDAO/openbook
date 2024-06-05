//! This module contains utility functions related openbook token and market info.

use solana_sdk::pubkey::Pubkey;

/// DEX versions, dep means deprecated
#[derive(Debug, PartialEq, PartialOrd, Default, Clone, Copy)]
pub enum DexVersion {
    /// DEX Version 1
    DepDexV0,
    /// DEX Version 1
    DepDexV1,
    /// DEX Version 2
    DexV2,
    /// OpenBook v1 Dex, aka srm
    #[default]
    DexV3,
    /// OpenBook v2 Dex, aka opnb
    DexV4,
}

/// Represents tokens associated with markets on the Solana blockchain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token {
    SOL,
    USDC,
    SLND,
    RAY,
    ETH,
    MNDE,
    JLP,
    // TODO: Add more tokens
}

impl Token {
    /// Parse a string into a Token enum value, ignoring case.
    pub fn from_str(s: &str) -> Option<Token> {
        match s.to_lowercase().as_str() {
            "sol" => Some(Token::SOL),
            "usdc" => Some(Token::USDC),
            "slnd" => Some(Token::SLND),
            "ray" => Some(Token::RAY),
            "eth" => Some(Token::ETH),
            "mnde" => Some(Token::MNDE),
            "jlp" => Some(Token::JLP),
            _ => None,
        }
    }
}

pub static SPL_TOKEN_ID: &'static str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// Represents the layout versions associated with Solana programs.
///
/// This static array contains tuples where the first element represents the program ID in string format,
/// and the second element represents the associated layout version.
pub static PROGRAM_LAYOUT_VERSIONS: [(&str, DexVersion); 5] = [
    (
        "4ckmDgGdxQoPDLUkDT3vHgSAkzA3QRdNq5ywwY4sUSJn",
        DexVersion::DepDexV0,
    ),
    (
        "BJ3jrUzddfuSrZHXSCxMUUQsjKEyLmuuyZebkcaFp2fg",
        DexVersion::DepDexV1,
    ),
    (
        "EUqojwWA2rd19FZrzeBncJsm38Jm1hEhE3zsmX3bRc2o",
        DexVersion::DexV2,
    ),
    (
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX",
        DexVersion::DexV3,
    ),
    (
        "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb",
        DexVersion::DexV4,
    ),
];

/// Represents openbook market ids, base mints, and associated tokens.
///
/// This static array contains tuples where the first element represents the market ID,
/// the second element represents the associated base mint, and the third element represents
/// the associated token, expressed as an enum.
pub static MARKET_IDS_TO_NAMES: [(&'static str, &'static str, Token); 7] = [
    (
        "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6",
        "So11111111111111111111111111111111111111112",
        Token::SOL,
    ),
    (
        "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        Token::USDC,
    ),
    (
        "HTHMfoxePjcXFhrV74pfCUNoWGe374ecFwiDjPGTkzHr",
        "SLNDpmoWTVADgEdndyvWzroNL7zSi1dF9PC3xHGtPwp",
        Token::SLND,
    ),
    (
        "DZjbn4XC8qoHKikZqzmhemykVzmossoayV9ffbsUqxVj",
        "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
        Token::RAY,
    ),
    (
        "BbJgE7HZMaDp5NTYvRh5jZSkQPVDTU8ubPFtpogUkEj4",
        "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs",
        Token::ETH,
    ),
    (
        "CC9VYJprbxacpiS94tPJ1GyBhfvrLQbUiUSVMWvFohNW",
        "MNDEFzGvMt87ueuHvVU9VcTqsAP5b3fTGPsHuuPA5ey",
        Token::MNDE,
    ),
    (
        "ASUyMMNBpFzpW3zDSPYdDVggKajq1DMKFFPK1JS9hoSR",
        "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4",
        Token::JLP,
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
/// use openbook::tokens_and_markets::DexVersion;
///
/// let program_id = Pubkey::new_unique();
/// let version = get_layout_version(&program_id);
///
/// assert_eq!(version, DexVersion::DexV3);
/// ```
pub fn get_layout_version(program_id: &Pubkey) -> DexVersion {
    PROGRAM_LAYOUT_VERSIONS
        .iter()
        .find(|(id, _)| *id == program_id.to_string())
        .map(|(_, version)| *version)
        .unwrap_or(DexVersion::DexV3)
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
/// use openbook::tokens_and_markets::DexVersion;
///
/// let program_id = get_program_id(DexVersion::default());
///
/// assert_eq!(&program_id, "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX");
/// ```
pub fn get_program_id(version: DexVersion) -> String {
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
/// use openbook::tokens_and_markets::Token;
///
/// let market_id = get_market_name(Token::USDC).0;
///
/// assert_eq!(&market_id, "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6");
/// ```
pub fn get_market_name(market_name: Token) -> (String, String) {
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
