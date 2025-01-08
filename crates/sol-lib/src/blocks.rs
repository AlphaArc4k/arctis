

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
      sleep(Duration::from_millis((block_retry + 1) as u64 * sleep_time_ms)).await;
  
      // get block
      let block = rpc_client
          .get_block_with_config(slot, block_config)
          .await;
  
      match block {
        Ok(block) => {
          return Ok(Some((block, block_retry)));
        },
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
            return Err(anyhow!("Failed to fetch block after {} retries", block_retries));
          }
        }
      }
  
    }
  
    return Err(anyhow!("Failed to fetch block after {} retries", block_retries));
  
  }


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