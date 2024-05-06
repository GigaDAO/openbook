use openbook::pubkey::Pubkey;
use openbook::rpc::Rpc;
use openbook::rpc_client::RpcClient;
use openbook::signature::Signature;

#[tokio::test]
async fn test_fetch_transaction() {
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    let signature = Signature::default();
    let rpc = Rpc::new(RpcClient::new(rpc_url));
    let result = rpc.fetch_transaction(&signature).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_fetch_signatures_for_address() {
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    let rpc = Rpc::new(RpcClient::new(rpc_url));
    let result = rpc
        .fetch_signatures_for_address(&Pubkey::default(), None, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_fetch_multiple_accounts() {
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set in .env file");
    let rpc = Rpc::new(RpcClient::new(rpc_url));
    let result = rpc.fetch_multiple_accounts(&[Pubkey::default()]).await;
    assert!(result.is_ok());
}
