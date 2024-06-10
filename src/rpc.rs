//! This module implements a thread safe client to interact with a remote Solana node.

use std::fmt;
use std::sync::Arc;

use anyhow::Result;
use backon::ExponentialBuilder;
use backon::Retryable;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_client::rpc_request::RpcError;
use solana_client::{
    client_error::ClientError,
    nonblocking::rpc_client::RpcClient,
    rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::{RpcAccountInfoConfig, RpcTransactionConfig},
    rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_rpc_client_api::client_error::ErrorKind;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Signer;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::transaction::Transaction;
use solana_sdk::{account::Account, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};

#[cfg(feature = "v2")]
use anchor_lang::{AccountDeserialize, Discriminator};

#[cfg(feature = "v2")]
use openbookdex_v2::state::OpenOrdersAccount;

#[cfg(feature = "v2")]
use solana_client::{
    rpc_config::RpcProgramAccountsConfig,
    rpc_filter::{Memcmp, RpcFilterType},
};

#[cfg(feature = "v2")]
use solana_account_decoder::UiAccountEncoding;

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
    ///     match rpc_client.fetch_signatures_for_address(&"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap(), Some(Signature::default()),
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

    #[cfg(feature = "v2")]
    pub async fn fetch_anchor_account<T: AccountDeserialize>(
        &self,
        address: &Pubkey,
    ) -> anyhow::Result<T> {
        let account = self.inner().get_account(address).await?;
        Ok(T::try_deserialize(&mut (&account.data as &[u8]))?)
    }

    #[cfg(feature = "v2")]
    pub async fn fetch_openbook_accounts(
        &self,
        program: Pubkey,
        owner: Pubkey,
    ) -> anyhow::Result<Vec<(Pubkey, OpenOrdersAccount)>> {
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![
                RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                    0,
                    OpenOrdersAccount::discriminator().to_vec(),
                )),
                RpcFilterType::Memcmp(Memcmp::new_raw_bytes(8, owner.to_bytes().to_vec())),
            ]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        };
        self.inner()
            .get_program_accounts_with_config(&program, config)
            .await?
            .into_iter()
            .map(|(key, account)| {
                Ok((
                    key,
                    OpenOrdersAccount::try_deserialize(&mut (&account.data as &[u8]))?,
                ))
            })
            .collect()
    }

    #[cfg(feature = "v2")]
    pub async fn fetch_anchor_accounts<T: AccountDeserialize + Discriminator>(
        &self,
        program: Pubkey,
    ) -> anyhow::Result<Vec<(Pubkey, T)>> {
        let account_type_filter =
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, T::discriminator().to_vec()));
        let config = RpcProgramAccountsConfig {
            filters: Some([vec![account_type_filter]].concat()),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        };
        self.inner()
            .get_program_accounts_with_config(&program, config)
            .await?
            .into_iter()
            .map(|(key, account)| Ok((key, T::try_deserialize(&mut (&account.data as &[u8]))?)))
            .collect()
    }

    pub async fn send_and_confirm(
        &self,
        owner: Keypair,
        instructions: Vec<Instruction>,
    ) -> anyhow::Result<(bool, Signature)> {
        let confirmed;
        let mut sig = Signature::default();
        let recent_hash = self
            .inner()
            .get_latest_blockhash_with_commitment(self.inner().commitment())
            .await?
            .0;
        let txn = Transaction::new_signed_with_payer(
            &instructions,
            Some(&owner.pubkey()),
            &[&owner],
            recent_hash,
        );

        match self
            .inner()
            .send_transaction_with_config(
                &txn,
                RpcSendTransactionConfig {
                    skip_preflight: false,
                    max_retries: None,
                    preflight_commitment: Some(self.inner().commitment().commitment),
                    encoding: None,
                    min_context_slot: None,
                },
            )
            .await
        {
            Ok(signature) => {
                match (|| async { self.inner().confirm_transaction(&signature).await })
                    .retry(&ExponentialBuilder::default())
                    .await
                {
                    Ok(_ret) => {
                        // Hack: We have received a signature. We assume it is confirmed due to the Solana network/Crank delay to get confirmation.
                        confirmed = true;
                        sig = signature;
                        tracing::debug!("transaction confirmed: {:?}", signature);
                    }
                    Err(err) => {
                        match err.kind() {
                            ErrorKind::Reqwest(reqwest_error) => {
                                if reqwest_error.is_timeout() {
                                    tracing::error!("Request timed out");
                                } else {
                                    tracing::error!("Reqwest error: {:?}", reqwest_error);
                                }
                            }
                            ErrorKind::RpcError(rpc_error) => match rpc_error {
                                RpcError::RpcRequestError(message) => {
                                    tracing::error!("RPC request error: {}", message);
                                }
                                RpcError::RpcResponseError {
                                    code,
                                    message,
                                    data,
                                } => {
                                    tracing::error!("RPC error code: {:?}", code);
                                    tracing::error!("RPC error message: {:?}", message);
                                    tracing::error!("RPC error data: {:?}", data);
                                }
                                RpcError::ParseError(message) => {
                                    tracing::error!("RPC parse error: {}", message);
                                }
                                RpcError::ForUser(message) => {
                                    tracing::error!("RPC error for user: {}", message);
                                }
                            },
                            _ => {
                                tracing::error!("Unexpected error: {:?}", err);
                            }
                        }
                        tracing::error!(
                            "Error occurred while processing instructions: {:?}",
                            instructions
                        );
                        confirmed = false;
                    }
                }
            }
            Err(err) => {
                match err.kind() {
                    ErrorKind::Reqwest(reqwest_error) => {
                        if reqwest_error.is_timeout() {
                            tracing::error!("Request timed out");
                        } else {
                            tracing::error!("Reqwest error: {:?}", reqwest_error);
                        }
                    }
                    ErrorKind::RpcError(rpc_error) => match rpc_error {
                        RpcError::RpcRequestError(message) => {
                            tracing::error!("RPC request error: {}", message);
                        }
                        RpcError::RpcResponseError {
                            code,
                            message,
                            data,
                        } => {
                            tracing::error!("RPC error code: {:?}", code);
                            tracing::error!("RPC error message: {:?}", message);
                            tracing::error!("RPC error data: {:?}", data);
                        }
                        RpcError::ParseError(message) => {
                            tracing::error!("RPC parse error: {}", message);
                        }
                        RpcError::ForUser(message) => {
                            tracing::error!("RPC error for user: {}", message);
                        }
                    },
                    _ => {
                        tracing::error!("Unexpected error: {:?}", err);
                    }
                }
                tracing::error!(
                    "Error occurred while processing instructions: {:?}",
                    instructions
                );
                confirmed = false;
            }
        };

        Ok((confirmed, sig))
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
