use openbook::{
    pubkey::Pubkey,
    tokens_and_markets::{get_layout_version, get_program_id},
};
use std::str::FromStr;

#[test]
fn test_get_layout_version_known_id() {
    let known_program_id =
        Pubkey::from_str("4ckmDgGdxQoPDLUkDT3vHgSAkzA3QRdNq5ywwY4sUSJn").unwrap();
    assert_eq!(get_layout_version(&known_program_id), 1);
}

#[test]
fn test_get_layout_version_non_existing_id() {
    let program_id = Pubkey::new_unique();
    assert_eq!(get_layout_version(&program_id), 3);
}

#[test]
fn test_get_program_id_existing_version() {
    assert_eq!(
        &get_program_id(1),
        "4ckmDgGdxQoPDLUkDT3vHgSAkzA3QRdNq5ywwY4sUSJn"
    );
    assert_eq!(
        &get_program_id(2),
        "EUqojwWA2rd19FZrzeBncJsm38Jm1hEhE3zsmX3bRc2o"
    );
    assert_eq!(
        &get_program_id(3),
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"
    );
}

#[test]
fn test_get_program_id_non_existing_version() {
    assert_eq!(
        &get_program_id(4),
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"
    );
}
