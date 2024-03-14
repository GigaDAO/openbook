use openbook::market::Market;
use openbook::rpc_client::RpcClient;
use openbook::utils::read_keypair;

#[tokio::test]
async fn test_market_state_info() {
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");

    let rpc_client = RpcClient::new(rpc_url);

    let keypair = read_keypair(&key_path);

    let market = Market::new(rpc_client, 3, "openbook", keypair).await;

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
}
