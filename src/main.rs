use anyhow::Result;
use tracing::{error, info};

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
        use openbook::matching::Side;
        use openbook::tokens_and_markets::{DexVersion, Token};
        use openbook::tui::run_tui;
        #[cfg(feature = "v1")]
        use openbook::v1::{ob_client::OBClient as OBV1Client, orders::OrderReturnType};
        #[cfg(feature = "v2")]
        use openbook::v2::ob_client::OBClient as OBV2Client;
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
        let mut ob_client;

        if args.command == Some(Commands::Tui) {
            ob_client = OBV1Client::new(
                CommitmentConfig::confirmed(),
                DexVersion::default(),
                Token::JLP,
                Token::USDC,
                false,
                123456789,
            )
            .await?;
        } else {
            ob_client = OBV1Client::new(
                CommitmentConfig::confirmed(),
                DexVersion::default(),
                Token::JLP,
                Token::USDC,
                true,
                123456789,
            )
            .await?;
        }

        // Todo: ob v2 client
        match args.command {
            Some(Commands::Info(_)) => {
                info!("\n[*] Market Info: {:?}", ob_client);
            }
            Some(Commands::Place(arg)) => {
                let side = match arg.side.as_str() {
                    "bid" => Side::Bid,
                    "ask" => Side::Ask,
                    _ => Side::Bid,
                };

                if let Some(ord_ret_type) = ob_client
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
                            info!("\n[*] Got Instructions: {:?}", insts);
                        }
                        OrderReturnType::Signature(signature) => {
                            info!("\n[*] Transaction successful, signature: {:?}", signature);
                            match ob_client.rpc_client.fetch_transaction(&signature).await {
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
                                Err(err) => error!(
                                    "[*] Unable to get confirmed transaction details: {}",
                                    err
                                ),
                            }
                        }
                    }
                }
            }
            Some(Commands::Cancel(arg)) => {
                if let Some(ord_ret_type) = ob_client.cancel_orders(arg.execute).await? {
                    match ord_ret_type {
                        OrderReturnType::Instructions(insts) => {
                            info!("\n[*] Got Instructions: {:?}", insts);
                        }
                        OrderReturnType::Signature(signature) => {
                            info!("\n[*] Transaction successful, signature: {:?}", signature);
                            match ob_client.rpc_client.fetch_transaction(&signature).await {
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
                                Err(err) => error!(
                                    "[*] Unable to get confirmed transaction details: {}",
                                    err
                                ),
                            }
                        }
                    }
                }
            }
            Some(Commands::Settle(arg)) => {
                if let Some(ord_ret_type) = ob_client.settle_balance(arg.execute).await? {
                    match ord_ret_type {
                        OrderReturnType::Instructions(insts) => {
                            info!("\n[*] Got Instructions: {:?}", insts);
                        }
                        OrderReturnType::Signature(signature) => {
                            info!("\n[*] Transaction successful, signature: {:?}", signature);
                            match ob_client.rpc_client.fetch_transaction(&signature).await {
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
                                Err(err) => error!(
                                    "[*] Unable to get confirmed transaction details: {}",
                                    err
                                ),
                            }
                        }
                    }
                }
            }
            Some(Commands::Match(arg)) => {
                let signature = ob_client.make_match_orders_transaction(arg.limit).await?;
                info!("\n[*] Transaction successful, signature: {:?}", signature);
                match ob_client.rpc_client.fetch_transaction(&signature).await {
                    Ok(confirmed_transaction) => {
                        info!("\n{:?}", confirmed_transaction);
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
                        error!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::CancelSettlePlace(arg)) => {
                let signature = ob_client
                    .cancel_settle_place(
                        arg.usdc_ask_target,
                        arg.target_usdc_bid,
                        arg.price_jlp_usdc_bid,
                        arg.ask_price_jlp_usdc,
                    )
                    .await?;
                info!("\n[*] Transaction successful, signature: {:?}", signature);
                match ob_client
                    .rpc_client
                    .fetch_transaction(&signature.ok_or("")?)
                    .await
                {
                    Ok(confirmed_transaction) => {
                        info!("\n{:?}", confirmed_transaction);
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
                        error!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::CancelSettlePlaceBid(arg)) => {
                let signature = ob_client
                    .cancel_settle_place_bid(arg.target_size_usdc_bid, arg.bid_price_jlp_usdc)
                    .await?;
                info!("\n[*] Transaction successful, signature: {:?}", signature);
                match ob_client
                    .rpc_client
                    .fetch_transaction(&signature.ok_or("")?)
                    .await
                {
                    Ok(confirmed_transaction) => {
                        info!("\n{:?}", confirmed_transaction);
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
                        error!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::CancelSettlePlaceAsk(arg)) => {
                let signature = ob_client
                    .cancel_settle_place_ask(arg.target_size_usdc_ask, arg.ask_price_jlp_usdc)
                    .await?;
                info!("\n[*] Transaction successful, signature: {:?}", signature);
                match ob_client
                    .rpc_client
                    .fetch_transaction(&signature.ok_or("")?)
                    .await
                {
                    Ok(confirmed_transaction) => {
                        info!("\n{:?}", confirmed_transaction);
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
                        error!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::Consume(arg)) => {
                let signature = ob_client
                    .make_consume_events_instruction(Vec::new(), arg.limit)
                    .await?;
                info!("\n[*] Transaction successful, signature: {:?}", signature);
                match ob_client.rpc_client.fetch_transaction(&signature).await {
                    Ok(confirmed_transaction) => {
                        info!("\n{:?}", confirmed_transaction);
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
                        error!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::ConsumePermissioned(arg)) => {
                let signature = ob_client
                    .make_consume_events_permissioned_instruction(Vec::new(), arg.limit)
                    .await?;
                info!("\n[*] Transaction successful, signature: {:?}", signature);
                match ob_client.rpc_client.fetch_transaction(&signature).await {
                    Ok(confirmed_transaction) => {
                        info!("\n{:?}", confirmed_transaction);
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
                        error!("[*] Unable to get confirmed transaction details: {}", err)
                    }
                }
            }
            Some(Commands::Load(_arg)) => {
                let l = ob_client.load_orders_for_owner().await?;
                info!("\n[*] Found Program Accounts: {:?}", l);
            }
            Some(Commands::Find(_arg)) => {
                // Todo: Decode accounts data
                let result = ob_client
                    .find_open_orders_accounts_for_owner(ob_client.open_orders.oo_key, 1000)
                    .await?;
                info!("\n[*] Found Open Orders Accounts: {:?}", result);
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
