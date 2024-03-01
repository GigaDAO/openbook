//! This module contains utility functions related openbook.

use crate::{account::Account, bs58, keypair::Keypair, pubkey::Pubkey, rpc_client::RpcClient};
use solana_account_decoder::UiAccountEncoding;
use solana_account_decoder::UiDataSliceConfig;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_client::rpc_config::RpcProgramAccountsConfig;
use solana_client::rpc_filter::RpcFilterType;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::sysvar::slot_history::ProgramError;
use std::fs;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

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
/// let bytes_array = u64_slice_to_bytes(&slice);
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
/// use openbook::market::read_keypair;
///
/// let path = String::from("/path/to/keypair_file.json");
/// let keypair = read_keypair(&path);
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
/// use openbook::market::get_unix_secs;
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
/// use openbook::orders::get_filtered_program_accounts;
/// use openbook::{pubkey::Pubkey, account::Account, rpc_client::RpcClient};
///
/// let rpc_url = String::default();
/// let connection = RpcClient::new(rpc_url);
/// let program_id = Pubkey::default();
/// let filters = vec![];
///
/// match get_filtered_program_accounts(&connection, &program_id, filters) {
///     Ok(accounts) => println!("Filtered accounts: {:?}", accounts),
///     Err(err) => eprintln!("Error getting filtered accounts: {:?}", err),
/// }
/// ```
pub fn get_filtered_program_accounts(
    connection: &RpcClient,
    program_id: &Pubkey,
    filters: Vec<RpcFilterType>,
) -> Result<Vec<Account>, ProgramError> {
    let _config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: Some(UiDataSliceConfig {
                offset: 0,
                length: 5,
            }),
            commitment: Some(CommitmentConfig::processed()),
            min_context_slot: Some(1234),
        },
        with_context: Some(false),
    };
    // TODO: Fix 410 resource is no longer available?
    // let resp = connection
    //     .get_program_accounts_with_config(&program_id, config)
    //     .unwrap();
    let resp = connection.get_account(program_id).unwrap();

    // let mut result = Vec::new();

    // for (_, account) in resp.into_iter() {
    //     result.push(account);
    // }

    Ok(vec![resp])
}
