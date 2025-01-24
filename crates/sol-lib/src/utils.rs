use chrono::DateTime;

pub const WSOL: &str = "So11111111111111111111111111111111111111112";

pub fn get_ts_now() -> u64 {
    let now = std::time::SystemTime::now();
    let since_the_epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs()
}

pub fn get_ts_precise() -> i64 {
    let now = std::time::SystemTime::now();
    let since_the_epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_millis() as i64
}

pub fn get_approx_slot_diff(block_time: i64, ts: i64) -> i64 {
    // ~400 ms per slot
    // slots per sec != blocks per sec
    // empirical value to minimize requests
    let blocks_per_second: f64 = 1000.0 / 458.93;
    let ts_diff = (block_time - ts) as f64;

    (ts_diff * blocks_per_second).round() as i64
}

pub fn format_block_time(block_time: i64) -> String {
    let d = DateTime::from_timestamp(block_time, 0).unwrap();
    d.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn env_is_set(var_name: &str) -> bool {
    match std::env::var(var_name) {
        Ok(s) => s == "yes",
        _ => false,
    }
}

pub fn format_with_decimals(amount: u64, decimals: u8) -> f64 {
    let amount = amount as f64;
    amount / 10u64.pow(decimals as u32) as f64
}

#[cfg(test)]
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

#[cfg(test)]
pub async fn get_test_transaction(sig: &str) -> EncodedConfirmedTransactionWithStatusMeta {
    use crate::client::get_client;

    dotenvy::dotenv().ok();

    let rpc_url = std::env::var("solana_rpc_url")
        .unwrap_or("https://api.mainnet-beta.solana.com".to_string());
    let rpc_client = get_client(&rpc_url);

    // TODO cache transaction
    crate::transaction::tx::get_transaction(&rpc_client, sig)
        .await
        .unwrap()
}

#[cfg(test)]
use crate::transaction::wrapper::TransactionWrapper;
#[cfg(test)]
use arctis_types::BlockInfo;
#[cfg(test)]
use solana_transaction_status::UiCompiledInstruction;

#[cfg(test)]
pub struct TestData {
    pub tx: TransactionWrapper,
    pub block_info: BlockInfo,
    pub ix: UiCompiledInstruction,
}

#[cfg(test)]
pub async fn get_test_data(sig: &str, ix_index: usize) -> TestData {
    let tx = get_test_transaction(sig).await;
    let block_info = BlockInfo {
        slot: tx.slot,
        block_time: tx.block_time.unwrap(),
    };
    let tx = TransactionWrapper::new(tx.transaction);

    let top_level_ix = tx.get_instructions();
    let ix = top_level_ix[ix_index].clone();

    TestData { tx, block_info, ix }
}
