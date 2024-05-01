use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::style::Stylize;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::collections::HashMap;
use std::{error::Error, io};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::commitment_config::CommitmentConfig;
use crate::market::Market;
use crate::rpc_client::RpcClient;
use crate::utils::read_keypair;

enum InputMode {
    Normal,
    Editing,
}

enum CurrentInput {
    RpcUrl,
    KeyPath,
    BaseMint,
    QuoteMint,
}

struct App {
    rpc_url_input: Input,
    key_path_input: Input,
    base_mint_input: Input,
    quote_mint_input: Input,
    input_mode: InputMode,
    current_input: Option<CurrentInput>,
    messages: HashMap<String, String>,
    market: Option<Market>,
}

impl Default for App {
    fn default() -> App {
        App {
            rpc_url_input: Input::default(),
            key_path_input: Input::default(),
            base_mint_input: Input::default(),
            quote_mint_input: Input::default(),
            input_mode: InputMode::Normal,
            current_input: Some(CurrentInput::RpcUrl),
            messages: HashMap::new(),
            market: None,
        }
    }
}

pub async fn run_tui() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
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

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('e') => {
                        app.input_mode = InputMode::Editing;
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

                        let base_mint =
                            Box::leak(app.base_mint_input.value().to_string().into_boxed_str());
                        let quote_mint =
                            Box::leak(app.quote_mint_input.value().to_string().into_boxed_str());

                        if app.market.is_none() {
                            app.market = Some(
                                Market::new(rpc_client, 3, base_mint, quote_mint, owner, true)
                                    .await,
                            );
                        }

                        app.messages.insert(
                            "Market Address".to_string(),
                            app.market.as_ref().unwrap().market_address.to_string(),
                        );
                        app.messages.insert(
                            "Base ATA".to_string(),
                            app.market.as_ref().unwrap().base_ata.to_string(),
                        );
                        app.messages.insert(
                            "Quote ATA".to_string(),
                            app.market.as_ref().unwrap().quote_ata.to_string(),
                        );
                        app.messages.insert(
                            "Mint ATA".to_string(),
                            app.market.as_ref().unwrap().ata_address.to_string(),
                        );
                        app.messages.insert(
                            "Coin Vault".to_string(),
                            app.market.as_ref().unwrap().coin_vault.to_string(),
                        );
                        app.messages.insert(
                            "PC Vault".to_string(),
                            app.market.as_ref().unwrap().pc_vault.to_string(),
                        );
                        app.messages.insert(
                            "Vault Signer Key".to_string(),
                            app.market.as_ref().unwrap().vault_signer_key.to_string(),
                        );
                        app.messages.insert(
                            "Event Queue".to_string(),
                            app.market.as_ref().unwrap().event_queue.to_string(),
                        );
                        app.messages.insert(
                            "Request Queue".to_string(),
                            app.market.as_ref().unwrap().request_queue.to_string(),
                        );
                        app.messages.insert(
                            "Asks Address".to_string(),
                            app.market
                                .as_ref()
                                .unwrap()
                                .market_info
                                .asks_address
                                .to_string(),
                        );
                        app.messages.insert(
                            "Bids Address".to_string(),
                            app.market
                                .as_ref()
                                .unwrap()
                                .market_info
                                .bids_address
                                .to_string(),
                        );
                        app.messages.insert(
                            "Coin Decimals".to_string(),
                            app.market.as_ref().unwrap().coin_decimals.to_string(),
                        );
                        app.messages.insert(
                            "PC Decimals".to_string(),
                            app.market.as_ref().unwrap().pc_decimals.to_string(),
                        );
                        app.messages.insert(
                            "Coin Lot Size".to_string(),
                            app.market.as_ref().unwrap().coin_lot_size.to_string(),
                        );
                        app.messages.insert(
                            "PC Lot Size".to_string(),
                            app.market.as_ref().unwrap().pc_lot_size.to_string(),
                        );
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
                                    app.current_input = Some(CurrentInput::BaseMint);
                                }
                                CurrentInput::BaseMint => {
                                    app.current_input = Some(CurrentInput::QuoteMint);
                                }
                                CurrentInput::QuoteMint => {
                                    app.current_input = Some(CurrentInput::RpcUrl);
                                }
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
                                CurrentInput::BaseMint => {
                                    app.base_mint_input.handle_event(&Event::Key(key));
                                }
                                CurrentInput::QuoteMint => {
                                    app.quote_mint_input.handle_event(&Event::Key(key));
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
        .constraints(
            [
                Constraint::Max(2),
                Constraint::Max(3),
                Constraint::Max(3),
                Constraint::Max(100),
            ]
            .as_ref(),
        )
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
                Span::raw(" to fetch market info"),
            ],
            Style::default(),
        ),
    };
    let text = Text::from(Line::from(msg));
    let _ = text.clone().patch_style(style);
    let help_message = Paragraph::new(text);

    frame.render_widget(help_message, chunks[0]);

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
        .block(Block::default().borders(Borders::ALL).title("RPC_URL"));

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
        .block(Block::default().borders(Borders::ALL).title("KEY_PATH"));
    // frame.render_widget(key_path_input, chunks[2]);

    let first_row_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(chunks[1]);
    frame.render_widget(rpc_url_input, first_row_layout[0]);
    frame.render_widget(key_path_input, first_row_layout[1]);

    let scroll = app.base_mint_input.visual_scroll(width as usize);
    let base_mint_input = Paragraph::new(app.base_mint_input.value())
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

    let scroll = app.quote_mint_input.visual_scroll(width as usize);
    let quote_mint_input = Paragraph::new(app.quote_mint_input.value())
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

    let second_row_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(chunks[2]);
    frame.render_widget(base_mint_input, second_row_layout[0]);
    frame.render_widget(quote_mint_input, second_row_layout[1]);

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
                    CurrentInput::BaseMint => frame.set_cursor(
                        second_row_layout[0].x
                            + ((app.base_mint_input.visual_cursor()).max(scroll) - scroll) as u16
                            + 1,
                        second_row_layout[0].y + 1,
                    ),
                    CurrentInput::QuoteMint => frame.set_cursor(
                        second_row_layout[1].x
                            + ((app.quote_mint_input.visual_cursor()).max(scroll) - scroll) as u16
                            + 1,
                        second_row_layout[1].y + 1,
                    ),
                }
            }
        }
    }

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .map(|(key, val)| {
            let content = vec![Line::from(Span::raw(format!("{}: {}", key, val)))];
            ListItem::new(content)
        })
        .collect();
    let messages =
        List::new(messages).block(Block::default().borders(Borders::ALL).title("Market Info"));
    frame.render_widget(messages, chunks[3]);
}
