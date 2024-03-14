use openbook::market::Market;
use openbook::rpc_client::RpcClient;
use openbook::utils::read_keypair;

#[tokio::test]
async fn test_market_state_info() {
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");

    let rpc_client = RpcClient::new(rpc_url);

    let keypair = read_keypair(&key_path);

    let market = Market::new(rpc_client, 3, "usdc", keypair).await;

    assert_eq!(
        &market.program_id.to_string(),
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"
    );
    assert_eq!(
        &market.market_address.to_string(),
        "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6"
    );
    assert_eq!(
        &market.event_queue.to_string(),
        "8CvwxZ9Db6XbLD46NZwwmVDZZRDy7eydFcAGkXKh9axa"
    );
    assert_eq!(
        &market.request_queue.to_string(),
        "CPjXDcggXckEq9e4QeXUieVJBpUNpLEmpihLpg5vWjGF"
    );
    assert_eq!(
        &market.market_info.bids_address.to_string(),
        "5jWUncPNBMZJ3sTHKmMLszypVkoRK6bfEQMQUHweeQnh"
    );
    assert_eq!(
        &market.market_info.asks_address.to_string(),
        "EaXdHx7x3mdGA38j5RSmKYSXMzAFzzUXCLNBEDXDn1d5"
    );
    assert_eq!(
        &market.usdc_ata.to_string(),
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );
    assert_eq!(
        &market.wsol_ata.to_string(),
        "So11111111111111111111111111111111111111112"
    );

    // Additional Details

    assert_eq!(market.coin_decimals, 9);

    // Base Decimals
    assert_eq!(market.pc_decimals, 6);

    // Base Lot Size
    assert_eq!(market.coin_lot_size, 1000000);

    assert_eq!(market.pc_lot_size, 1);
}
