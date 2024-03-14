use openbook::fees::{get_fee_rates, get_fee_tier, supports_srm_fee_discounts};
use openbook::pubkey::Pubkey;

#[test]
fn test_supports_srm_fee_discounts() {
    let program_id = Pubkey::new_unique();
    assert!(supports_srm_fee_discounts(&program_id));
}

#[test]
fn test_get_fee_rates() {
    assert_eq!(get_fee_rates(1), (0.002, -0.0003));
    assert_eq!(get_fee_rates(3), (0.0016, -0.0003));
    assert_eq!(get_fee_rates(6), (0.001, -0.0005));
    assert_eq!(get_fee_rates(7), (0.0022, -0.0003));
}

#[test]
fn test_get_fee_tier() {
    assert_eq!(get_fee_tier(1.5, 0.0), 6);
    assert_eq!(get_fee_tier(0.0, 1_000_001.0), 5);
    assert_eq!(get_fee_tier(0.0, 100_001.0), 4);
    assert_eq!(get_fee_tier(0.0, 10_001.0), 3);
    assert_eq!(get_fee_tier(0.0, 1_001.0), 2);
    assert_eq!(get_fee_tier(0.0, 101.0), 1);
    assert_eq!(get_fee_tier(0.0, 0.0), 0);
}
