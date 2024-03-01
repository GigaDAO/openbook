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
📖 OPENBOOK CLI
==============

A command-line tool for interacting with the OpenBook market on the Solana blockchain.

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
  Place a limit bid:
    openbook place --bid 100

  Cancel an order:
    openbook cancel --order 42

  Settle balances:
    openbook settle

For more information, visit: github.com/wiseaidev/openbook
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
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Place a limit bid.
    Place(Place),
    /// Cancel an order.
    Cancel(Cancel),
    /// Settle balance.
    Settle(Settle),
    /// Match orders transaction.
    Match(Match),
    /// Consume events instruction.
    Consume(Consume),
    /// Consume events permissioned instruction.
    ConsumePermissioned(ConsumePermissioned),
    /// Load orders for owner.
    Load(Load),
    /// Find open orders accounts for owner.
    Find(Find),
}

/// Represents options for placing a limit bid in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone)]
pub struct Place {
    /// Maximum bid amount.
    #[arg(short, long)]
    pub bid: u64,
}

/// Represents options for cancelling an order in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone)]
pub struct Cancel {
    /// Order ID to cancel.
    #[arg(short, long)]
    pub order: u64,
}

/// Represents options for settling balances in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone)]
pub struct Settle {
    /// Comming Soon: future options related to settling balances.
    #[arg(short, long)]
    pub future_option: u64,
}

/// Represents options for match orders transactions in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone)]
pub struct Match {
    /// The maximum number of orders to match.
    #[arg(short, long)]
    pub order: u16,
}

/// Represents options for consume events instructions in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone)]
pub struct Consume {
    /// Limit for consume events instruction.
    #[arg(short, long)]
    pub limit: u16,
}

/// Represents options for consume events permissioned instructions in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone)]
pub struct ConsumePermissioned {
    /// Limit for consume events permissioned instruction.
    #[arg(short, long)]
    pub limit: u16,
}

/// Represents options for loading orders for a specific owner in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone)]
pub struct Load {
    /// Number of orders to load.
    #[arg(short, long)]
    pub num: u64,
}

/// Represents options for finding open orders accounts for a specific owner in the OpenBook market.
#[cfg(feature = "cli")]
#[derive(Args, Debug, Clone)]
pub struct Find {
    /// Comming Soon: future options related to finding open orders accounts.
    #[arg(short, long)]
    pub future_option: String,
}
