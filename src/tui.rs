use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::style::Stylize;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{
        Constraint::{Length, Max, Min, Percentage},
        Direction, Layout,
    },
    style::palette::tailwind,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame, Terminal,
};

use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

use solana_sdk::signature::Signer;
use std::collections::HashMap;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::commitment_config::CommitmentConfig;
#[cfg(feature = "v1")]
use crate::matching::Side as OBV1Side;
use crate::rpc::Rpc;
use crate::rpc_client::RpcClient;
use crate::utils::read_keypair;
#[cfg(feature = "v1")]
use crate::v1::{ob_client::OBClient as OBClientV1, orders::OrderReturnType};
#[cfg(feature = "v2")]
use crate::v2::market::CreateMarketArgs;
#[cfg(feature = "v2")]
use crate::v2::ob_client::OBClient as OBClientV2;

use anyhow::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum SdkVersion {
    /// OpenBook v1.
    V1,
    /// OpenBook v2.
    V2,
}

#[derive(Clone)]
pub enum SdkClient {
    OBClientV1(OBClientV1),
    OBClientV2(OBClientV2),
}

enum InputMode {
    Normal,
    Editing,
}

enum CurrentInput {
    RpcUrl,
    KeyPath,
    MarketID,
    OBV1Side,
    TargetPrice,
    MarketName,
    BaseMint,
    QuoteMint,
    BaseLotSize,
    QuoteLotSize,
    MakerFee,
    TakerFee,
    OracleA,
    OracleB,
    OpenOrdersAdmin,
    CollectFeeAdmin,
    ConsumeEventsAdmin,
    CloseMarketAdmin,
    TimeExpiry,
}

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter)]
enum SelectedTab {
    #[default]
    #[strum(to_string = "🔎 Market Info")]
    Tab1,
    #[strum(to_string = "🛒 Create Market")]
    Tab2,
    #[strum(to_string = "🧮 Lot Calculator")]
    Tab3,
    #[strum(to_string = "📈 Trade")]
    Tab4,
    #[strum(to_string = "🔄 Swap")]
    Tab5,
}

impl SelectedTab {
    /// Get the previous tab, if there is no previous tab return the current tab.
    fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab, if there is no next tab return the current tab.
    fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }
}

impl SelectedTab {
    fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    const fn palette(self) -> tailwind::Palette {
        match self {
            Self::Tab1 => tailwind::BLUE,
            Self::Tab2 => tailwind::YELLOW,
            Self::Tab3 => tailwind::EMERALD,
            Self::Tab4 => tailwind::INDIGO,
            Self::Tab5 => tailwind::SLATE,
        }
    }
}

struct App {
    rpc_url_input: Input,
    key_path_input: Input,
    market_id_input: Input,
    side_input: Input,
    target_price_input: Input,
    input_mode: InputMode,
    transaction_status: String,
    current_input: Option<CurrentInput>,
    market_info: HashMap<String, String>,
    wallet_info: HashMap<String, String>,
    ob_client: Option<SdkClient>,
    selected_tab: SelectedTab,
    market_name: Input,
    market_base_mint: Input,
    market_quote_mint: Input,
    market_base_lot_size: Input,
    market_quote_lot_size: Input,
    market_maker_fee: Input,
    market_taker_fee: Input,
    market_oracle_a: Input,
    market_oracle_b: Input,
    market_open_orders_admin: Input,
    market_collect_fee_admin: Input,
    market_consume_events_admin: Input,
    market_close_market_admin: Input,
    market_time_expiry: Input,
}

impl Default for App {
    fn default() -> App {
        App {
            rpc_url_input: Input::new("https://api.mainnet-beta.solana.com".to_string()),
            key_path_input: Input::new("~/.config/solana/id.json".to_string()),
            market_id_input: Input::default(),
            side_input: Input::default(),
            target_price_input: Input::default(),
            transaction_status: Default::default(),
            input_mode: InputMode::Normal,
            current_input: Some(CurrentInput::RpcUrl),
            market_info: HashMap::new(),
            wallet_info: HashMap::new(),
            ob_client: None,
            selected_tab: Default::default(),
            market_name: Input::default(),
            market_base_mint: Input::default(),
            market_quote_mint: Input::default(),
            market_base_lot_size: Input::default(),
            market_quote_lot_size: Input::default(),
            market_maker_fee: Input::default(),
            market_taker_fee: Input::default(),
            market_oracle_a: Input::default(),
            market_oracle_b: Input::default(),
            market_open_orders_admin: Input::default(),
            market_collect_fee_admin: Input::default(),
            market_consume_events_admin: Input::default(),
            market_close_market_admin: Input::default(),
            market_time_expiry: Input::default(),
        }
    }
}

