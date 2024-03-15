//! This module contains utility functions related openbook.

use crate::{account::Account, bs58, keypair::Keypair, rpc_client::RpcClient};
use log::debug;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::RpcFilterType,
};
use solana_sdk::sysvar::slot_history::ProgramError;
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
/// use openbook::utils::u64_slice_to_bytes;
///
/// let slice = [1, 2, 3, 4];
/// let bytes_array = u64_slice_to_bytes(slice);
/// ```
pub fn u64_slice_to_bytes(array: [u64; 4]) -> [u8; 32] {
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
    let secret_string: String = fs::read_to_string(path).expect("Can't find key file");
    let secret_bytes: Vec<u8> = match serde_json::from_str(&secret_string) {
        Ok(bytes) => bytes,
        Err(_) => match bs58::decode(&secret_string.trim()).into_vec() {
            Ok(bytes) => bytes,
            Err(_) => panic!("failed to load secret key from file"),
        },
    };
    Keypair::from_bytes(&secret_bytes).expect("failed to generate keypair from secret bytes")
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

/// Helper function for getting filtered program accounts based on specified filters.
///
/// # Arguments
///
/// * `connection` - The RPC client for interacting with the Solana blockchain.
/// * `program_id` - The program ID associated with the accounts.
/// * `filters` - List of filters to apply for querying accounts.
///
/// # Returns
///
/// A `Result` containing a vector of `Account` instances or a `ProgramError`.
///
/// # Examples
///
/// ```rust
/// use openbook::utils::get_filtered_program_accounts;
/// use openbook::rpc_filter::RpcFilterType;
/// use openbook::rpc_filter::MemcmpEncoding;
/// use openbook::bs58;
/// use openbook::rpc_filter::Memcmp;
/// use openbook::rpc_filter::MemcmpEncodedBytes;
/// use openbook::{pubkey::Pubkey, account::Account, rpc_client::RpcClient};
/// use openbook::tokens_and_markets::get_market_name;
/// use std::str::FromStr;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set");
///     let connection = RpcClient::new(rpc_url);
///     let market_address: Pubkey = get_market_name("usdc").0.parse().unwrap();
///
///     let filters = vec![
///         RpcFilterType::Memcmp(Memcmp {
///             offset: 32,
///             bytes: MemcmpEncodedBytes::Base58(bs58::encode(market_address).into_string()),
///             encoding: Some(MemcmpEncoding::Binary),
///         }),
///         RpcFilterType::DataSize(165),
///     ];
///
///     match get_filtered_program_accounts(&connection, filters).await {
///         Ok(accounts) => println!("Filtered accounts: {:?}", accounts),
///         Err(err) => eprintln!("Error getting filtered accounts: {:?}", err),
///     }
///     Ok(())
/// }
/// ```
pub async fn get_filtered_program_accounts(
    connection: &RpcClient,
    filters: Vec<RpcFilterType>,
) -> Result<Vec<Account>, ProgramError> {
    let config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(connection.commitment()),
            ..RpcAccountInfoConfig::default()
        },
        with_context: Some(false),
    };
    // Error 410: Resource No Longer Available
    // The occurrence of this error is attributed to the high cost of the
    // `get_program_accounts_with_config` call on the mainnet-beta network. As a result,
    // consider utilizing the Helius network as an alternative to mitigate this issue.
    let accounts = connection
        .get_program_accounts_with_config(&anchor_spl::token::ID, config)
        .await
        .unwrap();

    let mut result = Vec::new();

    for (i, account) in accounts.iter().enumerate() {
        debug!("\n[*] ATA {:?}:  {:?}\n", i, account);
        result.push(account.1.clone());
    }

    Ok(result)
}
