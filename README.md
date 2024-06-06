# 📖 OpenBook

[![Work In Progress](https://img.shields.io/badge/Work%20In%20Progress-red)](https://github.com/wiseaidev)
[![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-blue.svg)](https://www.rust-lang.org)
[![Maintenance](https://img.shields.io/badge/Maintained%3F-yes-green.svg)](https://github.com/wiseaidev)
[![CircleCI](https://dl.circleci.com/status-badge/img/gh/GigaDAO/openbook/tree/master.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/gh/GigaDAO/openbook/tree/master)
[![Crates.io](https://img.shields.io/crates/v/openbook.svg)](https://crates.io/crates/openbook)
[![Crates.io Downloads](https://img.shields.io/crates/d/openbook)](https://crates.io/crates/openbook)
[![docs](https://docs.rs/openbook/badge.svg)](https://docs.rs/openbook/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![Banner](https://github.com/GigaDAO/openbook/assets/62179149/ed83a9a8-4b8d-421d-be31-8eea73529444)

📖 A CLI, TUI and SDK to interact with the OpenBook market on the Solana blockchain.

> [!WARNING]  
> The current release is not yet production-ready. This project is still undergoing active development.

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

- Fetch market info in the OpenBook market.
- Place a limit bid in the OpenBook market.
- Cancel an existing order in the OpenBook market.
- Settle balances in the OpenBook market.
- Cancel settle place order in the OpenBook market.
- Cancel settle place bid order in the OpenBook market.
- Cancel settle place ask order in the OpenBook market.
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

## ⌨ Usage as TUI

https://github.com/GigaDAO/openbook/assets/62179149/23b411ac-243c-4f89-b8a2-fcc021eb9fdd

```sh
openbook tui
```

> [!NOTE]
> To trade on the openbook market, you need an open order account. This current release of this crate generates one for your wallet, but it's empty. You'll have to add funds to it. In the future, you can add funds through the TUI. But, if you already have funds in your open order account, set up this variable before starting the TUI:

```sh
export OOS_KEY=<your_associated_oo_sol_account>
```

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

### Cancel Settle Place Order:

```sh
openbook cancel-settle-place -u 5.0 -t 2.5 -p 5.0 -a 5.0
```

### Cancel Settle Place Bid Order:

```sh
openbook cancel-settle-place-bid -t 0.5 -b 15.0
```

### Cancel Settle Place Ask Order:

```sh
openbook cancel-settle-place-ask -t 0.5 -a 15.0
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

### OpenBook V1

```toml
[dependencies]
openbook = { version = "0.1.0" , features = ["v1"] } 
```

```rust
use openbook::v1::orders::OrderReturnType;
use openbook::v1::ob_client::OBClient;
use openbook::matching::Side;
use openbook::commitment_config::CommitmentConfig;
use openbook::pubkey::Pubkey;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let commitment = CommitmentConfig::confirmed();

    let market_id = "ASUyMMNBpFzpW3zDSPYdDVggKajq1DMKFFPK1JS9hoSR"
        .parse()
        .unwrap();

    let mut ob_client = OBClient::new(commitment, market_id, true, 1000).await?;
    println!("Initialized OpenBook V1 Client: {:?}", ob_client);

    println!("[*] Place Limit Order");
    if let Some(ord_ret_type) = ob_client
        .place_limit_order(
            0.1,
            Side::Bid, // or Side::Ask
            0.1,
            true,
            2.0,
        )
        .await?
    {
        match ord_ret_type {
            OrderReturnType::Instructions(insts) => {
                println!("[*] Got Instructions: {:?}", insts);
            }
            OrderReturnType::Signature(sign) => {
                println!("[*] Transaction successful, signature: {:?}", sign);
            }
        }
    }

    println!("[*] Cancel Orders");
    if let Some(ord_ret_type) = ob_client
        .cancel_orders(
            true
        )
        .await?
    {
        match ord_ret_type {
            OrderReturnType::Instructions(insts) => {
                println!("[*] Got Instructions: {:?}", insts);
            }
            OrderReturnType::Signature(sign) => {
                println!("[*] Transaction successful, signature: {:?}", sign);
            }
        }
    }

    println!("[*] Settle Balance");
    if let Some(ord_ret_type) = ob_client
        .settle_balance(
            true
        )
        .await?
    {
        match ord_ret_type {
            OrderReturnType::Instructions(insts) => {
                println!("[*] Got Instructions: {:?}", insts);
            }
            OrderReturnType::Signature(sign) => {
                println!("[*] Transaction successful, signature: {:?}", sign);
            }
        }
    }

    println!("[*] Cancel Settle Place Order");
    let result = ob_client
        .cancel_settle_place(
            10.0,
            0.5,
            15.0,
            1.3,
        )
        .await?;
    println!("[*] Transaction successful, signature: {:?}", result);

    println!("[*] Cancel Settle Place Bid Order");
    let result = ob_client
        .cancel_settle_place_bid(0.5, 15.0)
        .await?;
    println!("[*] Transaction successful, signature: {:?}", result);

    println!("[*] Cancel Settle Ask Order");
    let result = ob_client
        .cancel_settle_place_ask(0.5, 15.0)
        .await?;
    println!("[*] Transaction successful, signature: {:?}", result);

    let m = ob_client.make_match_orders_transaction(1).await?;
    println!("Match Order Result: {:?}", m);

    let open_orders_accounts = vec![Pubkey::new_from_array([0; 32])];
    let limit = 10;

    let e = ob_client.make_consume_events_instruction(open_orders_accounts.clone(), limit).await?;
    println!("Consume Events Result: {:?}", e);

    let p = ob_client.make_consume_events_permissioned_instruction(open_orders_accounts.clone(), limit).await?;
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
| `cancel-settle-place -u <USDC_ASK_TARGET> -b <TARGET_USDC_BID> -p <PRICE_JLP_USDC_BID> -a <ASK_PRICE_JLP_USDC>` | - | Cancel all limit orders, settle balances, and place new bid and ask orders. |
| `cancel-settle-place-bid -b <TARGET_SIZE_USDC_BID> -p <BID_PRICE_JLP_USDC>` | - | Cancel all limit orders, settle balances, and place a bid order. |
| `cancel-settle-place-ask -a <TARGET_SIZE_USDC_ASK> -p <ASK_PRICE_JLP_USDC>` | - | Cancel all limit orders, settle balances, and place an ask order. |
| `match --limit <LIMIT>`                | -             | Match orders transaction with the specified limit.      |
| `consume --limit <LIMIT>`              | -             | Consume events instruction with the specified limit.     |
| `consume-permissioned --limit <LIMIT>` | -             | Consume events permissioned instruction with the specified limit. |
| `load`                                 | -             | Load orders for the current owner, bids + asks.                      |
| `info`                                 | -             | Fetch OpenBook market info.                              |
| `tui`                                 | -             | launch tui.                              |

## 🤝 Contributing

Contributions and feedback are welcome! If you'd like to contribute, report an issue, or suggest an enhancement, please engage with the project on [GitHub](https://github.com/gigadao/openbook). Your contributions help improve this CLI and library for the community.

## 📄 License

This project is licensed under the [MIT License](LICENSE).