pub async fn run_tui(version: SdkVersion) -> Result<(), Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app, version).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    version: SdkVersion,
) -> Result<(), Error> {
    loop {
        terminal.draw(|f| ui(f, &mut app, version.clone()))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('e') => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('d') | KeyCode::Right => {
                        app.selected_tab = app.selected_tab.next();
                    }
                    KeyCode::Char('a') | KeyCode::Left => {
                        app.selected_tab = app.selected_tab.previous();
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Enter => {
                        let commitment_config = CommitmentConfig::confirmed();
                        let rpc_client = RpcClient::new_with_commitment(
                            app.rpc_url_input.value().to_string(),
                            commitment_config,
                        );
                        let owner = read_keypair(&app.key_path_input.value().to_string());

                        assert_eq!(rpc_client.commitment(), CommitmentConfig::confirmed());

                        let market_id = app.market_id_input.value().parse()?;

                        if app.ob_client.is_none() {
                            match version {
                                SdkVersion::V1 => {
                                    let mut ob_client = OBClientV1::new(
                                        commitment_config,
                                        market_id,
                                        true,
                                        123456789,
                                    )
                                    .await?;
                                    ob_client.rpc_client = Rpc::new(rpc_client);
                                    ob_client.owner = owner.into();
                                    app.ob_client = Some(SdkClient::OBClientV1(ob_client));
                                }
                                SdkVersion::V2 => {
                                    let mut ob_client =
                                        OBClientV2::new(commitment_config, market_id, false, true)
                                            .await?;
                                    ob_client.rpc_client = Rpc::new(rpc_client);
                                    ob_client.owner = owner.into();
                                    app.ob_client = Some(SdkClient::OBClientV2(ob_client));
                                }
                            }
                        }

                        match app.selected_tab {
                            SelectedTab::Tab1 => match app.ob_client.clone().unwrap() {
                                SdkClient::OBClientV1(ob_client) => {
                                    app.market_info.insert(
                                        "Market Address".to_string(),
                                        ob_client.market_info.market_address.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Base Mint".to_string(),
                                        ob_client.market_info.base_mint.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Quote Mint".to_string(),
                                        ob_client.market_info.quote_mint.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Coin Vault".to_string(),
                                        ob_client.market_info.coin_vault.to_string(),
                                    );
                                    app.market_info.insert(
                                        "PC Vault".to_string(),
                                        ob_client.market_info.pc_vault.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Event Queue".to_string(),
                                        ob_client.market_info.event_queue.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Request Queue".to_string(),
                                        ob_client.market_info.request_queue.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Asks Address".to_string(),
                                        ob_client.market_info.asks_address.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Bids Address".to_string(),
                                        ob_client.market_info.bids_address.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Coin Decimals".to_string(),
                                        ob_client.market_info.coin_decimals.to_string(),
                                    );
                                    app.market_info.insert(
                                        "PC Decimals".to_string(),
                                        ob_client.market_info.pc_decimals.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Coin Lot Size".to_string(),
                                        ob_client.market_info.coin_lot_size.to_string(),
                                    );
                                    app.market_info.insert(
                                        "PC Lot Size".to_string(),
                                        ob_client.market_info.pc_lot_size.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Vault Signer Key".to_string(),
                                        ob_client.market_info.vault_signer_key.to_string(),
                                    );

                                    app.wallet_info.insert(
                                        "Wallet Public Key".to_string(),
                                        ob_client.owner.pubkey().to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Base ATA".to_string(),
                                        ob_client.base_ata.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Quote ATA".to_string(),
                                        ob_client.quote_ata.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Open Order Account".to_string(),
                                        ob_client.open_orders.oo_key.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Min Ask".to_string(),
                                        ob_client.open_orders.min_ask.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Max Bid".to_string(),
                                        ob_client.open_orders.max_bid.to_string(),
                                    );

                                    let open_asks: String = ob_client
                                        .open_orders
                                        .open_asks
                                        .iter()
                                        .map(|&x| x.to_string())
                                        .collect::<Vec<String>>()
                                        .join(", ");
                                    app.wallet_info.insert("Open Asks".to_string(), open_asks);

                                    let open_bids: String = ob_client
                                        .open_orders
                                        .open_bids
                                        .iter()
                                        .map(|&x| x.to_string())
                                        .collect::<Vec<String>>()
                                        .join(", ");
                                    app.wallet_info.insert("Open Bids".to_string(), open_bids);
                                }
                                SdkClient::OBClientV2(ob_client) => {
                                    app.market_info.insert(
                                        "Market Name".to_string(),
                                        ob_client.market_info.name.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Market Address".to_string(),
                                        ob_client.market_id.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Base Decimals".to_string(),
                                        ob_client.market_info.base_decimals.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Quote Decimals".to_string(),
                                        ob_client.market_info.quote_decimals.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Market Authority".to_string(),
                                        ob_client.market_info.market_authority.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Collect Fee Admin".to_string(),
                                        ob_client.market_info.collect_fee_admin.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Event Heap".to_string(),
                                        ob_client.market_info.event_heap.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Base Lot Size".to_string(),
                                        ob_client.market_info.base_lot_size.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Sequence Number".to_string(),
                                        ob_client.market_info.seq_num.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Registration Time".to_string(),
                                        ob_client.market_info.registration_time.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Maker Fee".to_string(),
                                        ob_client.market_info.maker_fee.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Taker Fee".to_string(),
                                        ob_client.market_info.taker_fee.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Maker Volume".to_string(),
                                        ob_client.market_info.maker_volume.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Taker VBolume".to_string(),
                                        ob_client.market_info.taker_volume_wo_oo.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Quote Mint".to_string(),
                                        ob_client.market_info.quote_mint.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Base Vault".to_string(),
                                        ob_client.market_info.market_base_vault.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Quote Vault".to_string(),
                                        ob_client.market_info.market_quote_vault.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Asks Address".to_string(),
                                        ob_client.market_info.asks.to_string(),
                                    );
                                    app.market_info.insert(
                                        "Bids Address".to_string(),
                                        ob_client.market_info.bids.to_string(),
                                    );

                                    app.wallet_info.insert(
                                        "Wallet Public Key".to_string(),
                                        ob_client.owner.pubkey().to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Base ATA".to_string(),
                                        ob_client.base_ata.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Quote ATA".to_string(),
                                        ob_client.quote_ata.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Index Account".to_string(),
                                        ob_client.index_account.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Open Orders Account".to_string(),
                                        ob_client.open_orders_account.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Asks Base in OOs".to_string(),
                                        ob_client.oo_state.asks_base_in_oos.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Bids Base in OOs".to_string(),
                                        ob_client.oo_state.bids_base_in_oos.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Base Free in OOs".to_string(),
                                        ob_client.oo_state.base_free_in_oos.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Quote Free in OOs".to_string(),
                                        ob_client.oo_state.quote_free_in_oos.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Quote Balance".to_string(),
                                        ob_client.ata_balances.quote_balance.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Base Balance".to_string(),
                                        ob_client.ata_balances.base_balance.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Total Balance".to_string(),
                                        ob_client.ata_balances.total_balance.to_string(),
                                    );
                                    app.wallet_info.insert(
                                        "Price".to_string(),
                                        ob_client.ata_balances.price.to_string(),
                                    );

                                    let open_asks: String = ob_client
                                        .open_orders
                                        .iter()
                                        .filter(|x| x.is_buy)
                                        .map(|x| x.price.to_string())
                                        .collect::<Vec<String>>()
                                        .join(", ");
                                    app.wallet_info.insert("Open Asks".to_string(), open_asks);

                                    let open_bids: String = ob_client
                                        .open_orders
                                        .iter()
                                        .filter(|x| x.is_buy)
                                        .map(|x| x.price.to_string())
                                        .collect::<Vec<String>>()
                                        .join(", ");
                                    app.wallet_info.insert("Open Bids".to_string(), open_bids);
                                }
                            },
                            SelectedTab::Tab2 => match app.ob_client.clone().unwrap() {
                                SdkClient::OBClientV1(_ob_client) => {}
                                SdkClient::OBClientV2(ob_client) => {
                                    let market_args = CreateMarketArgs {
                                        name: app.market_name.to_string(),
                                        base_mint: app
                                            .market_base_mint
                                            .to_string()
                                            .parse()
                                            .expect("Invalid Pubkey"),
                                        quote_mint: app
                                            .market_quote_mint
                                            .to_string()
                                            .parse()
                                            .expect("Invalid Pubkey"),
                                        base_lot_size: app
                                            .market_base_lot_size
                                            .to_string()
                                            .parse()
                                            .expect("Invalid i64"),
                                        quote_lot_size: app
                                            .market_quote_lot_size
                                            .to_string()
                                            .parse()
                                            .expect("Invalid i64"),
                                        maker_fee: app
                                            .market_maker_fee
                                            .to_string()
                                            .parse()
                                            .expect("Invalid i64"),
                                        taker_fee: app
                                            .market_taker_fee
                                            .to_string()
                                            .parse()
                                            .expect("Invalid i64"),
                                        oracle_a: app.market_oracle_a.to_string().parse().ok(),
                                        oracle_b: app.market_oracle_b.to_string().parse().ok(),
                                        open_orders_admin: app
                                            .market_open_orders_admin
                                            .to_string()
                                            .parse()
                                            .ok(),
                                        collect_fee_admin: app
                                            .market_collect_fee_admin
                                            .to_string()
                                            .parse()
                                            .expect("Invalid Pubkey"),
                                        consume_events_admin: app
                                            .market_consume_events_admin
                                            .to_string()
                                            .parse()
                                            .ok(),
                                        close_market_admin: app
                                            .market_close_market_admin
                                            .to_string()
                                            .parse()
                                            .ok(),
                                        time_expiry: app
                                            .market_time_expiry
                                            .to_string()
                                            .parse()
                                            .expect("Invalid i64"),
                                    };

                                    let (_confirmed, _sig, market_id) =
                                        ob_client.create_market(market_args).await?;

                                    app.transaction_status = format!(
                                        "Transaction successful, got market id: {:?}",
                                        market_id
                                    )
                                    .to_string();
                                }
                            },
                            SelectedTab::Tab3 => {}
                            SelectedTab::Tab4 => match app.ob_client.clone().unwrap() {
                                SdkClient::OBClientV1(ob_client) => {
                                    let side = match app.side_input.value() {
                                        "bid" => OBV1Side::Bid,
                                        "ask" => OBV1Side::Ask,
                                        _ => OBV1Side::Bid,
                                    };
                                    let price =
                                        app.target_price_input.value().parse::<f64>().unwrap();

                                    let result = ob_client
                                        .place_limit_order(5.0, side, 5.0, true, price)
                                        .await?;
                                    match result {
                                        Some(OrderReturnType::Signature(signature)) => {
                                            app.transaction_status = format!(
                                                "Transaction successful, signature: {:?}",
                                                signature
                                            )
                                            .to_string();
                                        }
                                        Some(OrderReturnType::Instructions(_)) => {}
                                        _ => {}
                                    }
                                }
                                SdkClient::OBClientV2(_ob_client) => {}
                            },
                            SelectedTab::Tab5 => {}
                        }
                    }
                    KeyCode::Tab => {
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::RpcUrl => {
                                    app.current_input = Some(CurrentInput::KeyPath);
                                }
                                CurrentInput::KeyPath => match app.selected_tab {
                                    SelectedTab::Tab1 => {
                                        app.current_input = Some(CurrentInput::MarketID);
                                    }
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::MarketName);
                                    }
                                    SelectedTab::Tab3 => {
                                        app.current_input = Some(CurrentInput::MarketID);
                                    }
                                    SelectedTab::Tab4 => {
                                        app.current_input = Some(CurrentInput::MarketID);
                                    }
                                    SelectedTab::Tab5 => {
                                        app.current_input = Some(CurrentInput::MarketID);
                                    }
                                },
                                CurrentInput::MarketID => match app.selected_tab {
                                    SelectedTab::Tab1 => {
                                        app.current_input = Some(CurrentInput::RpcUrl);
                                    }
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::MarketName);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {
                                        app.current_input = Some(CurrentInput::OBV1Side);
                                    }
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::OBV1Side => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {}
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {
                                        app.current_input = Some(CurrentInput::TargetPrice);
                                    }
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::TargetPrice => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::BaseMint);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {
                                        app.current_input = Some(CurrentInput::RpcUrl);
                                    }
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::MarketName => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::BaseMint);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::BaseMint => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::QuoteMint);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::QuoteMint => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::BaseLotSize);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::BaseLotSize => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::QuoteLotSize);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::QuoteLotSize => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::MakerFee);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::MakerFee => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::TakerFee);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::TakerFee => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::OracleA);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::OracleA => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::OracleB);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::OracleB => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::OpenOrdersAdmin);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::OpenOrdersAdmin => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::CollectFeeAdmin);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::CollectFeeAdmin => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::ConsumeEventsAdmin);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::ConsumeEventsAdmin => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::CloseMarketAdmin);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::CloseMarketAdmin => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::TimeExpiry);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                                CurrentInput::TimeExpiry => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::RpcUrl);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                    SelectedTab::Tab5 => {}
                                },
                            }
                        }
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    _ => {
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::RpcUrl => {
                                    app.rpc_url_input.handle_event(&Event::Key(key));
                                }
                                CurrentInput::KeyPath => {
                                    app.key_path_input.handle_event(&Event::Key(key));
                                }
                                CurrentInput::MarketID => {
                                    app.market_id_input.handle_event(&Event::Key(key));
                                }
                                CurrentInput::OBV1Side => {
                                    app.side_input.handle_event(&Event::Key(key));
                                }
                                CurrentInput::TargetPrice => {
                                    app.target_price_input.handle_event(&Event::Key(key));
                                }
                                CurrentInput::MarketName => {
                                    app.market_name.handle_event(&Event::Key(key));
                                }
                                CurrentInput::BaseMint => {
                                    app.market_base_mint.handle_event(&Event::Key(key));
                                }
                                CurrentInput::QuoteMint => {
                                    app.market_quote_mint.handle_event(&Event::Key(key));
                                }
                                CurrentInput::BaseLotSize => {
                                    app.market_base_lot_size.handle_event(&Event::Key(key));
                                }
                                CurrentInput::QuoteLotSize => {
                                    app.market_quote_lot_size.handle_event(&Event::Key(key));
                                }
                                CurrentInput::MakerFee => {
                                    app.market_maker_fee.handle_event(&Event::Key(key));
                                }
                                CurrentInput::TakerFee => {
                                    app.market_taker_fee.handle_event(&Event::Key(key));
                                }
                                CurrentInput::OracleA => {
                                    app.market_oracle_a.handle_event(&Event::Key(key));
                                }
                                CurrentInput::OracleB => {
                                    app.market_oracle_b.handle_event(&Event::Key(key));
                                }
                                CurrentInput::OpenOrdersAdmin => {
                                    app.market_open_orders_admin.handle_event(&Event::Key(key));
                                }
                                CurrentInput::CollectFeeAdmin => {
                                    app.market_collect_fee_admin.handle_event(&Event::Key(key));
                                }
                                CurrentInput::ConsumeEventsAdmin => {
                                    app.market_consume_events_admin
                                        .handle_event(&Event::Key(key));
                                }
                                CurrentInput::CloseMarketAdmin => {
                                    app.market_close_market_admin.handle_event(&Event::Key(key));
                                }
                                CurrentInput::TimeExpiry => {
                                    app.market_time_expiry.handle_event(&Event::Key(key));
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

fn ui(frame: &mut Frame, app: &mut App, version: SdkVersion) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Max(20), Max(3), Max(3), Max(100)].as_ref())
        .split(frame.size());

    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to execute function"),
            ],
            Style::default(),
        ),
    };
    let text = Text::from(Line::from(msg).centered());
    let _ = text.clone().patch_style(style);
    let help_message = Paragraph::new(text);

    let vertical = Layout::new(
        Direction::Vertical,
        [Max(1), Max(1), Min(0), Length(1), Max(1)],
    )
    .split(chunks[0]);

    let titles = SelectedTab::iter().map(SelectedTab::title);
    let highlight_style = (Color::default(), app.selected_tab.palette().c700);
    let selected_tab_index = app.selected_tab as usize;
    let all_tabs = Tabs::new(titles)
        .highlight_style(highlight_style)
        .select(selected_tab_index)
        .padding("", "")
        .divider(" ");

    frame.render_widget(all_tabs.clone(), vertical[3]);

    match version {
        SdkVersion::V1 => {
            let ob_version = Line::raw("📖 OpenBook 1️⃣")
                .fg(Color::LightGreen)
                .bg(tailwind::SLATE.c700)
                .bold()
                .centered();
            frame.render_widget(ob_version.clone(), vertical[0]);
        }
        SdkVersion::V2 => {
            let ob_version = Line::raw("📖 OpenBook 2️⃣")
                .fg(Color::LightGreen)
                .bg(tailwind::SLATE.c700)
                .bold()
                .centered();
            frame.render_widget(ob_version.clone(), vertical[0]);
        }
    }

    // frame.render_widget(help_message, chunks[0]);

    let width = chunks[0].width.max(3);

    let scroll = app.rpc_url_input.visual_scroll(width as usize);
    let rpc_url_input = Paragraph::new(app.rpc_url_input.value())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => {
                let mut style = Style::default().fg(Color::Green);
                if let Some(current_input) = &app.current_input {
                    match current_input {
                        CurrentInput::RpcUrl => {
                            style = Style::default().white().on_black();
                        }
                        _ => {}
                    }
                }
                style
            }
        })
        .scroll((0, scroll as u16))
        .block(Block::default().borders(Borders::ALL).title("🔗 RPC Url"));

    // frame.render_widget(rpc_url_input, chunks[1]);

    let scroll = app.key_path_input.visual_scroll(width as usize);
    let key_path_input = Paragraph::new(app.key_path_input.value())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => {
                let mut style = Style::default().fg(Color::Green);
                if let Some(current_input) = &app.current_input {
                    match current_input {
                        CurrentInput::KeyPath => {
                            style = Style::default().white().on_black();
                        }
                        _ => {}
                    }
                }
                style
            }
        })
        .scroll((0, scroll as u16))
        .block(Block::default().borders(Borders::ALL).title("🗝  Key Path"));
    // frame.render_widget(key_path_input, chunks[2]);

    let first_row_layout =
        Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)]).split(chunks[1]);
    frame.render_widget(rpc_url_input, first_row_layout[0]);
    frame.render_widget(key_path_input, first_row_layout[1]);

    let scroll = app.side_input.visual_scroll(width as usize);
    let side_input = Paragraph::new(app.side_input.value())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => {
                let mut style = Style::default().fg(Color::Green);
                if let Some(current_input) = &app.current_input {
                    match current_input {
                        CurrentInput::OBV1Side => {
                            style = Style::default().white().on_black();
                        }
                        _ => {}
                    }
                }
                style
            }
        })
        .scroll((0, scroll as u16))
        .block(Block::default().borders(Borders::ALL).title("🛍️ Bid or Ask"));

    let scroll = app.target_price_input.visual_scroll(width as usize);
    let target_price_input = Paragraph::new(app.target_price_input.value())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => {
                let mut style = Style::default().fg(Color::Green);
                if let Some(current_input) = &app.current_input {
                    match current_input {
                        CurrentInput::TargetPrice => {
                            style = Style::default().white().on_black();
                        }
                        _ => {}
                    }
                }
                style
            }
        })
        .scroll((0, scroll as u16))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🎯 Target Price"),
        );

    let third_row_layout =
        Layout::new(Direction::Vertical, [Percentage(85), Max(6)]).split(chunks[3]);
    let market_rows = Layout::new(
        Direction::Vertical,
        [
            Max(3),
            Max(3),
            Max(3),
            Max(3),
            Max(3),
            Max(3),
            Max(3),
            Max(3),
        ],
    )
    .split(third_row_layout[0]);
    let order_input_row_layout =
        Layout::new(Direction::Vertical, [Max(3), Max(10)]).split(chunks[3]);

    let order_row_layout = Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)])
        .split(order_input_row_layout[0]);

    let first_row_market =
        Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)]).split(market_rows[0]);

    let second_row_market =
        Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)]).split(market_rows[1]);

    let third_row_market =
        Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)]).split(market_rows[2]);

    let fourth_row_market =
        Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)]).split(market_rows[3]);

    let fifth_row_market =
        Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)]).split(market_rows[4]);

    let sixth_row_market =
        Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)]).split(market_rows[5]);

    let seventh_row_market =
        Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)]).split(market_rows[6]);

    match app.input_mode {
        InputMode::Normal => {}
        InputMode::Editing => {
            if let Some(current_input) = &app.current_input {
                match current_input {
                    CurrentInput::RpcUrl => frame.set_cursor(
                        first_row_layout[0].x
                            + ((app.rpc_url_input.visual_cursor()).max(scroll) - scroll) as u16
                            + 1,
                        first_row_layout[0].y + 1,
                    ),
                    CurrentInput::KeyPath => frame.set_cursor(
                        first_row_layout[1].x
                            + ((app.key_path_input.visual_cursor()).max(scroll) - scroll) as u16
                            + 1,
                        first_row_layout[1].y + 1,
                    ),
                    CurrentInput::MarketID => frame.set_cursor(
                        chunks[2].x
                            + ((app.market_id_input.visual_cursor()).max(scroll) - scroll) as u16
                            + 1,
                        chunks[2].y + 1,
                    ),
                    CurrentInput::OBV1Side => frame.set_cursor(
                        order_row_layout[0].x
                            + ((app.side_input.visual_cursor()).max(scroll) - scroll) as u16
                            + 1,
                        order_row_layout[0].y + 1,
                    ),
                    CurrentInput::TargetPrice => frame.set_cursor(
                        order_row_layout[1].x
                            + ((app.target_price_input.visual_cursor()).max(scroll) - scroll)
                                as u16
                            + 1,
                        order_row_layout[1].y + 1,
                    ),
                    CurrentInput::MarketName => {
                        frame.set_cursor(
                            first_row_market[0].x
                                + ((app.market_name.visual_cursor()).max(scroll) - scroll) as u16
                                + 1,
                            first_row_market[0].y + 1,
                        );
                    }
                    CurrentInput::BaseMint => {
                        frame.set_cursor(
                            first_row_market[1].x
                                + ((app.market_base_mint.visual_cursor()).max(scroll) - scroll)
                                    as u16
                                + 1,
                            first_row_market[1].y + 1,
                        );
                    }
                    CurrentInput::QuoteMint => {
                        frame.set_cursor(
                            second_row_market[0].x
                                + ((app.market_quote_mint.visual_cursor()).max(scroll) - scroll)
                                    as u16
                                + 1,
                            second_row_market[0].y + 1,
                        );
                    }
                    CurrentInput::BaseLotSize => {
                        frame.set_cursor(
                            second_row_market[1].x
                                + ((app.market_base_lot_size.visual_cursor()).max(scroll) - scroll)
                                    as u16
                                + 1,
                            second_row_market[1].y + 1,
                        );
                    }
                    CurrentInput::QuoteLotSize => {
                        frame.set_cursor(
                            third_row_market[0].x
                                + ((app.market_quote_lot_size.visual_cursor()).max(scroll) - scroll)
                                    as u16
                                + 1,
                            third_row_market[0].y + 1,
                        );
                    }
                    CurrentInput::MakerFee => {
                        frame.set_cursor(
                            third_row_market[1].x
                                + ((app.market_maker_fee.visual_cursor()).max(scroll) - scroll)
                                    as u16
                                + 1,
                            third_row_market[1].y + 1,
                        );
                    }
                    CurrentInput::TakerFee => {
                        frame.set_cursor(
                            fourth_row_market[0].x
                                + ((app.market_taker_fee.visual_cursor()).max(scroll) - scroll)
                                    as u16
                                + 1,
                            fourth_row_market[0].y + 1,
                        );
                    }
                    CurrentInput::OracleA => {
                        frame.set_cursor(
                            fourth_row_market[1].x
                                + ((app.market_oracle_a.visual_cursor()).max(scroll) - scroll)
                                    as u16
                                + 1,
                            fourth_row_market[1].y + 1,
                        );
                    }
                    CurrentInput::OracleB => {
                        frame.set_cursor(
                            fifth_row_market[0].x
                                + ((app.market_oracle_b.visual_cursor()).max(scroll) - scroll)
                                    as u16
                                + 1,
                            fifth_row_market[0].y + 1,
                        );
                    }
                    CurrentInput::OpenOrdersAdmin => {
                        frame.set_cursor(
                            fifth_row_market[1].x
                                + ((app.market_open_orders_admin.visual_cursor()).max(scroll)
                                    - scroll) as u16
                                + 1,
                            fifth_row_market[1].y + 1,
                        );
                    }
                    CurrentInput::CollectFeeAdmin => {
                        frame.set_cursor(
                            sixth_row_market[0].x
                                + ((app.market_collect_fee_admin.visual_cursor()).max(scroll)
                                    - scroll) as u16
                                + 1,
                            sixth_row_market[0].y + 1,
                        );
                    }
                    CurrentInput::ConsumeEventsAdmin => {
                        frame.set_cursor(
                            sixth_row_market[1].x
                                + ((app.market_consume_events_admin.visual_cursor()).max(scroll)
                                    - scroll) as u16
                                + 1,
                            sixth_row_market[1].y + 1,
                        );
                    }
                    CurrentInput::CloseMarketAdmin => {
                        frame.set_cursor(
                            seventh_row_market[0].x
                                + ((app.market_close_market_admin.visual_cursor()).max(scroll)
                                    - scroll) as u16
                                + 1,
                            seventh_row_market[0].y + 1,
                        );
                    }
                    CurrentInput::TimeExpiry => {
                        frame.set_cursor(
                            seventh_row_market[1].x
                                + ((app.market_time_expiry.visual_cursor()).max(scroll) - scroll)
                                    as u16
                                + 1,
                            seventh_row_market[1].y + 1,
                        );
                    }
                }
            }
        }
    }

    let market_info: Vec<ListItem> = app
        .market_info
        .iter()
        .map(|(key, val)| {
            let content = vec![Line::from(Span::raw(format!("{}: {}", key, val)))];
            ListItem::new(content)
        })
        .collect();

    let wallet_info: Vec<ListItem> = app
        .wallet_info
        .iter()
        .map(|(key, val)| {
            let content = vec![Line::from(Span::raw(format!("{}: {}", key, val)))];
            ListItem::new(content)
        })
        .collect();
    let market_info = List::new(market_info).block(
        Block::default()
            .borders(Borders::ALL)
            .title("💧 Market Info"),
    );

    let wallet_info = List::new(wallet_info).block(
        Block::default()
            .borders(Borders::ALL)
            .title("💰 Wallet Info"),
    );

    let transaction_status = Paragraph::new(app.transaction_status.clone())
        .block(Block::default().borders(Borders::ALL).title("Response"));

    match app.selected_tab {
        SelectedTab::Tab1 => {
            let third_row_info_layout =
                Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)])
                    .split(third_row_layout[0]);
            frame.render_widget(market_info, third_row_info_layout[0]);
            frame.render_widget(wallet_info, third_row_info_layout[1]);

            let scroll = app.market_id_input.visual_scroll(width as usize);
            let market_id_input = Paragraph::new(app.market_id_input.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::MarketID => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("#️ Market ID"));

            frame.render_widget(market_id_input, chunks[2]);
        }
        SelectedTab::Tab2 => {
            let width = chunks[0].width.max(3);

            let scroll = app.rpc_url_input.visual_scroll(width as usize);
            let market_name_input = Paragraph::new(app.market_name.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::MarketName => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Market Name"));

            frame.render_widget(market_name_input, first_row_market[0]);

            let market_base_input = Paragraph::new(app.market_base_mint.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::BaseMint => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Base Mint"));

            frame.render_widget(market_base_input, first_row_market[1]);

            let market_quote_mint = Paragraph::new(app.market_quote_mint.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::QuoteMint => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Quote Mint"));

            frame.render_widget(market_quote_mint, second_row_market[0]);

            let market_base_lot_size = Paragraph::new(app.market_base_lot_size.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::BaseLotSize => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Base Lot Size"),
                );

            frame.render_widget(market_base_lot_size, second_row_market[1]);

            let market_quote_lot_size = Paragraph::new(app.market_quote_lot_size.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::QuoteLotSize => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Quote Lot Size"),
                );

            frame.render_widget(market_quote_lot_size, third_row_market[0]);

            let market_maker_fee = Paragraph::new(app.market_maker_fee.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::MakerFee => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Maker Fee"));

            frame.render_widget(market_maker_fee.clone(), third_row_market[1]);

            let market_taker_fee = Paragraph::new(app.market_taker_fee.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::TakerFee => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Taker Fee"));

            frame.render_widget(market_taker_fee, fourth_row_market[0]);

            let market_oracle_a = Paragraph::new(app.market_oracle_a.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::OracleA => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Oracle A"));

            frame.render_widget(market_oracle_a, fourth_row_market[1]);

            let market_oracle_b = Paragraph::new(app.market_oracle_b.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::OracleB => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Oracle B"));

            frame.render_widget(market_oracle_b, fifth_row_market[0]);

            let market_open_orders_admin = Paragraph::new(app.market_open_orders_admin.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::OpenOrdersAdmin => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Open Orders Admin"),
                );

            frame.render_widget(market_open_orders_admin, fifth_row_market[1]);

            let market_collect_fee_admin = Paragraph::new(app.market_collect_fee_admin.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::CollectFeeAdmin => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Collect Fee Admin"),
                );

            frame.render_widget(market_collect_fee_admin, sixth_row_market[0]);

            let market_consume_events_admin =
                Paragraph::new(app.market_consume_events_admin.value())
                    .style(match app.input_mode {
                        InputMode::Normal => Style::default(),
                        InputMode::Editing => {
                            let mut style = Style::default().fg(Color::Green);
                            if let Some(current_input) = &app.current_input {
                                match current_input {
                                    CurrentInput::ConsumeEventsAdmin => {
                                        style = Style::default().white().on_black();
                                    }
                                    _ => {}
                                }
                            }
                            style
                        }
                    })
                    .scroll((0, scroll as u16))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Consume Events Admin"),
                    );

            frame.render_widget(market_consume_events_admin, sixth_row_market[1]);

            let market_close_market_admin = Paragraph::new(app.market_close_market_admin.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::CollectFeeAdmin => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Close Market Admin"),
                );

            frame.render_widget(market_close_market_admin, seventh_row_market[0]);

            let market_time_expiry = Paragraph::new(app.market_time_expiry.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::ConsumeEventsAdmin => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Time Expiry"));

            frame.render_widget(market_time_expiry, seventh_row_market[1]);
            frame.render_widget(transaction_status, market_rows[7]);
        }
        SelectedTab::Tab3 => {
            let scroll = app.market_id_input.visual_scroll(width as usize);
            let market_id_input = Paragraph::new(app.market_id_input.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::MarketID => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("#️ Market ID"));

            frame.render_widget(market_id_input, chunks[2]);
            let inner = Paragraph::new("Coming Soon!")
                .block(Block::default().borders(Borders::ALL).title("Soon"));
            frame.render_widget(inner, third_row_layout[0]);
        }
        SelectedTab::Tab4 => {
            let scroll = app.market_id_input.visual_scroll(width as usize);
            let market_id_input = Paragraph::new(app.market_id_input.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::MarketID => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("#️ Market ID"));

            frame.render_widget(market_id_input, chunks[2]);
            frame.render_widget(side_input, order_row_layout[0]);
            frame.render_widget(target_price_input, order_row_layout[1]);
            frame.render_widget(transaction_status, order_input_row_layout[1]);
        }
        SelectedTab::Tab5 => {
            let scroll = app.market_id_input.visual_scroll(width as usize);
            let market_id_input = Paragraph::new(app.market_id_input.value())
                .style(match app.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => {
                        let mut style = Style::default().fg(Color::Green);
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::MarketID => {
                                    style = Style::default().white().on_black();
                                }
                                _ => {}
                            }
                        }
                        style
                    }
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("#️ Market ID"));

            frame.render_widget(market_id_input, chunks[2]);
            let inner = Paragraph::new("Coming Soon!")
                .block(Block::default().borders(Borders::ALL).title("Soon"));
            frame.render_widget(inner, third_row_layout[0]);
        }
    }

    let footer_layout =
        Layout::new(Direction::Vertical, [Max(1), Max(1), Max(1)]).split(third_row_layout[1]);

    let top_footer = Line::raw("◄ ► or a/d to change tab").centered();

    let bottom_footer = Line::raw("© GigaDAO Foundation")
        .fg(Color::LightGreen)
        .bg(tailwind::SLATE.c700)
        .bold()
        .centered();

    frame.render_widget(top_footer, footer_layout[0]);
    frame.render_widget(help_message, footer_layout[1]);
    frame.render_widget(bottom_footer, footer_layout[2]);
}
