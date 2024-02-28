use openbook_v1_sdk::market::read_keypair;
use openbook_v1_sdk::market::Market;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    let market_address = std::env::var("SOL_USDC_MARKET_ID")
        .expect("SOL_USDC_MARKET_ID is not set in .env file")
        .parse()
        .unwrap();
    let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
        .expect("OPENBOOK_V1_PROGRAM_ID is not set in .env file")
        .parse()
        .unwrap();
    let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set in .env file");
    let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set in .env file");
    let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set in .env file");

    let rpc_client = RpcClient::new(rpc_url);
    let keypair = read_keypair(&key_path);

    let market = Market::new(rpc_client, program_id, market_address, keypair);
    println!("Market Info: {:?}", market);

    let max_bid = 1;
    let r = market.place_limit_bid(max_bid)?;
    println!("Place Order Results: {:?}", r);

    let order_id_to_cancel = 2;
    let c = market.cancel_order(order_id_to_cancel)?;
    println!("Cancel Order Results: {:?}", c);

    let s = market.settle_balance()?;
    println!("Settle Balance Results: {:?}", s);

    let m = market.make_match_orders_transaction(1)?;
    println!("Make Match Order Results: {:?}", m);

    let m = market.make_match_orders_transaction(1)?;
    println!("Make Match Order Results: {:?}", m);

    let open_orders_accounts = vec![Pubkey::new_from_array([0; 32])];
    let limit = 10;

    let e = market.make_consume_events_instruction(open_orders_accounts.clone(), limit)?;
    println!("Make Consume Events Results: {:?}", e);

    let p =
        market.make_consume_events_permissioned_instruction(open_orders_accounts.clone(), limit)?;
    println!("Make Consume Events Permissioned Results: {:?}", p);

    Ok(())
}
