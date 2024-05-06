use openbook::market::Market;
use openbook::rpc::Rpc;
use openbook::rpc_client::RpcClient;
use openbook::tokens_and_markets::{DexVersion, Token};
use openbook::traits::MarketInfo;

#[tokio::test]
async fn test_market_state_info() -> anyhow::Result<(), anyhow::Error> {
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");

    let rpc_client = Rpc::new(RpcClient::new(rpc_url));

    let market = Market::new(
        rpc_client,
        DexVersion::default(),
        Token::JLP,
        Token::USDC,
        true,
    )
    .await?;

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
        &market.bids_address.to_string(),
        "E9jHtpUqgTF2Ln8UhmyRXRNJsGKuNMVaSVaGowk9Vvr6"
    );
    assert_eq!(
        &market.asks_address.to_string(),
        "6Kus1PbGpDRZ8R57PG2UM5b5vmyMp9wAHsXzsFQfPzsZ"
    );
    assert_eq!(
        &market.quote_mint.to_string(),
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );
    assert_eq!(
        &market.base_mint.to_string(),
        "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4"
    );

    assert_eq!(market.coin_decimals, 9);

    assert_eq!(market.pc_decimals, 6);

    assert_eq!(market.coin_lot_size, 100000);

    assert_eq!(market.pc_lot_size, 10);

    Ok(())
}
