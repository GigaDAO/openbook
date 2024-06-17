<div align="center">

# 📖 OpenBook

[![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-blue.svg)](https://www.rust-lang.org)
[![Maintenance](https://img.shields.io/badge/Maintained%3F-yes-green.svg)](https://github.com/wiseaidev)
[![CircleCI](https://dl.circleci.com/status-badge/img/gh/GigaDAO/openbook/tree/master.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/gh/GigaDAO/openbook/tree/master)
[![Crates.io](https://img.shields.io/crates/v/openbook.svg)](https://crates.io/crates/openbook)
[![Crates.io Downloads](https://img.shields.io/crates/d/openbook)](https://crates.io/crates/openbook)
[![docs](https://docs.rs/openbook/badge.svg)](https://docs.rs/openbook/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[![GigaDAO Discord](https://dcbadge.limes.pink/api/server/gigadao-gigadex-now-live-920188005686337566)](https://discord.gg/gigadao-gigadex-now-live-920188005686337566)

| 🐧 Linux `(Recommended)` | 🪟 Windows |
| :------: | :--------: |
| ![Linux Banner](https://github.com/GigaDAO/openbook/assets/62179149/24912675-0f8e-4f5d-8933-56f47b9ba33b) | ![Windows Banner](https://github.com/GigaDAO/openbook/assets/62179149/c1874bab-300c-4867-a1f9-458fceb1efc2) |
| [Download Executable File](https://github.com/GigaDAO/openbook/releases/download/v0.1.0/openbook) | [Download `.exe` File](https://github.com/GigaDAO/openbook/releases/download/v0.1.0/openbook.exe) |
| `cargo install openbook --all-features` | `cargo install openbook --all-features` |

</div>

📖1️⃣2️⃣ A CLI, TUI and SDK to interact with OpenBook V1 and V2 markets on the Solana blockchain.

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

The following features are available on both OpenBook V1 and V2 markets:

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

### 📖 OpenBook 2️⃣

```sh
openbook
# same as `openbook v2`, the default is openbook v2
```

![OpenBook V2](https://github.com/GigaDAO/openbook/assets/62179149/24912675-0f8e-4f5d-8933-56f47b9ba33b)

> [!NOTE]
> To trade on the openbook v2 market, you will need an indexer and open order account. This current release of this crate generates one for your wallet, but it's empty. You'll have to add funds to it. In the future, you can add funds through the TUI. But, if you already have funds in your open order account, set up this variable before starting the TUI:

```sh
export INDEX=<your_associated_index_account>
export OOS_KEY=<your_associated_oo_sol_account>
```

### 📖 OpenBook 1️⃣

```sh
openbook v1
```

![OpenBook V1](https://github.com/GigaDAO/openbook/assets/62179149/aa68614d-086d-4439-b18b-105baa3c3f63)

> [!NOTE]
> To trade on the openbook v1 market, you will need an open order account. This current release of this crate generates one for your wallet, but it's empty. You'll have to add funds to it. In the future, you can add funds through the TUI. But, if you already have funds in your open order account, set up this variable before starting the TUI:

```sh
export OOS_KEY=<your_associated_oo_sol_account>
```

## ⌨ Usage as CLI

### 📖 OpenBook 1️⃣

#### Fetch Market Info:

```sh
openbook v1 info
```

<details>
<summary><code>Show Result</code></summary>

```sh
[*] OB_V1_Client {
    owner: Bugys1jBEBcFegRSjFvbpvNzJjo7qtdgSV7ytEYsWJ7Q
    rpc_client: RpcClient { commitment: CommitmentConfig { commitment: Confirmed } }
    quote_ata: 8onULHc8pHT7N7XnVfbkkLeU8mqEHnQbGwiBnTdVESio
    base_ata: 4P8mfc9dP7MxD5Uq9T5nxHD4GduVtCiKWYu8Nted8cXg
    open_orders: OpenOrders {
        oo_key: B8Vn7eYX1xMB1a3NanJfqhyEgWRfR5ptzscXnjv3qhN8
        min_ask: 29689
        max_bid: 28030
        open_asks: []
        open_bids: []
        bids_address: E9jHtpUqgTF2Ln8UhmyRXRNJsGKuNMVaSVaGowk9Vvr6
        asks_address: 6Kus1PbGpDRZ8R57PG2UM5b5vmyMp9wAHsXzsFQfPzsZ
        open_asks_prices: []
        open_bids_prices: []
        base_total: 0.0
        quote_total: 0.0
    }

    market_info: Market {
        program_id: srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX
        market_address: ASUyMMNBpFzpW3zDSPYdDVggKajq1DMKFFPK1JS9hoSR
        coin_decimals: 9
        pc_decimals: 6
        coin_lot_size: 100000
        account_flags: 3
        pc_lot_size: 10
        quote_mint: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
        base_mint: 27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4
        coin_vault: BNSehB5QgfqfUGa8N2GW7qwjGQD8sUjNUzTSEcmmupZL
        pc_vault: HstJUT5jehxW29UMUjockEWTVhwUhwxk1WQhHzPcZJXt
        vault_signer_key: G26Hizvx9zttK3Nu3n9oQouEoK89aeSqUQw6AKx4oWic
        event_queue: FM1a4He7jBDBQXfbUK35xpwf6tx2DfRYAzX48AkVcNqP
        request_queue: 7oGLtLJbcaTWQprDoYyCBTUW5n598qYRQP6KKw5DML4L
        bids_address: E9jHtpUqgTF2Ln8UhmyRXRNJsGKuNMVaSVaGowk9Vvr6
        asks_address: 6Kus1PbGpDRZ8R57PG2UM5b5vmyMp9wAHsXzsFQfPzsZ
        events_authority: 11111111111111111111111111111111
    }

}
```

</details>

#### Place a limit bid order:

```sh
openbook v1 place -t 2.93 -s bid -b 5.0 -p 5.0 -e
```

<details>
<summary><code>Show Result</code></summary>

```sh
[*] Transaction successful, signature: zcu668wmN5RFfCuiXuLXJMVJnPmAKt2PJ4NFvwprMYwgogVLKYRJ6M1bk3Fe1c5pJD1T4abLm6TUJPsfWZ8ux37
  Version: legacy
  Recent Blockhash: 8Lg3E4BqcT2gzMMk2R6kEDyTTXoBWa4JWBkFhx9ATRJC
  Signature 0: zcu668wmN5RFfCuiXuLXJMVJnPmAKt2PJ4NFvwprMYwgogVLKYRJ6M1bk3Fe1c5pJD1T4abLm6TUJPsfWZ8ux37
  Account 0: srw- Bugys1jBEBcFegRSjFvbpvNzJjo7qtdgSV7ytEYsWJ7Q (fee payer)
  Account 1: -rw- 6Kus1PbGpDRZ8R57PG2UM5b5vmyMp9wAHsXzsFQfPzsZ
  Account 2: -rw- 7oGLtLJbcaTWQprDoYyCBTUW5n598qYRQP6KKw5DML4L
  Account 3: -rw- 8onULHc8pHT7N7XnVfbkkLeU8mqEHnQbGwiBnTdVESio
  Account 4: -rw- ASUyMMNBpFzpW3zDSPYdDVggKajq1DMKFFPK1JS9hoSR
  Account 5: -rw- BNSehB5QgfqfUGa8N2GW7qwjGQD8sUjNUzTSEcmmupZL
  Account 6: -rw- E9jHtpUqgTF2Ln8UhmyRXRNJsGKuNMVaSVaGowk9Vvr6
  Account 7: -rw- FM1a4He7jBDBQXfbUK35xpwf6tx2DfRYAzX48AkVcNqP
  Account 8: -rw- HstJUT5jehxW29UMUjockEWTVhwUhwxk1WQhHzPcZJXt
  Account 9: -rw- J393rZhx4VcGaRA48N21T1EZmCJuCkxfzUo8mQWoY7LS
  Account 10: -r-- SysvarRent111111111111111111111111111111111
  Account 11: -r-- TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
  Account 12: -r-x srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX
  Instruction 0
    Program:   srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX (12)
    Account 0: ASUyMMNBpFzpW3zDSPYdDVggKajq1DMKFFPK1JS9hoSR (4)
    Account 1: J393rZhx4VcGaRA48N21T1EZmCJuCkxfzUo8mQWoY7LS (9)
    Account 2: 7oGLtLJbcaTWQprDoYyCBTUW5n598qYRQP6KKw5DML4L (2)
    Account 3: FM1a4He7jBDBQXfbUK35xpwf6tx2DfRYAzX48AkVcNqP (7)
    Account 4: E9jHtpUqgTF2Ln8UhmyRXRNJsGKuNMVaSVaGowk9Vvr6 (6)
    Account 5: 6Kus1PbGpDRZ8R57PG2UM5b5vmyMp9wAHsXzsFQfPzsZ (1)
    Account 6: 8onULHc8pHT7N7XnVfbkkLeU8mqEHnQbGwiBnTdVESio (3)
    Account 7: Bugys1jBEBcFegRSjFvbpvNzJjo7qtdgSV7ytEYsWJ7Q (0)
    Account 8: BNSehB5QgfqfUGa8N2GW7qwjGQD8sUjNUzTSEcmmupZL (5)
    Account 9: HstJUT5jehxW29UMUjockEWTVhwUhwxk1WQhHzPcZJXt (8)
    Account 10: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA (11)
    Account 11: SysvarRent111111111111111111111111111111111 (10)
    Data: [0, 10, 0, 0, 0, 0, 0, 0, 0, 50, 0, 0, 0, 0, 0, 0, 0, 228, 22, 0, 0, 0, 0, 0, 0, 80, 181, 44, 0, 0, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 26, 247, 58, 224, 156, 186, 128, 150, 255, 255, 168, 48, 100, 102, 0, 0, 0, 0]
  Status: Error processing Instruction 0: custom program error: 0x3f
    Fee: ◎0.000005
    Account 0 balance: ◎0.394319711 -> ◎0.394314711
    Account 1 balance: ◎0.12953952
    Account 2 balance: ◎0.00175392
    Account 3 balance: ◎0.00203928
    Account 4 balance: ◎0.00359136
    Account 5 balance: ◎0.00203928
    Account 6 balance: ◎0.12953952
    Account 7 balance: ◎0.31478688
    Account 8 balance: ◎0.00203928
    Account 9 balance: ◎0.02335776
    Account 10 balance: ◎0.0010092
    Account 11 balance: ◎0.93408768
    Account 12 balance: ◎0.00114144
  Compute Units Consumed: 6002
```

</details>

#### Place a limit ask order:

```sh
openbook v1 place -t 10.0 -s ask -b 0.5 -e -p 15.0
```

#### Cancel all limit orders:

```sh
openbook v1 cancel -e
```

#### Settle balances:

```sh
openbook v1 settle -e
```

#### Cancel Settle Place Order:

```sh
openbook v1 cancel-settle-place -u 5.0 -t 2.5 -p 5.0 -a 5.0
```

#### Cancel Settle Place Bid Order:

```sh
openbook v1 cancel-settle-place-bid -t 0.5 -b 15.0
```

#### Cancel Settle Place Ask Order:

```sh
openbook v1 cancel-settle-place-ask -t 0.5 -a 15.0
```

#### Fetch all orders for current owner (bids + asks):

```sh
openbook v1 load
```

#### Match orders transaction:

```sh
openbook v1 match --limit 3
```

#### Consume events instruction:

```sh
openbook v1 consume --limit 2
```

#### Consume events permissioned instruction:

```sh
openbook v1 consume-permissioned --limit 2
```

> [!TIP]
> Use `v1 --market-id` argument to overwrite the market id in the cli.

### 📖 OpenBook 2️⃣

#### Fetch Market Info:

```sh
openbook v2 info
```

> [!TIP]
> Use your own rpcpool or Helius for example to fetch the account openorder key, otherwise, you will get the following error:
> Error: HTTP status client error (410 Gone) for url (https://api.mainnet-beta.solana.com/)

<details>
<summary><code>Show Result</code></summary>

```sh
[*] OB_V2_Client {
    owner: Bugys1jBEBcFegRSjFvbpvNzJjo7qtdgSV7ytEYsWJ7Q
    rpc_client: RpcClient { commitment: CommitmentConfig { commitment: Confirmed } }
    quote_ata: 8onULHc8pHT7N7XnVfbkkLeU8mqEHnQbGwiBnTdVESio
    base_ata: 85zohW51yFvDhhwfuuFTkD6YGwmo8BugFx1aZYENWsu
    open_orders: [OpenOrders {
        order_id: 0
        is_buy: true
        price: 156.15699999999677
        amount: 3.2
        timestamp: 1717934030
        slot: 0
}
, OpenOrders {
        order_id: 9658973013853826267
        is_buy: true
        price: 2.9309999999966863
        amount: 341.1
        timestamp: 1717934416
        slot: 2
}
]
    market_info: MarketInfo {
    name: "SOL-USDC"
    base_decimals: 9
    quote_decimals: 6
    market_authority: B44ts4KVwst9dYSYqGB5vY4Wee2KB3AK3e92yEdJzwrw
    collect_fee_admin: J9zjCmmGBfv6wDSwmRW43vVJ6vooCftXjQYtc7uhETdr
    open_orders_admin: NonZeroPubkeyOption { key: 11111111111111111111111111111111 }
    consume_events_admin: NonZeroPubkeyOption { key: 11111111111111111111111111111111 }
    close_market_admin: NonZeroPubkeyOption { key: 11111111111111111111111111111111 }
    bids: Ad5skEiFoaeA27G3UhbpuwnFBCvmuuGEyoiijZhcd5xX
    asks: 53v47CBoaKwoM8tSEDN4oNyCc2ZJenDeuhMJTEw7fL2M
    event_heap: F7s6bScqRXB2gsU6s8QHSXJTmpS5t6SfVBs4V2k3HNKn
    oracle_a: NonZeroPubkeyOption { key: 11111111111111111111111111111111 }
    oracle_b: NonZeroPubkeyOption { key: 11111111111111111111111111111111 }
    oracle_conf_filter: 0.10000000149011612
    quote_lot_size: 1
    base_lot_size: 1000000
    seq_num: 39013687
    registration_time: 1705132794
    maker_fee: 1000
    taker_fee: 1000
    fees_accrued: 232787762473
    fees_to_referrers: 116401626607
    referrer_rebates_accrued: 179429
    fees_available: 1296315
    maker_volume: 116393905748447
    taker_volume_wo_oo: 116384694993127
    base_mint: So11111111111111111111111111111111111111112
    quote_mint: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
    market_base_vault: D4eZbLyKTtCk8jjbrkqTU2vADKUdGJLj1y4matSdeYW5
    base_deposit_total: 890149000000
    market_quote_vault: EA3Qa1WUxuY2BZo6b2Dy3ZxGiK3qS5hkeSwbCzUNomBm
    quote_deposit_total: 122924389076
}

    market_id: CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3
    index_account: DCwvnbmfeUCXKBc8xLFhBJwt6XPfmBkx4fKaJB4kskLi
    open_orders_account: DkST9PhuqfdxH5sBjt1171nbX71KbuAxrkw3GVv6ZjeW
    oo_state: OpenOrderState {
    asks base in oos: 0.0
    bids base in oos: 14.994665
    base free in oos: 0.032
    quote free in oos: 14.991413
}

    ata_balances: AtaBalances {
    quote balance: 12.91488
    base balance: 0.13096072
    total balance: 13.04584072
    price: 1.0
}

}
```

</details>

> [!TIP]
> Use `v2 --market-id` argument to overwrite the market id in the cli.

#### Place a limit bid order:

```sh
openbook v2 place -p 0.001 -s bid -t 1
```

<details>
<summary><code>Show Result</code></summary>

```sh
[*] Transaction successful, signature: 247o59Zq6whJQEKiRRU4gWKv4AUaX77WwciTeiCkhnaXth1k4gKmoLpgXgojGqPKjoxhRsShrqQEfNLge1hVceYG
2024-06-09T22:38:35.398259Z  INFO 
EncodedConfirmedTransactionWithStatusMeta { slot: 270899054, transaction: EncodedTransactionWithStatusMeta { transaction: Binary("ATS0gb/FEoRtWQfCfjNFaWbyUpSNLxHnuVUYL+o6H4e3F0lIqLlYS7nxnRXn/5qHmdP9qcra6VC+J8HiS/FjvQEBAAIKohRG/SHSqd4bNgk96sjOYpPd6KNxSlux7kYBS+FkB588LeWtpGPfntkqrLmTW/MQLVW9Nw1GwM8ucxVYKsxFSnP98LGDhJ/ybrUq8uWKvzWc3Qpkvvh8E4f2QpW0Nd1EjveagppmyVh9NXIGTw0Bu7mAX2BvMYmYkrlA15HQ9PSWgHlgvAz3Z3x8KWarha2BMyZsqEbzAmtDepM8+Mm5KacjXI27/y9eRSqBJTejbSZKPNm3H93t4JR5DNkAIT+qw3jg0kqnkNPjNhW2cX9TsNhBiL70MalsYDHszXbtA8DRxb79bp/33peN90XvJVUtvNyE0YS9gCLrvZLGUYMh9Qbd9uHXZaGT2cvhRs7reawctIXtX1s3kTqM9YV+/wCpC/6/vfur+tC0ZXG/lYweuCR4e7CZS7GEayl4Bx4Xmb6yEk0J06j3rHHL8r+QC2b6Vbzd29CWDuMlpNeRAkcMyQEJDAAECQIFAwEHBgkJCDQzwpuvbYJgagABAAAAAAAAAEBUiQAAAAAAKEYPAAAAAAAUvkfXL+UJSQI4gGdmAAAAAAIM", Base64), meta: Some(UiTransactionStatusMeta { err: None, status: Ok(()), fee: 5000, pre_balances: [380632810, 633916800, 2039280, 633916800, 9688320, 6792960, 2039280, 636255360, 934087680, 1141440], post_balances: [380627810, 633916800, 2039280, 633916800, 9688320, 6792960, 2039280, 636255360, 934087680, 1141440], inner_instructions: Some([]), log_messages: Some(["Program opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb invoke [1]", "Program log: Instruction: PlaceOrder", "Program opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb consumed 27535 of 200000 compute units", "Program return: opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb AZt1rP3/////AQAAAAAAAAA=", "Program opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb success"]), pre_token_balances: Some([UiTransactionTokenBalance { account_index: 2, mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", ui_token_amount: UiTokenAmount { ui_amount: Some(12.91488), decimals: 6, amount: "12914880", ui_amount_string: "12.91488" }, owner: Some("Bugys1jBEBcFegRSjFvbpvNzJjo7qtdgSV7ytEYsWJ7Q"), program_id: Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") }, UiTransactionTokenBalance { account_index: 6, mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", ui_token_amount: UiTokenAmount { ui_amount: Some(111009.20637), decimals: 6, amount: "111009206370", ui_amount_string: "111009.20637" }, owner: Some("B44ts4KVwst9dYSYqGB5vY4Wee2KB3AK3e92yEdJzwrw"), program_id: Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") }]), post_token_balances: Some([UiTransactionTokenBalance { account_index: 2, mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", ui_token_amount: UiTokenAmount { ui_amount: Some(12.91488), decimals: 6, amount: "12914880", ui_amount_string: "12.91488" }, owner: Some("Bugys1jBEBcFegRSjFvbpvNzJjo7qtdgSV7ytEYsWJ7Q"), program_id: Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") }, UiTransactionTokenBalance { account_index: 6, mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", ui_token_amount: UiTokenAmount { ui_amount: Some(111009.20637), decimals: 6, amount: "111009206370", ui_amount_string: "111009.20637" }, owner: Some("B44ts4KVwst9dYSYqGB5vY4Wee2KB3AK3e92yEdJzwrw"), program_id: Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") }]), rewards: Some([]), loaded_addresses: Some(UiLoadedAddresses { writable: [], readonly: [] }), return_data: Some(UiTransactionReturnData { program_id: "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb", data: ("AZt1rP3/////AQAAAAAAAAA=", Base64) }), compute_units_consumed: Some(27535) }), version: Some(Legacy(Legacy)) }, block_time: Some(1717972675) }
  Version: legacy
  Recent Blockhash: Cz7kpy1Rt8Ub1sedTGnsBYXVmaa6fcYFPxkFhPX59TFv
  Signature 0: 247o59Zq6whJQEKiRRU4gWKv4AUaX77WwciTeiCkhnaXth1k4gKmoLpgXgojGqPKjoxhRsShrqQEfNLge1hVceYG
  Account 0: srw- Bugys1jBEBcFegRSjFvbpvNzJjo7qtdgSV7ytEYsWJ7Q (fee payer)
  Account 1: -rw- 53v47CBoaKwoM8tSEDN4oNyCc2ZJenDeuhMJTEw7fL2M
  Account 2: -rw- 8onULHc8pHT7N7XnVfbkkLeU8mqEHnQbGwiBnTdVESio
  Account 3: -rw- Ad5skEiFoaeA27G3UhbpuwnFBCvmuuGEyoiijZhcd5xX
  Account 4: -rw- B8Vn7eYX1xMB1a3NanJfqhyEgWRfR5ptzscXnjv3qhN8
  Account 5: -rw- CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3
  Account 6: -rw- EA3Qa1WUxuY2BZo6b2Dy3ZxGiK3qS5hkeSwbCzUNomBm
  Account 7: -rw- F7s6bScqRXB2gsU6s8QHSXJTmpS5t6SfVBs4V2k3HNKn
  Account 8: -r-- TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
  Account 9: -r-x opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb
  Instruction 0
    Program:   opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb (9)
    Account 0: Bugys1jBEBcFegRSjFvbpvNzJjo7qtdgSV7ytEYsWJ7Q (0)
    Account 1: B8Vn7eYX1xMB1a3NanJfqhyEgWRfR5ptzscXnjv3qhN8 (4)
    Account 2: opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb (9)
    Account 3: 8onULHc8pHT7N7XnVfbkkLeU8mqEHnQbGwiBnTdVESio (2)
    Account 4: CFSMrBssNG8Ud1edW59jNLnq2cwrQ9uY5cM3wXmqRJj3 (5)
    Account 5: Ad5skEiFoaeA27G3UhbpuwnFBCvmuuGEyoiijZhcd5xX (3)
    Account 6: 53v47CBoaKwoM8tSEDN4oNyCc2ZJenDeuhMJTEw7fL2M (1)
    Account 7: F7s6bScqRXB2gsU6s8QHSXJTmpS5t6SfVBs4V2k3HNKn (7)
    Account 8: EA3Qa1WUxuY2BZo6b2Dy3ZxGiK3qS5hkeSwbCzUNomBm (6)
    Account 9: opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb (9)
    Account 10: opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb (9)
    Account 11: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA (8)
    Data: [51, 194, 155, 175, 109, 130, 96, 106, 0, 1, 0, 0, 0, 0, 0, 0, 0, 64, 84, 137, 0, 0, 0, 0, 0, 40, 70, 15, 0, 0, 0, 0, 0, 20, 190, 71, 215, 47, 229, 9, 73, 2, 56, 128, 103, 102, 0, 0, 0, 0, 2, 12]
  Status: Ok
    Fee: ◎0.000005
    Account 0 balance: ◎0.38063281 -> ◎0.38062781
    Account 1 balance: ◎0.6339168
    Account 2 balance: ◎0.00203928
    Account 3 balance: ◎0.6339168
    Account 4 balance: ◎0.00968832
    Account 5 balance: ◎0.00679296
    Account 6 balance: ◎0.00203928
    Account 7 balance: ◎0.63625536
    Account 8 balance: ◎0.93408768
    Account 9 balance: ◎0.00114144
  Compute Units Consumed: 27535
  Log Messages:
    Program opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb invoke [1]
    Program log: Instruction: PlaceOrder
    Program opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb consumed 27535 of 200000 compute units
    Program return: opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb AZt1rP3/////AQAAAAAAAAA=
    Program opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb success
  Return Data from Program opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb:
    Length: 17 (0x11) bytes
0000:   01 9b 75 ac  fd ff ff ff  ff 01 00 00  00 00 00 00   ..u.............
0010:   00
```

</details>

## 💻 Usage as Dependency

### OpenBook V1

```toml
[dependencies]
openbook = { version = "0.1.0" , features = ["v1"] } 
```

```rust , ignore
use openbook::v1::orders::OrderReturnType;
use openbook::v1::ob_client::OBClient;
use openbook::matching::Side;
use openbook::commitment_config::CommitmentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let commitment = CommitmentConfig::confirmed();

    let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6"
        .parse()
        .unwrap();

    let mut ob_client = OBClient::new(
        commitment,
        market_id,
        true,
        1000
    ).await?;

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

    let m = ob_client.match_orders_transaction(1).await?;
    println!("Match Order Result: {:?}", m);

    let open_orders_accounts = vec![ob_client.open_orders.oo_key];
    let limit = 10;

    let e = ob_client.consume_events_instruction(open_orders_accounts.clone(), limit).await?;
    println!("Consume Events Result: {:?}", e);

    let p = ob_client.consume_events_permissioned_instruction(open_orders_accounts.clone(), limit).await?;
    println!("Consume Events Permissioned Result: {:?}", p);

    Ok(())
}
```

### OpenBook V2

```toml
[dependencies]
openbook = { version = "0.1.0" , features = ["v2"] } 
```

```rust , ignore
use openbook::v2::ob_client::OBClient;
use openbook::v2_state::Side;
use openbook::commitment_config::CommitmentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let commitment = CommitmentConfig::confirmed();

    let market_id = "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6"
        .parse()
        .unwrap();

    let mut ob_client = OBClient::new(
        commitment,
        market_id,
        true, // Create new indexer and open orders accounts
        true // load all info related to the open orders account
    ).await?;

    println!("Initialized OpenBook V1 Client: {:?}", ob_client);

    println!("[*] Place Limit Order");

    let (_confirmed, signature, order_id, slot) = ob_client_v2
        .place_limit_order(
            0.003, // price
            5, // amount
            Side::Bid,
        )
        .await?;

    Ok(())
}
```

## 🎨 Top Level Command

| Commands                                | Description                                              |
|----------------------------------------|----------------------------------------------------------|
| `v1` | - | OpenBook V1 client.       |
| `v2` | - | OpenBook V2 client.       |

### V1 SubCommands

| SubCommands                                 | Default Value | Description                                              |
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
| `info`                                 | -             | Fetch OpenBook V1 market info.                              |

### V2 SubCommands

| SubCommands                                 | Default Value | Description                                              |
|----------------------------------------|---------------|----------------------------------------------------------|
| `info`                                 | -             | Fetch OpenBook V2 market info.                              |

## 🤝 Contributing

Contributions and feedback are welcome! If you'd like to contribute, report an issue, or suggest an enhancement, please engage with the project on [GitHub](https://github.com/gigadao/openbook). Your contributions help improve this CLI and library for the community.

## 📄 License

This project is licensed under the [MIT License](LICENSE).
