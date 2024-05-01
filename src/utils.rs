//! This module contains utility functions related openbook.

use crate::{bs58, keypair::Keypair};
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
