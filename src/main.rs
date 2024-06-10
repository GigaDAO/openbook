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
        use openbook::cli::{Cli, Commands, V1ActionsCommands, V2ActionsCommands};
        use openbook::commitment_config::CommitmentConfig;
        use openbook::matching::Side;
        use tokio::time::{sleep, Duration};

        use openbook::tui::run_tui;
        #[cfg(feature = "v2")]
        use openbook::tui::SdkVersion;
        #[cfg(feature = "v1")]
        use openbook::v1::{ob_client::OBClient as OBV1Client, orders::OrderReturnType};
        #[cfg(feature = "v2")]
        use openbook::v2::ob_client::OBClient as OBV2Client;
        use openbook::v2_state::Side as V2Side;
        use solana_cli_output::display::println_transaction;
        use tracing::{error, info};
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

        const CRANK_DELAY_MS: u64 = 50_000;

        match args.command {
            Some(Commands::V1(cmd)) => {
                let mut ob_client_v1 = OBV1Client::new(
                    CommitmentConfig::confirmed(),
                    cmd.market_id.parse().unwrap(),
                    true,
                    123456789,
                )
                .await?;
                match cmd.command {
                    Some(V1ActionsCommands::Info(_)) => {
                        info!("\n[*] {:?}", ob_client_v1);
                    }
                    Some(V1ActionsCommands::Place(arg)) => {
                        let side = match arg.side.to_ascii_lowercase().as_str() {
                            "bid" => Side::Bid,
                            "ask" => Side::Ask,
                            _ => Side::Bid,
                        };

                        if let Some(ord_ret_type) = ob_client_v1
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
                                    info!(
                                        "\n[*] Transaction successful, signature: {:?}",
                                        signature
                                    );
                                    // wait for the tx to be cranked
                                    sleep(Duration::from_millis(CRANK_DELAY_MS)).await;
                                    match ob_client_v1
                                        .rpc_client
                                        .fetch_transaction(&signature)
                                        .await
                                    {
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
                    Some(V1ActionsCommands::Cancel(arg)) => {
                        if let Some(ord_ret_type) = ob_client_v1.cancel_orders(arg.execute).await? {
                            match ord_ret_type {
                                OrderReturnType::Instructions(insts) => {
                                    info!("\n[*] Got Instructions: {:?}", insts);
                                }
                                OrderReturnType::Signature(signature) => {
                                    info!(
                                        "\n[*] Transaction successful, signature: {:?}",
                                        signature
                                    );
                                    // wait for the tx to be cranked
                                    sleep(Duration::from_millis(CRANK_DELAY_MS)).await;
                                    match ob_client_v1
                                        .rpc_client
                                        .fetch_transaction(&signature)
                                        .await
                                    {
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
                    Some(V1ActionsCommands::Settle(arg)) => {
                        if let Some(ord_ret_type) = ob_client_v1.settle_balance(arg.execute).await?
                        {
                            match ord_ret_type {
                                OrderReturnType::Instructions(insts) => {
                                    info!("\n[*] Got Instructions: {:?}", insts);
                                }
                                OrderReturnType::Signature(signature) => {
                                    info!(
                                        "\n[*] Transaction successful, signature: {:?}",
                                        signature
                                    );
                                    // wait for the tx to be cranked
                                    sleep(Duration::from_millis(CRANK_DELAY_MS)).await;
                                    match ob_client_v1
                                        .rpc_client
                                        .fetch_transaction(&signature)
                                        .await
                                    {
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
                    Some(V1ActionsCommands::Match(arg)) => {
                        let (_confirmed, signature) =
                            ob_client_v1.match_orders_transaction(arg.limit).await?;
                        info!("\n[*] Transaction successful, signature: {:?}", signature);
                        // wait for the tx to be cranked
                        sleep(Duration::from_millis(CRANK_DELAY_MS)).await;
                        match ob_client_v1.rpc_client.fetch_transaction(&signature).await {
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
                    Some(V1ActionsCommands::CancelSettlePlace(arg)) => {
                        let (_confirmed, signature) = ob_client_v1
                            .cancel_settle_place(
                                arg.usdc_ask_target,
                                arg.target_usdc_bid,
                                arg.price_jlp_usdc_bid,
                                arg.ask_price_jlp_usdc,
                            )
                            .await?;
                        info!("\n[*] Transaction successful, signature: {:?}", signature);
                        // wait for the tx to be cranked
                        sleep(Duration::from_millis(CRANK_DELAY_MS)).await;
                        match ob_client_v1.rpc_client.fetch_transaction(&signature).await {
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
                    Some(V1ActionsCommands::CancelSettlePlaceBid(arg)) => {
                        let (_confirmed, signature) = ob_client_v1
                            .cancel_settle_place_bid(
                                arg.target_size_usdc_bid,
                                arg.bid_price_jlp_usdc,
                            )
                            .await?;
                        info!("\n[*] Transaction successful, signature: {:?}", signature);
                        // wait for the tx to be cranked
                        sleep(Duration::from_millis(CRANK_DELAY_MS)).await;
                        match ob_client_v1.rpc_client.fetch_transaction(&signature).await {
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
                    Some(V1ActionsCommands::CancelSettlePlaceAsk(arg)) => {
                        let (_confirmed, signature) = ob_client_v1
                            .cancel_settle_place_ask(
                                arg.target_size_usdc_ask,
                                arg.ask_price_jlp_usdc,
                            )
                            .await?;
                        info!("\n[*] Transaction successful, signature: {:?}", signature);
                        // wait for the tx to be cranked
                        sleep(Duration::from_millis(CRANK_DELAY_MS)).await;
                        match ob_client_v1.rpc_client.fetch_transaction(&signature).await {
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
                    Some(V1ActionsCommands::Consume(arg)) => {
                        let (_confirmed, signature) = ob_client_v1
                            .consume_events_instruction(Vec::new(), arg.limit)
                            .await?;
                        info!("\n[*] Transaction successful, signature: {:?}", signature);
                        // wait for the tx to be cranked
                        sleep(Duration::from_millis(CRANK_DELAY_MS)).await;
                        match ob_client_v1.rpc_client.fetch_transaction(&signature).await {
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
                    Some(V1ActionsCommands::ConsumePermissioned(arg)) => {
                        let (_confirmed, signature) = ob_client_v1
                            .consume_events_permissioned_instruction(Vec::new(), arg.limit)
                            .await?;
                        info!("\n[*] Transaction successful, signature: {:?}", signature);
                        // wait for the tx to be cranked
                        sleep(Duration::from_millis(CRANK_DELAY_MS)).await;
                        match ob_client_v1.rpc_client.fetch_transaction(&signature).await {
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
                    Some(V1ActionsCommands::Load(_arg)) => {
                        let l = ob_client_v1.load_orders_for_owner().await?;
                        info!("\n[*] Found Program Accounts: {:?}", l);
                    }
                    Some(V1ActionsCommands::Find(_arg)) => {
                        let result = ob_client_v1
                            .find_open_orders_accounts_for_owner(
                                ob_client_v1.open_orders.oo_key,
                                1000,
                            )
                            .await?;
                        info!("\n[*] Found Open Orders Accounts: {:?}", result);
                    }
                    None => {
                        let _ = run_tui(SdkVersion::V1).await;
                    }
                }
            }
            Some(Commands::V2(cmd)) => {
                let mut ob_client_v2 = OBV2Client::new(
                    CommitmentConfig::confirmed(),
                    cmd.market_id.parse().unwrap(),
                    false,
                    true,
                )
                .await?;

                match cmd.command {
                    Some(V2ActionsCommands::Info(_)) => {
                        info!("\n[*] {:?}", ob_client_v2);
                    }
                    Some(V2ActionsCommands::Place(arg)) => {
                        let side = match arg.side.as_str() {
                            "bid" => V2Side::Bid,
                            "ask" => V2Side::Ask,
                            _ => V2Side::Bid,
                        };

                        let (_confirmed, signature, _order_id, _slot) = ob_client_v2
                            .place_limit_order(
                                arg.price_target,
                                arg.target_amount_quote as u64,
                                side,
                            )
                            .await?;
                        info!("\n[*] Transaction successful, signature: {:?}", signature);
                        // wait for the tx to be cranked
                        sleep(Duration::from_millis(CRANK_DELAY_MS)).await;
                        match ob_client_v2.rpc_client.fetch_transaction(&signature).await {
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
                    None => {
                        let _ = run_tui(SdkVersion::V2).await;
                    }
                }
            }
            None => {
                // default is OpenBook V2
                let _ = run_tui(SdkVersion::V2).await;
            }
        };
    }
    Ok(())
}
