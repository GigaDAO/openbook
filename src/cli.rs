//! This module contains the cli functionalities related to the openbook market.

#[cfg(feature = "cli")]
use clap::builder::styling::{AnsiColor, Effects, Styles};
#[cfg(feature = "cli")]
use clap::{Args, Parser, Subcommand};

#[cfg(feature = "cli")]
fn styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Red.on_default() | Effects::BOLD)
        .usage(AnsiColor::Red.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default() | Effects::BOLD)
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Green.on_default())
}

#[cfg(feature = "cli")]
#[derive(Parser, Debug, Clone)]
#[command(
    author = "Mahmoud Harmouch",
    version,
    name = "openbook",
    propagate_version = true,
    styles = styles(),
    help_template = r#"{before-help}{name} {version}
{about-with-newline}

{usage-heading} {usage}

{all-args}{after-help}

AUTHORS:
    {author}
"#,
    about=r#"
      ████████      
   ██████████       
  ████  ████        
 ████  ████         
████   ███████████ 
████    ███████████
       ███     ████
       ██     ████ 
      █     █████  
        ████████   
        █████      

📖1️⃣2️⃣ A CLI, TUI and SDK to interact with OpenBook V1 and V2 markets on the Solana blockchain.

FUNCTIONALITIES:
  - Place Limit Bid: Place a limit bid in the OpenBook market.
  - Cancel Order: Cancel an existing order in the OpenBook market.
  - Settle Balance: Settle balances in the OpenBook market.
  - Match Orders Transaction: Match orders transactions in the OpenBook market.
  - Consume Events Instruction: Consume events instructions in the OpenBook market.
  - Consume Events Permissioned Instruction: Consume events permissioned instructions in the OpenBook market.
  - Load Orders For Owner: Load orders for a specific owner in the OpenBook market.
  - Find Open Orders Accounts For Owner: Find open orders accounts for a specific owner in the OpenBook market.

USAGE:
  openbook [OPTIONS] <COMMAND>

EXAMPLES:
  Launch TUI:
    openbook

  Place a bid limit order on openbook v1 market:
    openbook v1 place -t 5.0 -s bid -b 5. -e -p 2.1

  Place a ask limit order on openbook v1 market:
    openbook v1 place -t 5.0 -s ask -b 5. -e -p 2.1

  Cancel all limit orders on openbook v1 market:
    openbook v1 cancel -e

  Settle balances on openbook v1 market:
    openbook v1 settle -e

For more information, visit: github.com/gigadao/openbook
"#
)]
pub struct Cli {
    /// Turn debugging information on.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Represents OpenBook-related subcommands.
#[cfg(feature = "cli")]
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum Commands {
    /// Select OpenBook v1.
    V1(V1),
    /// Select OpenBook v2.
    V2(V2),
}

#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct V1 {
    /// The market id to trade on.
    #[arg(short, long, default_value_t = String::from("8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6"))]
    pub market_id: String,
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Option<V1ActionsCommands>,
}

#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct V2 {
    /// The market id to trade on.
    #[arg(short, long, default_value_t = String::from("gQN1TNHiqj5x82ZQd7JZ8rm8WD4xwWtXxd4onReWZNK"))]
    pub market_id: String,
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Option<V2ActionsCommands>,
}

/// Represents OpenBook V1 market actions subcommands.
#[cfg(feature = "cli")]
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum V1ActionsCommands {
    /// Place a limit order.
    Place(Place),
    /// Cancel an order.
    Cancel(Cancel),
    /// Settle balance.
    Settle(Settle),
    /// Match orders transaction.
    Match(Match),
    /// Cancel, Settle, and Place orders transaction.
    CancelSettlePlace(CancelSettlePlace),
    /// Cancel, Settle, and Place Bid orders transaction.
    CancelSettlePlaceBid(CancelSettlePlaceBid),
    /// Cancel, Settle, and Place Ask orders transaction.
    CancelSettlePlaceAsk(CancelSettlePlaceAsk),
    /// Consume events instruction.
    Consume(Consume),
    /// Consume events permissioned instruction.
    ConsumePermissioned(ConsumePermissioned),
    /// Load orders for owner.
    Load(Load),
    /// Find open orders accounts for owner.
    Find(Find),
    /// Fetch Market Info.
    Info(Info),
}

/// Represents OpenBook V2 market actions subcommands.
#[cfg(feature = "cli")]
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum V2ActionsCommands {
    /// Fetch Market Info.
    Info(Info),
}

/// Represents options for placing a limit order in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct Place {
    /// Target amount in quote currency.
    #[arg(short, long)]
    pub target_amount_quote: f64,

    /// Side of the order (Bid or Ask).
    #[arg(short, long)]
    pub side: String,

    /// Best offset in USDC.
    #[arg(short, long)]
    pub best_offset_usdc: f64,

    /// Flag indicating whether to execute the order immediately.
    #[arg(short, long)]
    pub execute: bool,

    /// Target price for the order.
    #[arg(short, long)]
    pub price_target: f64,
}

/// Represents options for executing a combination of canceling all limit orders,
/// settling balance, and placing new bid and ask orders.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct CancelSettlePlace {
    /// The target size in USDC for the ask order.
    #[arg(short, long)]
    pub usdc_ask_target: f64,

    /// The target size in USDC for the bid order.
    #[arg(short, long)]
    pub target_usdc_bid: f64,

    /// The bid price in JLP/USDC.
    #[arg(short, long)]
    pub price_jlp_usdc_bid: f64,

    /// The ask price in JLP/USDC.
    #[arg(short, long)]
    pub ask_price_jlp_usdc: f64,
}

/// Represents options for executing a combination of canceling all limit orders,
/// settling balance, and placing a bid order.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct CancelSettlePlaceBid {
    /// The target size in USDC for the bid order.
    #[arg(short, long)]
    pub target_size_usdc_bid: f64,

    /// The bid price in JLP/USDC.
    #[arg(short, long)]
    pub bid_price_jlp_usdc: f64,
}

/// Represents options for executing a combination of canceling all limit orders,
/// settling balance, and placing an ask order.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct CancelSettlePlaceAsk {
    /// The target size in USDC for the ask order.
    #[arg(short, long)]
    pub target_size_usdc_ask: f64,

    /// The ask price in JLP/USDC.
    #[arg(short, long)]
    pub ask_price_jlp_usdc: f64,
}

/// Represents options for cancelling an order in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct Cancel {
    /// Flag indicating whether to execute the order immediately.
    #[arg(short, long)]
    pub execute: bool,
}

/// Represents options for settling balances in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct Settle {
    /// Flag indicating whether to execute the order immediately.
    #[arg(short, long)]
    pub execute: bool,
}

/// Represents options for match orders transactions in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct Match {
    /// The maximum number of orders to match.
    #[arg(short, long)]
    pub limit: u16,
}

/// Represents options for consume events instructions in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct Consume {
    /// Limit for consume events instruction.
    #[arg(short, long)]
    pub limit: u16,
}

/// Represents options for consume events permissioned instructions in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct ConsumePermissioned {
    /// Limit for consume events permissioned instruction.
    #[arg(short, long)]
    pub limit: u16,
}

/// Represents options for loading orders for the current owner in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct Load {}

/// Represents options for finding open orders accounts for a specific owner in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct Find {}

/// Represents options for fetching the OpenBook market info.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct Info {}
