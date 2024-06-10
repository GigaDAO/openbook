use openbook::v2::ob_client::OBClient;
use openbook::v2_state::Side;
use openbook::commitment_config::CommitmentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let commitment = CommitmentConfig::confirmed();

    let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6"
        .parse()
        .unwrap();

    let mut ob_client = OBClient::new(
        commitment,
        market_id,
        true, // Create new indexer and open orders accounts
        true // load all info related to the open orders account
    ).await?;

    println!("Initialized OpenBook V1 Client: {:?}", ob_client);

    println!("[*] Place Limit Order");

    let (_confirmed, signature, _order_id, _slot) = ob_client
        .place_limit_order(
            0.003, // price
            5, // amount
            Side::Bid,
        )
        .await?;

    println!("[*] Transaction successful, signature: {:?}", signature);

    Ok(())
}