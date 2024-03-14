/// The entry point for the OpenBook CLI application.
///
/// # Returns
///
/// Returns `Ok(())` on success or an error if an error occurs.|
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "cli")]
    {
        use clap::Parser;
        use openbook::cli::{Cli, Commands};
        use openbook::market::Market;
        use openbook::rpc_client::RpcClient;
        use openbook::utils::read_keypair;
        use solana_sdk::commitment_config::CommitmentConfig;

        let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set");
        let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set");

        let args = Cli::parse();

        let commitment_config = CommitmentConfig::confirmed();
        let rpc_client = RpcClient::new_with_commitment(rpc_url, commitment_config);

        let keypair = read_keypair(&key_path);

        let mut market = Market::new(rpc_client, 3, "openbook", keypair).await;

        match args.command {
            Some(Commands::Info(_)) => {
                println!("[*] Market Info: {:?}", market);
            }
            Some(Commands::Place(arg)) => {
                let r = market.place_limit_bid(arg.bid).await?;
                println!("[*] Transaction successful, signature: {:?}", r);
            }
            Some(Commands::Cancel(arg)) => {
                let c = market.cancel_order(arg.order).await?;
                println!("[*] Transaction successful, signature: {:?}", c);
            }
            Some(Commands::Settle(_arg)) => {
                let s = market.settle_balance().await?;
                println!("[*] Transaction successful, signature: {:?}", s);
            }
            Some(Commands::Match(arg)) => {
                let m = market.make_match_orders_transaction(arg.order).await?;
                println!("[*] Transaction successful, signature: {:?}", m);
            }
            Some(Commands::Consume(arg)) => {
                let e = market
                    .make_consume_events_instruction(Vec::new(), arg.limit)
                    .await?;
                println!("[*] Transaction successful, signature: {:?}", e);
            }
            Some(Commands::ConsumePermissioned(arg)) => {
                let p = market
                    .make_consume_events_permissioned_instruction(Vec::new(), arg.limit)
                    .await?;
                println!("[*] Transaction successful, signature: {:?}", p);
            }
            Some(Commands::Load(arg)) => {
                let l = market.load_orders_for_owner(arg.num).await?;
                println!("[*] Found Program Accounts: {:?}", l);
            }
            Some(Commands::Find(_arg)) => {
                // Todo: Handle Find Open Accounts command
                println!("[*] Find Open Accounts Results: Placeholder");
            }
            None => println!(
                "\x1b[1;91m{}\x1b[0m",
                "Unknown command. Use '--help' for usage instructions."
            ),
        };
    }
    Ok(())
}
