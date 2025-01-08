

pub fn get_ts_now() -> u64 {
    let now = std::time::SystemTime::now();
    let since_the_epoch = now.duration_since(std::time::UNIX_EPOCH).expect("Time went backwards");
    since_the_epoch.as_secs()
}
  
pub fn get_ts_precise() -> i64 {
    let now = std::time::SystemTime::now();
    let since_the_epoch = now.duration_since(std::time::UNIX_EPOCH).expect("Time went backwards");
    since_the_epoch.as_millis() as i64
}
  
pub fn get_approx_slot_diff(block_time: i64, ts: i64) -> i64 {
    // ~400 ms per slot
    // slots per sec != blocks per sec
    // empirical value to minimize requests
    let blocks_per_second: f64 = 1000.0 / 458.93;
    let ts_diff = (block_time - ts) as f64;
    let slot_diff = (ts_diff * blocks_per_second).round() as i64;
    slot_diff
}
  
pub fn format_block_time(block_time: i64) -> String {
    let d = DateTime::from_timestamp(block_time as i64, 0).unwrap();
    d.format("%Y-%m-%d %H:%M:%S").to_string()
}
  
pub fn env_is_set(var_name: &str) -> bool {
    match std::env::var(var_name) {
        Ok(s) => s == "yes",
        _ => false
    }
}