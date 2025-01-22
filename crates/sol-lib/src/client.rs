use std::sync::Arc;

pub use solana_client::nonblocking::rpc_client::RpcClient;

pub fn get_client(rpc_url: &str) -> Arc<RpcClient> {
    let client = RpcClient::new(rpc_url.to_string());
    Arc::new(client)
}
