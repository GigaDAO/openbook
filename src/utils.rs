//! This module contains utility functions related openbook.

use crate::{bs58, keypair::Keypair};
use solana_sdk::{account::Account, account_info::AccountInfo, pubkey::Pubkey};
use std::{fs, time::SystemTime, time::UNIX_EPOCH};

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
/// use openbook::utils::u64_slice_to_pubkey;
///
/// let slice = [1, 2, 3, 4];
/// let pubkey = u64_slice_to_pubkey(slice);
/// ```
pub fn u64_slice_to_pubkey(array: [u64; 4]) -> [u8; 32] {
    let mut result = [0u8; 32];
    for (i, &item) in array.iter().enumerate() {
        result[i * 8..(i + 1) * 8].copy_from_slice(&item.to_le_bytes());
    }
    result
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
/// use openbook::utils::read_keypair;
///
/// let path = String::from("/path/to/keypair_file.json");
/// // let keypair = read_keypair(&path);
/// ```
pub fn read_keypair(path: &String) -> Keypair {
    let secret_string: String = fs::read_to_string(path).unwrap_or_default();
    let mut keypair = Keypair::new();
    if !secret_string.is_empty() {
        let secret_bytes: Vec<u8> = match serde_json::from_str(&secret_string) {
            Ok(bytes) => bytes,
            Err(_) => match bs58::decode(&secret_string.trim()).into_vec() {
                Ok(bytes) => bytes,
                Err(_) => panic!("failed to load secret key from file"),
            },
        };
        keypair = Keypair::from_bytes(&secret_bytes)
            .expect("failed to generate keypair from secret bytes");
    }
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
/// use openbook::utils::get_unix_secs;
///
/// let timestamp = get_unix_secs();
/// ```
pub fn get_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Creates an `AccountInfo` instance from an `Account`.
///
/// # Arguments
///
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
/// use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
/// use openbook::tokens_and_markets::{get_market_name, get_program_id};
/// use openbook::utils::{read_keypair, create_account_info_from_account};
/// use openbook::state::MarketState;
/// use openbook::tokens_and_markets::{DexVersion, Token};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
///     let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
///
///     let program_id = get_program_id(DexVersion::default()).parse()?;
///     let market_address = get_market_name(Token::USDC).0.parse()?;
///
///     let rpc_client = RpcClient::new(rpc_url.clone());
///
///     let mut account = rpc_client.get_account(&market_address).await?;
///
///     let account_info = create_account_info_from_account(
///         &mut account,
///         &market_address,
///         &program_id,
///         false,
///         false,
///     );
///
///     println!("{:?}", account_info);
///
///     Ok(())
/// }
/// ```
pub fn create_account_info_from_account<'a>(
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
        my_program_id,
        account.executable,
        account.rent_epoch,
    )
}
