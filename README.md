# 📖 OpenBook

[![CircleCI](https://dl.circleci.com/status-badge/img/gh/gigadao/openbook/tree/master.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/gh/gigadao/openbook/tree/master)
[![Crates.io](https://img.shields.io/crates/v/openbook.svg)](https://crates.io/crates/openbook)
[![docs](https://docs.rs/openbook/badge.svg)](https://docs.rs/openbook/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

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

Before using the `openbook` crate or CLI ot TUI, make sure to set the following environment variables:

```bash
export RPC_URL=https://api.mainnet-beta.solana.com
export KEY_PATH=<path_to_your_key_file>
```

> [!NOTE]
> Certain RPC methods, like `getProgramAccounts`, are no longer available on `api.mainnet-beta.solana.com`. We recommend utilizing [helius.dev](https://www.helius.dev) as an alternative.

## ⌨ Usage as TUI

https://github.com/GigaDAO/openbook/assets/62179149/23b411ac-243c-4f89-b8a2-fcc021eb9fdd

```sh
openbook tui
```

> [!NOTE]
> To interact with the openbook market by placing bids or asking, you'll need to set up an open order account for your wallet. In future releases, this crate will automatically fetch your associated open order account. However, for now, if you already have one, you'll need to set up this environment variable before launching the tui:

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
openbook cancel-settle-place -u 10.0 -t 0.5 -p 15.0 -a 1.3
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

```toml
[dependencies]
openbook = "0.0.10"
```

```rust
use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
use openbook::market::Market;
use openbook::utils::read_keypair;
use openbook::matching::Side;
use openbook::commitment_config::CommitmentConfig;
use openbook::market::OrderReturnType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");

    let commitment_config = CommitmentConfig::confirmed();
    let rpc_client = RpcClient::new_with_commitment(rpc_url, commitment_config);
    
    let keypair = read_keypair(&key_path);
    
    let mut market = Market::new(rpc_client, 3, "jlp", "usdc", keypair, true).await;

    println!("Initialized Market: {:?}", market);

    println!("[*] Place Limit Order");
    if let Some(ord_ret_type) = market
        .place_limit_order(
            10.0,
            Side::Bid, // or Side::Ask
            0.5,
            true,
            15.0,
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
    if let Some(ord_ret_type) = market
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
    if let Some(ord_ret_type) = market
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
    let result = market
        .cancel_settle_place(
            10.0,
            0.5,
            15.0,
            1.3,
        )
        .await?;
    println!("[*] Transaction successful, signature: {:?}", result);

    println!("[*] Cancel Settle Place Bid Order");
    let result = market
        .cancel_settle_place_bid(0.5, 15.0)
        .await?;
    println!("[*] Transaction successful, signature: {:?}", result);

    println!("[*] Cancel Settle Ask Order");
    let result = market
        .cancel_settle_place_ask(0.5, 15.0)
        .await?;
    println!("[*] Transaction successful, signature: {:?}", result);

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
