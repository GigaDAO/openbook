use anyhow::Result;

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
        use openbook::commitment_config::CommitmentConfig;
        use openbook::market::Market;
        use openbook::market::OrderReturnType;
        use openbook::matching::Side;
        use openbook::rpc_client::RpcClient;
        use openbook::signature::Signer;
        use openbook::tokens_and_markets::{DexVersion, Token};
        use openbook::tui::run_tui;
        use openbook::utils::read_keypair;
        use solana_cli_output::display::println_transaction;
        use tracing_subscriber::{filter, fmt};

        // Start configuring a `fmt` subscriber
        let filter = filter::LevelFilter::INFO;
        let subscriber = fmt()
            .compact()
            .with_max_level(filter)
            .with_file(false)
            .with_line_number(false)
            .with_thread_ids(false)
            .with_target(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;

        let args = Cli::parse();
        let mut market;

        let rpc_url =
            std::env::var("RPC_URL").unwrap_or("https://api.mainnet-beta.solana.com".to_string());
        let key_path = std::env::var("KEY_PATH").unwrap_or("".to_string());

        let commitment_config = CommitmentConfig::confirmed();
        let rpc_client = RpcClient::new_with_commitment(rpc_url, commitment_config);

        let owner = read_keypair(&key_path);

        assert_eq!(rpc_client.commitment(), CommitmentConfig::confirmed());
        if args.command == Some(Commands::Tui) {
            market = Market::new(
                rpc_client,
                DexVersion::default(),
                Token::JLP,
                Token::USDC,
                owner,
                false,
            )
            .await?;
        } else {
            market = Market::new(
                rpc_client,
                DexVersion::default(),
                Token::JLP,
                Token::USDC,
                owner,
                true,
            )
            .await?;
        }

        match args.command {
            Some(Commands::Info(_)) => {
                println!("[*] Market Info: {:?}", market);
            }
            Some(Commands::Place(arg)) => {
                let side = match arg.side.as_str() {
                    "bid" => Side::Bid,
                    "ask" => Side::Ask,
                    _ => Side::Bid,
                };

                if let Some(ord_ret_type) = market
                    .place_limit_order(
                        arg.target_amount_quote,
                        side,
                        arg.best_offset_usdc,
                        arg.execute,
                        arg.price_target,
                    )
                    .await?
                {
                    match ord_ret_type {
                        OrderReturnType::Instructions(insts) => {
                            println!("[*] Got Instructions: {:?}", insts);
                        }
                        OrderReturnType::Signature(signature) => {
                            println!("[*] Transaction successful, signature: {:?}", signature);
                            match market.rpc_client.fetch_transaction(&signature).await {
                                Ok(confirmed_transaction) => {
                                    println_transaction(
                                        &confirmed_transaction
                                            .transaction
                                            .transaction
                                            .decode()
                                            .expect("Successful decode"),
                                        confirmed_transaction.transaction.meta.as_ref(),
                                        "  ",
                                        None,
                                        None,
                                    );
                                }
                                Err(err) => eprintln!(
                                    "[*] Unable to get confirmed transaction details: {}",
                                    err
                                ),
                            }
                        }
                    }
                }
            }
            Some(Commands::Cancel(arg)) => {
                if let Some(ord_ret_type) = market.cancel_orders(arg.execute).await? {
                    match ord_ret_type {
                        OrderReturnType::Instructions(insts) => {
                            println!("[*] Got Instructions: {:?}", insts);
                        }
                        OrderReturnType::Signature(signature) => {
                            println!("[*] Transaction successful, signature: {:?}", signature);
                            match market.rpc_client.fetch_transaction(&signature).await {
                                Ok(confirmed_transaction) => {
                                    println_transaction(
                                        &confirmed_transaction
                                            .transaction
                                            .transaction
                                            .decode()
                                            .expect("Successful decode"),
                                        confirmed_transaction.transaction.meta.as_ref(),
                                        "  ",
                                        None,
                                        None,
                                    );
                                }
                                Err(err) => eprintln!(
                                    "[*] Unable to get confirmed transaction details: {}",
                                    err
                                ),
                            }
                        }
                    }
                }
            }
            Some(Commands::Settle(arg)) => {
                if let Some(ord_ret_type) = market.settle_balance(arg.execute).await? {
                    match ord_ret_type {
                        OrderReturnType::Instructions(insts) => {
                            println!("[*] Got Instructions: {:?}", insts);
                        }
                        OrderReturnType::Signature(signature) => {
                            println!("[*] Transaction successful, signature: {:?}", signature);
                            match market.rpc_client.fetch_transaction(&signature).await {
                                Ok(confirmed_transaction) => {
                                    println_transaction(
                                        &confirmed_transaction
                                            .transaction
                                            .transaction
                                            .decode()
                                            .expect("Successful decode"),
                                        confirmed_transaction.transaction.meta.as_ref(),
                                        "  ",
                                        None,
                                        None,
                                    );
                                }
                                Err(err) => eprintln!(
                                    "[*] Unable to get confirmed transaction details: {}",
                                    err
                                ),
                            }
                        }
                    }
                }
            }
            Some(Commands::Match(arg)) => {
                let signature = market.make_match_orders_transaction(arg.limit).await?;
                println!("[*] Transaction successful, signature: {:?}", signature);
                match market.rpc_client.fetch_transaction(&signature).await {
                    Ok(confirmed_transaction) => {
                        println!("{:?}", confirmed_transaction);
                        println_transaction(
                            &confirmed_transaction
                                .transaction
                                .transaction
                                .decode()
                                .expect("Successful decode"),
                            confirmed_transaction.transaction.meta.as_ref(),
                            "  ",
                            None,
                            None,
                        );
                    }
                    Err(err) => {
                        eprintln!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::CancelSettlePlace(arg)) => {
                let signature = market
                    .cancel_settle_place(
                        arg.usdc_ask_target,
                        arg.target_usdc_bid,
                        arg.price_jlp_usdc_bid,
                        arg.ask_price_jlp_usdc,
                    )
                    .await?;
                println!("[*] Transaction successful, signature: {:?}", signature);
                match market
                    .rpc_client
                    .fetch_transaction(&signature.ok_or("")?)
                    .await
                {
                    Ok(confirmed_transaction) => {
                        println!("{:?}", confirmed_transaction);
                        println_transaction(
                            &confirmed_transaction
                                .transaction
                                .transaction
                                .decode()
                                .expect("Successful decode"),
                            confirmed_transaction.transaction.meta.as_ref(),
                            "  ",
                            None,
                            None,
                        );
                    }
                    Err(err) => {
                        eprintln!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::CancelSettlePlaceBid(arg)) => {
                let signature = market
                    .cancel_settle_place_bid(arg.target_size_usdc_bid, arg.bid_price_jlp_usdc)
                    .await?;
                println!("[*] Transaction successful, signature: {:?}", signature);
                match market
                    .rpc_client
                    .fetch_transaction(&signature.ok_or("")?)
                    .await
                {
                    Ok(confirmed_transaction) => {
                        println!("{:?}", confirmed_transaction);
                        println_transaction(
                            &confirmed_transaction
                                .transaction
                                .transaction
                                .decode()
                                .expect("Successful decode"),
                            confirmed_transaction.transaction.meta.as_ref(),
                            "  ",
                            None,
                            None,
                        );
                    }
                    Err(err) => {
                        eprintln!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::CancelSettlePlaceAsk(arg)) => {
                let signature = market
                    .cancel_settle_place_ask(arg.target_size_usdc_ask, arg.ask_price_jlp_usdc)
                    .await?;
                println!("[*] Transaction successful, signature: {:?}", signature);
                match market
                    .rpc_client
                    .fetch_transaction(&signature.ok_or("")?)
                    .await
                {
                    Ok(confirmed_transaction) => {
                        println!("{:?}", confirmed_transaction);
                        println_transaction(
                            &confirmed_transaction
                                .transaction
                                .transaction
                                .decode()
                                .expect("Successful decode"),
                            confirmed_transaction.transaction.meta.as_ref(),
                            "  ",
                            None,
                            None,
                        );
                    }
                    Err(err) => {
                        eprintln!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::Consume(arg)) => {
                let signature = market
                    .make_consume_events_instruction(Vec::new(), arg.limit)
                    .await?;
                println!("[*] Transaction successful, signature: {:?}", signature);
                match market.rpc_client.fetch_transaction(&signature).await {
                    Ok(confirmed_transaction) => {
                        println!("{:?}", confirmed_transaction);
                        println_transaction(
                            &confirmed_transaction
                                .transaction
                                .transaction
                                .decode()
                                .expect("Successful decode"),
                            confirmed_transaction.transaction.meta.as_ref(),
                            "  ",
                            None,
                            None,
                        );
                    }
                    Err(err) => {
                        eprintln!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::ConsumePermissioned(arg)) => {
                let signature = market
                    .make_consume_events_permissioned_instruction(Vec::new(), arg.limit)
                    .await?;
                println!("[*] Transaction successful, signature: {:?}", signature);
                match market.rpc_client.fetch_transaction(&signature).await {
                    Ok(confirmed_transaction) => {
                        println!("{:?}", confirmed_transaction);
                        println_transaction(
                            &confirmed_transaction
                                .transaction
                                .transaction
                                .decode()
                                .expect("Successful decode"),
                            confirmed_transaction.transaction.meta.as_ref(),
                            "  ",
                            None,
                            None,
                        );
                    }
                    Err(err) => {
                        eprintln!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::Load(_arg)) => {
                let l = market.load_orders_for_owner().await?;
                println!("[*] Found Program Accounts: {:?}", l);
            }
            Some(Commands::Find(_arg)) => {
                // Todo: Decode accounts data
                let result = market
                    .find_open_orders_accounts_for_owner(&market.owner.pubkey(), 5000)
                    .await?;
                println!("[*] Found Open Orders Accounts: {:?}", result);
            }
            Some(Commands::Tui) => {
                let _ = run_tui().await;
            }
            None => println!(
                "\x1b[1;91m{}\x1b[0m",
                "Unknown command. Use '--help' for usage instructions."
            ),
        };
    }
    Ok(())
}
