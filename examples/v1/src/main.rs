use openbook::v1::orders::OrderReturnType;
use openbook::v1::ob_client::OBClient;
use openbook::matching::Side;
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
        true,
        1000
    ).await?;

    println!("Initialized OpenBook V1 Client: {:?}", ob_client);

    println!("[*] Place Limit Order");
    if let Some(ord_ret_type) = ob_client
        .place_limit_order(
            0.1,
            Side::Bid, // or Side::Ask
            0.1,
            true,
            2.0,
        )
        .await?
    {
        match ord_ret_type {
            OrderReturnType::Instructions(insts) => {
                println!("[*] Got Instructions: {:?}", insts);
            }
            OrderReturnType::Signature(sign) => {
                println!("[*] Transaction successful, signature: {:?}", sign);
            }
        }
    }

    println!("[*] Cancel Orders");
    if let Some(ord_ret_type) = ob_client
        .cancel_orders(
            true
        )
        .await?
    {
        match ord_ret_type {
            OrderReturnType::Instructions(insts) => {
                println!("[*] Got Instructions: {:?}", insts);
            }
            OrderReturnType::Signature(sign) => {
                println!("[*] Transaction successful, signature: {:?}", sign);
            }
        }
    }

    println!("[*] Settle Balance");
    if let Some(ord_ret_type) = ob_client
        .settle_balance(
            true
        )
        .await?
    {
        match ord_ret_type {
            OrderReturnType::Instructions(insts) => {
                println!("[*] Got Instructions: {:?}", insts);
            }
            OrderReturnType::Signature(sign) => {
                println!("[*] Transaction successful, signature: {:?}", sign);
            }
        }
    }

    println!("[*] Cancel Settle Place Order");
    let result = ob_client
        .cancel_settle_place(
            10.0,
            0.5,
            15.0,
            1.3,
        )
        .await?;
    println!("[*] Transaction successful, signature: {:?}", result);

    println!("[*] Cancel Settle Place Bid Order");
    let result = ob_client
        .cancel_settle_place_bid(0.5, 15.0)
        .await?;
    println!("[*] Transaction successful, signature: {:?}", result);

    println!("[*] Cancel Settle Ask Order");
    let result = ob_client
        .cancel_settle_place_ask(0.5, 15.0)
        .await?;
    println!("[*] Transaction successful, signature: {:?}", result);

    let m = ob_client.match_orders_transaction(1).await?;
    println!("Match Order Result: {:?}", m);

    let open_orders_accounts = vec![ob_client.open_orders.oo_key];
    let limit = 10;

    let e = ob_client.consume_events_instruction(open_orders_accounts.clone(), limit).await?;
    println!("Consume Events Result: {:?}", e);

    let p = ob_client.consume_events_permissioned_instruction(open_orders_accounts.clone(), limit).await?;
    println!("Consume Events Permissioned Result: {:?}", p);

    Ok(())
}