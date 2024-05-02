use openbook::market::Market;
use openbook::rpc_client::RpcClient;
use openbook::utils::read_keypair;

#[tokio::test]
async fn test_market_state_info() {
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");

    let rpc_client = RpcClient::new(rpc_url);

    let keypair = read_keypair(&key_path);

    let market = Market::new(rpc_client, 3, "jlp", "usdc", keypair, true).await;

    assert_eq!(
        &market.program_id.to_string(),
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"
    );
    assert_eq!(
        &market.market_address.to_string(),
        "ASUyMMNBpFzpW3zDSPYdDVggKajq1DMKFFPK1JS9hoSR"
    );
    assert_eq!(
        &market.event_queue.to_string(),
        "FM1a4He7jBDBQXfbUK35xpwf6tx2DfRYAzX48AkVcNqP"
    );
    assert_eq!(
        &market.request_queue.to_string(),
        "7oGLtLJbcaTWQprDoYyCBTUW5n598qYRQP6KKw5DML4L"
    );
    assert_eq!(
        &market.market_info.bids_address.to_string(),
        "E9jHtpUqgTF2Ln8UhmyRXRNJsGKuNMVaSVaGowk9Vvr6"
    );
    assert_eq!(
        &market.market_info.asks_address.to_string(),
        "6Kus1PbGpDRZ8R57PG2UM5b5vmyMp9wAHsXzsFQfPzsZ"
    );
    assert_eq!(
        &market.quote_ata.to_string(),
        "8onULHc8pHT7N7XnVfbkkLeU8mqEHnQbGwiBnTdVESio"
    );
    assert_eq!(
        &market.base_ata.to_string(),
        "4P8mfc9dP7MxD5Uq9T5nxHD4GduVtCiKWYu8Nted8cXg"
    );

    assert_eq!(market.coin_decimals, 9);

    assert_eq!(market.pc_decimals, 6);

    assert_eq!(market.coin_lot_size, 100000);

    assert_eq!(market.pc_lot_size, 10);
}
