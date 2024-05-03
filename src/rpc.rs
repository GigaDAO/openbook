//! This module implements a thread safe client to interact with a remote Solana node.

use solana_account_decoder::UiAccountEncoding;
use std::fmt;
use std::sync::Arc;

use anyhow::Result;
use backon::ExponentialBuilder;
use backon::Retryable;
use solana_client::{
    client_error::ClientError,
    nonblocking::rpc_client::RpcClient,
    rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcTransactionConfig},
    rpc_filter::RpcFilterType,
    rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_sdk::{account::Account, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};

/// Wrapper type for RpcClient providing additional functionality and enabling Debug trait implementation.
///
/// This struct holds an `Arc` of `RpcClient` to ensure thread safety and efficient resource sharing.
#[derive(Clone)]
pub struct Rpc(Arc<RpcClient>);

impl Rpc {
    /// Constructs a new Rpc wrapper around the provided RpcClient instance.
    ///
    /// # Parameters
    ///
    /// - `rpc_client`: An instance of RpcClient to wrap.
    ///
    /// # Returns
    ///
    /// A new Rpc wrapper around the provided RpcClient.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::rpc_client::RpcClient;
    /// use openbook::rpc::Rpc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set");
    ///
    ///     let connection = RpcClient::new(rpc_url);
    ///     let rpc_client = Rpc::new(connection);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn new(rpc_client: RpcClient) -> Self {
        Rpc(Arc::new(rpc_client))
    }

    /// Returns a reference to the inner RpcClient instance wrapped by this wrapper.
    pub fn inner(&self) -> &RpcClient {
        &self.0
    }

    /// Retrieves a transaction with the specified signature.
    ///
    /// # Parameters
    ///
    /// - `signature`: The signature of the transaction to retrieve.
    ///
    /// # Returns
    ///
    /// The transaction details if found, or an error otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::signature::Signature;
    /// use openbook::rpc_client::RpcClient;
    /// use openbook::rpc::Rpc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set");
    ///
    ///     let connection = RpcClient::new(rpc_url);
    ///     let rpc_client = Rpc::new(connection);
    ///
    ///     match rpc_client.fetch_transaction(&Signature::default()).await {
    ///             Ok(accounts) => println!("Filtered accounts: {:?}", accounts),
    ///             Err(err) => eprintln!("Error getting filtered accounts: {:?}", err),
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch_transaction(
        &self,
        signature: &Signature,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, ClientError> {
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            max_supported_transaction_version: Some(0),
            commitment: Some(self.inner().commitment()),
        };

        self.inner()
            .get_transaction_with_config(signature, config)
            .await
    }

    /// Retrieves confirmed transaction signatures associated with a specific address.
    ///
    /// # Parameters
    ///
    /// - `pubkey`: The public key of the address.
    /// - `before`: Optional. Limit results to transactions before this signature.
    /// - `until`: Optional. Limit results to transactions until this signature.
    ///
    /// # Returns
    ///
    /// A list of confirmed transaction signatures, or an error otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::signature::Signature;
    /// use openbook::rpc_client::RpcClient;
    /// use openbook::rpc::Rpc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set");
    ///
    ///     let connection = RpcClient::new(rpc_url);
    ///     let rpc_client = Rpc::new(connection);
    ///
    ///     match rpc_client.fetch_signatures_for_address(&anchor_spl::token::ID, Some(Signature::default()),
    ///         Some(Signature::default())).await {
    ///             Ok(accounts) => println!("Filtered accounts: {:?}", accounts),
    ///             Err(err) => eprintln!("Error getting filtered accounts: {:?}", err),
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch_signatures_for_address(
        &self,
        pubkey: &Pubkey,
        before: Option<Signature>,
        until: Option<Signature>,
    ) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>, ClientError> {
        (|| async {
            let config = GetConfirmedSignaturesForAddress2Config {
                before,
                until,
                commitment: Some(self.inner().commitment()),
                ..GetConfirmedSignaturesForAddress2Config::default()
            };
            self.inner()
                .get_signatures_for_address_with_config(pubkey, config)
                .await
        })
        .retry(&ExponentialBuilder::default())
        .await
    }

    /// Retrieves program accounts associated with a specific program.
    ///
    /// # Parameters
    ///
    /// - `program`: The public key of the program.
    /// - `filters`: Optional. Filters to apply to the accounts.
    ///
    /// # Returns
    ///
    /// A list of program accounts, or an error otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::rpc_filter::{RpcFilterType, Memcmp};
    /// use openbook::rpc::Rpc;
    /// use openbook::rpc_client::RpcClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set");
    ///
    ///     let connection = RpcClient::new(rpc_url);
    ///     let rpc_client = Rpc::new(connection);
    ///
    ///     let filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, vec![1u8]))];
    ///
    ///     match rpc_client.fetch_program_accounts(&anchor_spl::token::ID, Some(filters)).await {
    ///         Ok(accounts) => println!("Filtered accounts: {:?}", accounts),
    ///         Err(err) => eprintln!("Error getting filtered accounts: {:?}", err),
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch_program_accounts(
        &self,
        program: &Pubkey,
        filters: Option<Vec<RpcFilterType>>,
    ) -> Result<Vec<(Pubkey, Account)>, ClientError> {
        (|| async {
            let filters = filters.clone();

            let config = RpcProgramAccountsConfig {
                filters,
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(self.inner().commitment()),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            };
            self.inner()
                .get_program_accounts_with_config(program, config)
                .await
        })
        .retry(&ExponentialBuilder::default())
        .await
    }

    /// Retrieves multiple accounts associated with specified public keys.
    ///
    /// # Parameters
    ///
    /// - `pubkeys`: An array of public keys.
    ///
    /// # Returns
    ///
    /// A list of optional accounts, or an error otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use openbook::pubkey::Pubkey;
    /// use openbook::signature::Signature;
    /// use openbook::rpc_client::RpcClient;
    /// use openbook::rpc::Rpc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set");
    ///
    ///     let connection = RpcClient::new(rpc_url);
    ///     let rpc_client = Rpc::new(connection);
    ///
    ///     match rpc_client.fetch_multiple_accounts(&[Pubkey::default()]).await {
    ///             Ok(accounts) => println!("Filtered accounts: {:?}", accounts),
    ///             Err(err) => eprintln!("Error getting filtered accounts: {:?}", err),
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, ClientError> {
        Ok((|| async {
            let config = RpcAccountInfoConfig {
                commitment: Some(self.inner().commitment()),
                ..RpcAccountInfoConfig::default()
            };

            self.inner()
                .get_multiple_accounts_with_config(pubkeys, config)
                .await
        })
        .retry(&ExponentialBuilder::default())
        .await?
        .value)
    }
}

/// Implement the Debug trait for the wrapper type `Rpc`.
///
/// This implementation enables the `Rpc` struct to be debugged, providing relevant
/// information about the underlying `RpcClient`.
impl fmt::Debug for Rpc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Include relevant information about RpcClient
        f.debug_struct("RpcClient")
            .field("commitment", &self.inner().commitment())
            .finish()
    }
}
