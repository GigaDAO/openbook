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
use crate::v1::{ob_client::OBClient, orders::OrderReturnType};

use anyhow::{Error, Result};

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
}

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter)]
enum SelectedTab {
    #[default]
    #[strum(to_string = "Market Info")]
    Tab1,
    #[strum(to_string = "Actions")]
    Tab2,
    #[strum(to_string = "Swap")]
    Tab3,
    #[strum(to_string = "Chart")]
    Tab4,
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
            Self::Tab2 => tailwind::EMERALD,
            Self::Tab3 => tailwind::INDIGO,
            Self::Tab4 => tailwind::SLATE,
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
    ob_client: Option<OBClient>,
    selected_tab: SelectedTab,
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
        }
    }
}

pub async fn run_tui() -> Result<(), Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app).await;

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

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), Error> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

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

                        let market_id = app.market_id_input.value();

                        if app.ob_client.is_none() {
                            let mut ob_client = OBClient::new(
                                commitment_config,
                                market_id.parse().unwrap(),
                                true,
                                123456789,
                            )
                            .await?;
                            ob_client.rpc_client = Rpc::new(rpc_client);
                            ob_client.owner = owner.into();
                            app.ob_client = Some(ob_client);
                        }

                        match app.selected_tab {
                            SelectedTab::Tab1 => {
                                app.market_info.insert(
                                    "Market Address".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .market_address
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "Base Mint".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .base_mint
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "Quote Mint".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .quote_mint
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "Coin Vault".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .coin_vault
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "PC Vault".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .pc_vault
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "Event Queue".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .event_queue
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "Request Queue".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .request_queue
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "Asks Address".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .asks_address
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "Bids Address".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .bids_address
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "Coin Decimals".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .coin_decimals
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "PC Decimals".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .pc_decimals
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "Coin Lot Size".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .coin_lot_size
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "PC Lot Size".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .pc_lot_size
                                        .to_string(),
                                );
                                app.market_info.insert(
                                    "Vault Signer Key".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .market_info
                                        .vault_signer_key
                                        .to_string(),
                                );

                                app.wallet_info.insert(
                                    "Wallet Public Key".to_string(),
                                    app.ob_client.as_ref().unwrap().owner.pubkey().to_string(),
                                );
                                app.wallet_info.insert(
                                    "Base ATA".to_string(),
                                    app.ob_client.as_ref().unwrap().base_ata.to_string(),
                                );
                                app.wallet_info.insert(
                                    "Quote ATA".to_string(),
                                    app.ob_client.as_ref().unwrap().quote_ata.to_string(),
                                );
                                app.wallet_info.insert(
                                    "Open Order Account".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .open_orders
                                        .oo_key
                                        .to_string(),
                                );
                                app.wallet_info.insert(
                                    "Min Ask".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .open_orders
                                        .min_ask
                                        .to_string(),
                                );
                                app.wallet_info.insert(
                                    "Max Bid".to_string(),
                                    app.ob_client
                                        .as_ref()
                                        .unwrap()
                                        .open_orders
                                        .max_bid
                                        .to_string(),
                                );

                                let open_asks: String = app
                                    .ob_client
                                    .as_ref()
                                    .unwrap()
                                    .open_orders
                                    .open_asks
                                    .iter()
                                    .map(|&x| x.to_string())
                                    .collect::<Vec<String>>()
                                    .join(", ");
                                app.wallet_info.insert("Open Asks".to_string(), open_asks);

                                let open_bids: String = app
                                    .ob_client
                                    .as_ref()
                                    .unwrap()
                                    .open_orders
                                    .open_bids
                                    .iter()
                                    .map(|&x| x.to_string())
                                    .collect::<Vec<String>>()
                                    .join(", ");
                                app.wallet_info.insert("Open Bids".to_string(), open_bids);
                            }
                            SelectedTab::Tab2 => {
                                let side = match app.side_input.value() {
                                    "bid" => OBV1Side::Bid,
                                    "ask" => OBV1Side::Ask,
                                    _ => OBV1Side::Bid,
                                };
                                let price = app.target_price_input.value().parse::<f64>().unwrap();

                                let result = app
                                    .ob_client
                                    .as_ref()
                                    .unwrap()
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
                            SelectedTab::Tab3 => {}
                            SelectedTab::Tab4 => {}
                        }
                        // TODO: add all fields
                        // if let Some(current_input) = &app.current_input {
                        //     match current_input {
                        //         CurrentInput::RpcUrl => {
                        //             app.rpc_url_input.reset();
                        //         }
                        //         CurrentInput::KeyPath => {
                        //             app.key_path_input.reset();
                        //         }
                        //         CurrentInput::BaseMint => {
                        //             app.base_mint_input.reset();
                        //         }
                        //         CurrentInput::QuoteMint => {
                        //             app.quote_mint_input.reset();
                        //         }
                        //     }
                        // }
                    }
                    KeyCode::Tab => {
                        if let Some(current_input) = &app.current_input {
                            match current_input {
                                CurrentInput::RpcUrl => {
                                    app.current_input = Some(CurrentInput::KeyPath);
                                }
                                CurrentInput::KeyPath => {
                                    app.current_input = Some(CurrentInput::MarketID);
                                }
                                CurrentInput::MarketID => match app.selected_tab {
                                    SelectedTab::Tab1 => {
                                        app.current_input = Some(CurrentInput::RpcUrl);
                                    }
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::OBV1Side);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                },
                                CurrentInput::OBV1Side => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::TargetPrice);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
                                },
                                CurrentInput::TargetPrice => match app.selected_tab {
                                    SelectedTab::Tab1 => {}
                                    SelectedTab::Tab2 => {
                                        app.current_input = Some(CurrentInput::RpcUrl);
                                    }
                                    SelectedTab::Tab3 => {}
                                    SelectedTab::Tab4 => {}
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
                            }
                        }
                    }
                },
            }
        }
    }
}

fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Max(2), Max(3), Max(3), Max(100)].as_ref())
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

    let vertical = Layout::new(Direction::Vertical, [Max(1), Min(0), Length(1)]).split(chunks[0]);

    let titles = SelectedTab::iter().map(SelectedTab::title);
    let highlight_style = (Color::default(), app.selected_tab.palette().c700);
    let selected_tab_index = app.selected_tab as usize;
    let all_tabs = Tabs::new(titles)
        .highlight_style(highlight_style)
        .select(selected_tab_index)
        .padding("", "")
        .divider(" ");

    frame.render_widget(all_tabs.clone(), vertical[0]);

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
        .block(Block::default().borders(Borders::ALL).title("Rpc Url"));

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
        .block(Block::default().borders(Borders::ALL).title("Key Path"));
    // frame.render_widget(key_path_input, chunks[2]);

    let first_row_layout =
        Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)]).split(chunks[1]);
    frame.render_widget(rpc_url_input, first_row_layout[0]);
    frame.render_widget(key_path_input, first_row_layout[1]);

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
        .block(Block::default().borders(Borders::ALL).title("Quote Mint"));

    frame.render_widget(market_id_input, chunks[2]);

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
        .block(Block::default().borders(Borders::ALL).title("Bid or Ask"));

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
        .block(Block::default().borders(Borders::ALL).title("Target Price"));
    let order_input_row_layout =
        Layout::new(Direction::Vertical, [Max(3), Max(10)]).split(chunks[3]);

    let order_row_layout = Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)])
        .split(order_input_row_layout[0]);

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
    let market_info =
        List::new(market_info).block(Block::default().borders(Borders::ALL).title("Market Info"));

    let wallet_info =
        List::new(wallet_info).block(Block::default().borders(Borders::ALL).title("Wallet Info"));

    let transaction_status = Paragraph::new(app.transaction_status.clone())
        .block(Block::default().borders(Borders::ALL).title("Response"));

    let third_row_layout =
        Layout::new(Direction::Vertical, [Percentage(85), Max(6)]).split(chunks[3]);

    match app.selected_tab {
        SelectedTab::Tab1 => {
            let third_row_info_layout =
                Layout::new(Direction::Horizontal, [Percentage(50), Percentage(50)])
                    .split(third_row_layout[0]);
            frame.render_widget(market_info, third_row_info_layout[0]);
            frame.render_widget(wallet_info, third_row_info_layout[1]);
        }
        SelectedTab::Tab2 => {
            frame.render_widget(side_input, order_row_layout[0]);
            frame.render_widget(target_price_input, order_row_layout[1]);
            frame.render_widget(transaction_status, order_input_row_layout[1]);
        }
        SelectedTab::Tab3 => {
            let inner = Paragraph::new("Coming Soon!")
                .block(Block::default().borders(Borders::ALL).title("Soon"));
            frame.render_widget(inner, third_row_layout[0]);
        }
        SelectedTab::Tab4 => {
            let inner = Paragraph::new("Coming Soon!")
                .block(Block::default().borders(Borders::ALL).title("Soon"));
            frame.render_widget(inner, third_row_layout[0]);
        }
    }

    let footer_layout =
        Layout::new(Direction::Vertical, [Max(1), Max(1), Max(1)]).split(third_row_layout[1]);

    let top_footer = Line::raw("◄ ► or a/d to change tab | Press q to quit").centered();
    let bottom_footer = Line::raw("©️ GigaDAO Foundation")
        .bold()
        .centered()
        .fg(Color::LightGreen);
    frame.render_widget(top_footer, footer_layout[0]);
    frame.render_widget(help_message, footer_layout[1]);
    frame.render_widget(bottom_footer, footer_layout[2]);
}
