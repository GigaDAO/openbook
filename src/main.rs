/// The entry point for the OpenBook CLI application.
///
/// # Returns
///
/// Returns `Ok(())` on success or an error if an error occurs.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "cli")]
    {
        use clap::Parser;
        use openbook::cli::{Cli, Commands};
        use openbook::market::Market;
        use openbook::rpc_client::RpcClient;
        use openbook::utils::read_keypair;

        let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set");
        let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set");
        let market_address = std::env::var("SOL_USDC_MARKET_ID")
            .expect("SOL_USDC_MARKET_ID is not set")
            .parse()
            .unwrap();
        let program_id = std::env::var("OPENBOOK_V1_PROGRAM_ID")
            .expect("OPENBOOK_V1_PROGRAM_ID is not set")
            .parse()
            .unwrap();
        let _usdc_ata_str = std::env::var("USDC_ATA").expect("USDC_ATA is not set");
        let _wsol_ata_str = std::env::var("WSOL_ATA").expect("WSOL_ATA is not set");
        let _oos_key_str = std::env::var("OOS_KEY").expect("OOS_KEY is not set");

        let args = Cli::parse();

        let rpc_client = RpcClient::new(rpc_url);
        let keypair = read_keypair(&key_path);

        let mut market = Market::new(rpc_client, program_id, market_address, keypair);

        match args.command {
            Some(Commands::Place(arg)) => {
                let r = market.place_limit_bid(arg.bid)?;
                println!("Market Info: {:?}", r);
            }
            Some(Commands::Cancel(arg)) => {
                let c = market.cancel_order(arg.order)?;
                println!("Cancel Order Results: {:?}", c);
            }
            Some(Commands::Settle(_arg)) => {
                let s = market.settle_balance()?;
                println!("Settle Balance Results: {:?}", s);
            }
            Some(Commands::Match(arg)) => {
                let m = market.make_match_orders_transaction(arg.order)?;
                println!("Match Order Results: {:?}", m);
            }
            Some(Commands::Consume(arg)) => {
                let e = market.make_consume_events_instruction(Vec::new(), arg.limit)?;
                println!("Consume Events Results: {:?}", e);
            }
            Some(Commands::ConsumePermissioned(arg)) => {
                let p =
                    market.make_consume_events_permissioned_instruction(Vec::new(), arg.limit)?;
                println!("Consume Events Permissioned Results: {:?}", p);
            }
            Some(Commands::Load(arg)) => {
                let l = market.load_orders_for_owner(arg.num)?;
                println!("Load Orders For Owner Results: {:?}", l);
            }
            Some(Commands::Find(_arg)) => {
                // Todo: Handle Find Open Accounts command
                println!("Find Open Accounts Results: Placeholder");
            }
            None => println!(
                "\x1b[1;91m{}\x1b[0m",
                "Unknown command. Use '--help' for usage instructions."
            ),
        };
    }
    Ok(())
}
