use std::{str::FromStr, sync::Arc};

use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use anyhow::Result;

pub async fn get_transaction(rpc_client: &Arc<RpcClient>, signature: &str) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
  let sig = Signature::from_str(&signature)?;

  let config = RpcTransactionConfig {
    // encoding: Some(UiTransactionEncoding::JsonParsed),
    encoding: Some(UiTransactionEncoding::Json), 
    commitment: Some(CommitmentConfig::confirmed()),
    max_supported_transaction_version: Some(0),
  };
  
  let transaction = rpc_client.get_transaction_with_config(&sig, config).await?;
  Ok(transaction)
}