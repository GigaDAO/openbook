# 📖 OpenBook

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

### Get Market Info:

```sh
openbook info
```

### Place a limit bid:

```sh
openbook place --bid 100
```

### Cancel an order:

```sh
openbook cancel --bid 42
```

### Make match orders transaction:

```sh
openbook match --order 5
```

### Make consume events instruction:

```sh
openbook consume --limit 10
```

### Make consume events permissioned instruction:

```sh
openbook consume-permissioned --limit 15
```

### Load orders for owner:

```sh
openbook load --num 20
```

## 💻 Usage as Dependency

```toml
[dependencies]
openbook = "0.0.3"
```

```rust
use openbook::{pubkey::Pubkey, signature::Keypair, rpc_client::RpcClient};
use openbook::market::Market;
use openbook::utils::read_keypair;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    let key_path = std::env::var("KEY_PATH").expect("KEY_PATH is not set in .env file");
    
    let rpc_client = RpcClient::new(rpc_url);
    
    let keypair = read_keypair(&key_path);
    
    let mut market = Market::new(rpc_client, 3, "usdc", keypair).await;
    
    println!("Initialized Market: {:?}", market);

    let max_bid = 1;
    let r = market.place_limit_bid(max_bid).await?;
    println!("Place Order Results: {:?}", r);

    let order_id_to_cancel = 2;
    let c = market.cancel_order(order_id_to_cancel).await?;
    println!("Cancel Order Results: {:?}", c);

    let s = market.settle_balance().await?;
    println!("Settle Balance Results: {:?}", s);

    let m = market.make_match_orders_transaction(1).await?;
    println!("Match Order Results: {:?}", m);

    let open_orders_accounts = vec![Pubkey::new_from_array([0; 32])];
    let limit = 10;

    let e = market.make_consume_events_instruction(open_orders_accounts.clone(), limit).await?;
    println!("Consume Events Results: {:?}", e);

    let p = market.make_consume_events_permissioned_instruction(open_orders_accounts.clone(), limit).await?;
    println!("Consume Events Permissioned Results: {:?}", p);

    Ok(())
 }
```

## 🎨 Options

| Option                   | Default Value | Description                                              |
|--------------------------|---------------|----------------------------------------------------------|
| `place --bid <BID>`      | -         | Place a limit bid with the specified amount.             |
| `cancel --bid <BID>` | -             | Cancel an existing order with the given order ID.        |
| `settle`                 | -             | Settle balances in the OpenBook market.                  |
| `match --order <ORDER>` | -          | match orders transaction with the specified number of orders to match. |
| `consume --limit <LIMIT>` | -       | consume events instruction with the specified limit. |
| `consume-permissioned --limit <LIMIT>` | - | consume events permissioned instruction with the specified limit. |
| `load --num <NUM>`       | -             | Load orders for a specific owner with the specified number. |
| `find-open-accounts`     | -             | Find open orders accounts for a specific owner.           |

## 🤝 Contributing

Contributions and feedback are welcome! If you'd like to contribute, report an issue, or suggest an enhancement, please engage with the project on [GitHub](https://github.com/wiseaidev/openbook). Your contributions help improve this CLI and library for the community.

## 📄 License

This project is licensed under the [MIT License](LICENSE).
