use crate::utils::get_ts_precise;
use anyhow::{anyhow, Result};
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcBlockConfig, RpcBlockSubscribeConfig, RpcBlockSubscribeFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{UiConfirmedBlock, UiTransactionEncoding};
use std::sync::Arc;

use futures::stream::StreamExt;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

pub enum BlockStrategy {
    SlotFetch,
    BlocksWS,
    Geyser,
}

async fn monitor_blocks_ws(
    ws_rpc_url: &str,
    block_sender: mpsc::Sender<Option<(UiConfirmedBlock, i64, u64)>>,
) -> Result<u8> {
    let ws_rpc_url = ws_rpc_url.to_string();

    // Start subscription in separate task
    tokio::spawn(async move {
        let mut slot_notification_client;

        // loop for automatic reconnect
        loop {
            println!("Subscribing to block notifications");

            slot_notification_client = PubsubClient::new(&ws_rpc_url.to_string()).await.unwrap();

            let block_config = RpcBlockSubscribeConfig {
                encoding: Some(UiTransactionEncoding::Json), // perf: base64 > json >> base58 > binary
                transaction_details: Some(solana_transaction_status::TransactionDetails::Full),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
                show_rewards: None,
            };

            match slot_notification_client
                .block_subscribe(RpcBlockSubscribeFilter::All, Some(block_config))
                .await
            {
                Ok((mut slot_subscription, slot_unsubscribe)) => {
                    while let Some(slot_info) = slot_subscription.next().await {
                        let val = slot_info.value;
                        if val.block.is_none() {
                            // println!("{}", "Received block notification without block".red());
                            continue;
                        }

                        let slot = val.slot;

                        // TODO detect if blocks are sequential and fetch missing if it's the case
                        let ts_now = get_ts_precise();
                        let block = val.block.unwrap();
                        /*
                        let block_time = block.block_time.unwrap_or(0);
                        let diff_to_now = (ts_now / 1000) - block_time;
                        TODO log_metrics(slot, 0, diff_to_now, 0, 0);
                         */

                        let _ = block_sender.send(Some((block, ts_now, slot))).await;
                    }
                    println!("Websocket was killed - trying to reconnect");
                    slot_unsubscribe().await;
                }
                Err(e) => {
                    println!("Error subscribing to blocks: {:?}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    Ok(1)
}

pub async fn monitor_blocks(
    _rpc_client: &Arc<RpcClient>,
    ws_rpc_url: &str,
    block_sender: mpsc::Sender<Option<(UiConfirmedBlock, i64, u64)>>,
    strategy: BlockStrategy,
) -> Result<()> {
    println!("Monitoring blocks...");

    match strategy {
        BlockStrategy::SlotFetch => {
            // monitor_blocks_slot_fetch(rpc_client, ws_rpc_url, block_sender).await?;
        }
        BlockStrategy::BlocksWS => {
            monitor_blocks_ws(ws_rpc_url, block_sender).await?;
            return Ok(());
        }
        BlockStrategy::Geyser => {
            // monitor_blocks_geyser(rpc_client, ws_rpc_url, block_sender).await?;
        }
    }

    Ok(())
}

pub async fn get_block_with_retries(
    rpc_client: &Arc<RpcClient>,
    slot: u64,
    sleep_time_ms: u64,
    retries: Option<u8>,
) -> Result<Option<(UiConfirmedBlock, u8)>> {
    let block_config = RpcBlockConfig {
        encoding: Some(UiTransactionEncoding::Json), // perf: base64 > json >> base58 > binary
        transaction_details: Some(solana_transaction_status::TransactionDetails::Full),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
        rewards: None,
    };

    const GET_BLOCK_RETRIES: u8 = 7;

    let block_retries = retries.unwrap_or(GET_BLOCK_RETRIES);

    for block_retry in 0..block_retries {
        // wait X ms for block become available
        // TODO determine best value based on trial and error dynamically
        // retries are expensive so it might be worth sleeping more and have less retries
        sleep(Duration::from_millis(
            (block_retry + 1) as u64 * sleep_time_ms,
        ))
        .await;

        // get block
        let block = rpc_client.get_block_with_config(slot, block_config).await;

        match block {
            Ok(block) => {
                return Ok(Some((block, block_retry)));
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Block not available") {
                    continue;
                }
                if err_msg.contains("was skipped, or missing") {
                    return Ok(None);
                }

                println!("Error fetching block {}: {:?}", slot, e);
                if block_retry == block_retries - 1 {
                    return Err(anyhow!(
                        "Failed to fetch block after {} retries",
                        block_retries
                    ));
                }
            }
        }
    }

    Err(anyhow!(
        "Failed to fetch block after {} retries",
        block_retries
    ))
}

/*
async fn get_block_with_cache(
  slot: u64,
  block_cache: &Arc<dyn sol_lib::cache::JsonBlockCache>,
  rpc_client: &Arc<RpcClient>,
  rpc_semaphore: &Arc<Semaphore>,
  config: &DownloadConfig,
) -> Result<Option<UiConfirmedBlock>> {

  let DownloadConfig { max_retry_global, max_retry, sleep_duration_ms, data_location: _ } = config;

  let mut should_fetch = false;
  let mut block_retries: u8 = 0;

  let block = match block_cache.get_block(slot).await {
    Some(block) => {
      match block {
        CachedBlock::Success(block) => {
          Some(block)
        },
        CachedBlock::NotFound(EmptyBlock { slot: _, last_fetch: _, retries, error: _ }) => {
          // TODO based on error and retries we should decide if we retry to fetch block
          should_fetch = retries < *max_retry_global;
          block_retries = retries;
          // println!("Block not found in cache: {} - {}", slot, error);
          None
        }
      }
    },
    None => {
      // no attempts to download the block yet, or could not be deserialized
      should_fetch = true;
      // println!("Fetching block with rpc: {}", slot);
      None
    }
  };

  // we got a block from cache and are done here
  if let Some(block) = block {
    return Ok(Some(block));
  }

  // we got no block but it might just not exist or be available yet
  if !should_fetch {
    return Ok(None);
  }
  // else download block from rpc
  let permit = rpc_semaphore.clone().acquire_owned().await.unwrap();
  let block = get_block_with_retries(&rpc_client, slot, *sleep_duration_ms, Some(*max_retry)).await;

  // we get Ok(None) if block was skipped or missing, however sometimes the RPC finds those blocks later on
  match block {
    Ok(block) => {
      drop(permit);
      // println!("==> Got block or None: {}", slot);
      match block {
        Some((block, _)) => {
          let stored_block = CachedBlock::Success(block.clone());
          let _ = block_cache.set_block(slot, &stored_block).await;
          return Ok(Some(block));
        },
        None => {
          let stored_block = CachedBlock::NotFound(EmptyBlock {
            slot,
            retries: block_retries + 1,
            last_fetch: utils::get_ts_now(),
            error: "Block not available".to_string(),
          });
          let _ = block_cache.set_block(slot, &stored_block).await;
          return Ok(None);
        }
      }
    },
    Err(e) => {
      drop(permit);
      let stored_block = CachedBlock::NotFound(EmptyBlock {
        slot,
        retries: block_retries + 1,
        last_fetch: utils::get_ts_now(),
        error: e.to_string(),
      });
      let _ = block_cache.set_block(slot, &stored_block).await;
      return Ok(None);
    }
  };

}
 */
