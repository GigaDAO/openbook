# 📖 OpenBook

[![CircleCI](https://dl.circleci.com/status-badge/img/gh/wiseaidev/openbook/tree/master.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/gh/wiseaidev/openbook/tree/master)
[![Crates.io](https://img.shields.io/crates/v/openbook.svg)](https://crates.io/crates/openbook)
[![docs](https://docs.rs/openbook/badge.svg)](https://docs.rs/openbook/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> A CLI and library for interacting with the OpenBook market on the Solana blockchain.

## Table of Contents

- [Installation](#-installation)
- [Functionalities](#-functionalities)
- [Usage](#-usage-as-cli)
- [Usage as Dependency](#-usage-as-dep)
- [Options](#-options)
- [Contributing](#-contributing)
- [License](#-license)

## 🚀 Installation

To install `openbook` cli, use the following Cargo command:

```bash
cargo install --locked openbook --all-features
```

## ✨ Functionalities

- Place a limit bid in the OpenBook market.
- Cancel an existing order in the OpenBook market.
- Settle balances in the OpenBook market.
- Consume events instructions in the OpenBook market.
- Consume events permissioned instructions in the OpenBook market.
- Load orders for a specific owner in the OpenBook market.
- Find open orders accounts for a specific owner in the OpenBook market.

## Usage

Before using the `openbook` crate or CLI, make sure to set the following environment variables:

```bash
export RPC_URL=https://api.mainnet-beta.solana.com
export KEY_PATH=<path_to_your_key_file>
```

> [!NOTE]
> Certain RPC methods, like `getProgramAccounts`, are no longer available on `api.mainnet-beta.solana.com`. We recommend utilizing [helius.dev](https://www.helius.dev) as an alternative.

## ⌨ Usage as CLI

### Fetch Market Info:

```sh
openbook info
```

### Place a limit bid order:

```sh
openbook place -t 10.0 -s bid -b 0.5 -e -p 15.0
```

### Place a limit ask order:

```sh
openbook place -t 10.0 -s ask -b 0.5 -e -p 15.0
```

### Cancel all limit orders:

```sh
openbook cancel -e
```

### Settle balances:

```sh
openbook settle -e
```

### Fetch all orders for current owner (bids + asks):

```sh
openbook load
```

### Make match orders transaction:

```sh
openbook match --limit 3
```

### Make consume events instruction:

```sh
openbook consume --limit 2
```

### Make consume events permissioned instruction:

```sh
openbook consume-permissioned --limit 2
```

## 💻 Usage as Dependency

```toml
[dependencies]
openbook = "0.0.5"
```

```rust
use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
use openbook::market::Market;
use openbook::utils::read_keypair;
use openbook::matching::Side;
use openbook::commitment_config::CommitmentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");

    let commitment_config = CommitmentConfig::confirmed();
    let rpc_client = RpcClient::new_with_commitment(rpc_url, commitment_config);

    let keypair = read_keypair(&key_path);

    let mut market = Market::new(rpc_client, 3, "usdc", keypair).await;

    println!("Initialized Market: {:?}", market);

    let r = market
        .place_limit_order(
            10.0,
            Side::Bid, // or Side::Ask
            0.5,
            true,
            15.0,
        )
    .await?;
    println!("Place Limit Order Result: {:?}", r);

    let c = market.cancel_orders(true).await?;
    println!("Cancel Orders Result: {:?}", c);

    let s = market.settle_balance(true).await?;
    println!("Settle Balance Result: {:?}", s);

    let m = market.make_match_orders_transaction(1).await?;
    println!("Match Order Result: {:?}", m);

    let open_orders_accounts = vec![Pubkey::new_from_array([0; 32])];
    let limit = 10;

    let e = market.make_consume_events_instruction(open_orders_accounts.clone(), limit).await?;
    println!("Consume Events Result: {:?}", e);

    let p = market.make_consume_events_permissioned_instruction(open_orders_accounts.clone(), limit).await?;
    println!("Consume Events Permissioned Result: {:?}", p);

    Ok(())
 }
```

## 🎨 Options

| Option                                 | Default Value | Description                                              |
|----------------------------------------|---------------|----------------------------------------------------------|
| `place -t <TARGET_AMOUNT_QUOTE> -s <SIDE> -b <BEST_OFFSET_USDC> -e -p <PRICE_TARGET>` | - | Place a limit order with the specified parameters.       |
| `cancel -e`                            | -             | Cancel all existing order for the current owner.        |
| `settle -e`                            | -             | Settle balances in the OpenBook market.                  |
| `match --limit <LIMIT>`                | -             | Match orders transaction with the specified limit.      |
| `consume --limit <LIMIT>`              | -             | Consume events instruction with the specified limit.     |
| `consume-permissioned --limit <LIMIT>` | -             | Consume events permissioned instruction with the specified limit. |
| `find --future_option <FUTURE_OPTION>` | -             | Find open orders accounts for a specific owner.          |
| `load`                                 | -             | Load orders for the current owner, bids + asks.                      |
| `info`                                 | -             | Fetch OpenBook market info.                              |

## 🤝 Contributing

Contributions and feedback are welcome! If you'd like to contribute, report an issue, or suggest an enhancement, please engage with the project on [GitHub](https://github.com/wiseaidev/openbook). Your contributions help improve this CLI and library for the community.

## 📄 License

This project is licensed under the [MIT License](LICENSE).


Unresolved Questions:
- After completing all the tasks, should I transfer the repo ownership to the `@GigaDAO-GigaDEX` orginization? or create a new one?
