//! This module contains utility functions related openbook token and market info.

use solana_sdk::pubkey::Pubkey;

/// Represents the layout versions associated with Solana programs.
///
/// This static array contains tuples where the first element represents the program ID in string format,
/// and the second element represents the associated layout version.
///
/// # Example
///
/// ```
/// use solana_sdk::pubkey::Pubkey;
/// use openbook::tokens_and_markets::get_layout_version;
///
/// // Access the layout version for a specific program ID
/// let program_id = Pubkey::new_unique();
/// let version = get_layout_version(&program_id);
///
/// assert_eq!(version, 3);
/// ```
pub static PROGRAM_LAYOUT_VERSIONS: [(&str, u8); 4] = [
    ("4ckmDgGdxQoPDLUkDT3vHgSAkzA3QRdNq5ywwY4sUSJn", 1), // DEX Version 1
    ("BJ3jrUzddfuSrZHXSCxMUUQsjKEyLmuuyZebkcaFp2fg", 1), // DEX Version 1
    ("EUqojwWA2rd19FZrzeBncJsm38Jm1hEhE3zsmX3bRc2o", 2), // DEX Version 2
    ("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX", 3),  // DEX Version 3
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
/// use solana_sdk::pubkey::Pubkey;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_layout_version_known_id() {
        let program_id = Pubkey::new_unique();
        assert_eq!(get_layout_version(&program_id), 3);

        let known_program_id = Pubkey::new_from_array([
            0x4c, 0x6d, 0x47, 0x67, 0x44, 0x67, 0x64, 0x78, 0x51, 0x6f, 0x50, 0x44, 0x4c, 0x55,
            0x6b, 0x44, 0x54, 0x33, 0x76, 0x48, 0x67, 0x53, 0x41, 0x6b, 0x7a, 0x41, 0x33, 0x51,
            0x52, 0x64, 0x4e, 0x71,
        ]);
        assert_eq!(get_layout_version(&known_program_id), 3);
    }
}
