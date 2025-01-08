use anyhow::{Result, anyhow};
use sol_db::solana_db::{ProcessedTransaction, SolanaDatabase};
use sol_lib::{blocks::get_block_with_retries, client::get_client, transaction::tx::get_transaction};

use crate::parse::{self, block::process_block};

pub struct ExecutionContext {
  pub rpc_url: String,
  pub ws_url: String,
}

pub async fn parse_block(block_number: u64, ctx: &ExecutionContext) -> Result<SolanaDatabase> {
  let rpc_client = get_client(&ctx.rpc_url);
  let block = get_block_with_retries(&rpc_client, block_number, 200, None).await?;
  match block {
    Some((block, _)) => {
      let mut sol_db = SolanaDatabase::new()?;
      let _ = process_block(&block, &mut sol_db);
      Ok(sol_db)
    },
    None => {
      return Err(anyhow!("Block not found"));
    }
  }
}

pub async fn parse_transaction(tx_id: &str, ctx: &ExecutionContext) -> Result<ProcessedTransaction> {
  let rpc_client = get_client(&ctx.rpc_url);
  let tx = get_transaction(&rpc_client, tx_id).await?;
  let block_time = tx.block_time.unwrap();
  let slot = tx.slot;
  let transaction = tx.transaction;

  let result = parse::transaction::process_transaction(&transaction, slot, block_time)?;
  Ok(result)
}

pub async fn monitor_blocks(ctx: &ExecutionContext) -> Result<()> {
  println!("Monitoring blocks...");
  let rpc_client = get_client(&ctx.rpc_url);
  let slot = rpc_client.get_slot().await?;
  println!("Current slot: {}", slot);
  Err(anyhow!("Not implemented"))
}

