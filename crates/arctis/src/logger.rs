use chrono::Utc;
use flexi_logger::{DeferredNow, Duplicate, LogSpecBuilder, Logger};
use log::{info, LevelFilter, Record};
use std::io::{Result, Write};

pub fn init_logger() -> Result<()> {
    // Step 1: Configure log channels (WS, RPC, target) and their filtering rules
    let log_spec = LogSpecBuilder::new()
        .default(LevelFilter::Info) // Default log level
        .module("SERVER", LevelFilter::Info) // Enable logging for Server module
        .module("HTTP", LevelFilter::Info) // Enable logging for HTTP module
        .module("WS", LevelFilter::Info) // Enable logging for WS module
        .module("RPC", LevelFilter::Info) // Enable logging for RPC module
        .module("PERF", LevelFilter::Info) 
        .module("target", LevelFilter::Info)
        .module("client", LevelFilter::Info)
        .build();

    // Step 2: Initialize the logger
    Logger::with(log_spec)
        .format(custom_log_format) // Use custom log format
        .log_to_file(flexi_logger::FileSpec::default().directory("logs"))
        .use_utc()
        .duplicate_to_stderr(Duplicate::All) // Also print logs to stdout
        .start()
        .unwrap();

    eprintln!("Logger initialized.");
    Ok(())
}

// Custom log format
pub fn custom_log_format(
    writer: &mut dyn Write, // A dynamic writer (stdout, file, etc.)
    _now: &mut DeferredNow,  // DeferredNow to handle time formatting
    record: &Record,        // The actual log record
) -> Result<()> {
    // Format the time using chrono - UTC required
    let time = Utc::now().format("%Y-%m-%d %H:%M:%S:%3f").to_string();

    writeln!(
        writer,
        "{} [{}] {}", // E.g., 12:37:24:873 [Server] Listening on http://localhost
        time,
        record.target(), // Single channel, e.g., "Server"
        &record.args()   // The actual log message
    )
}

// This function accepts a flexible number of channels (up to 3)
pub fn log_message(channels: &[&str], message: &str) {
    let formatted_channels = channels.join("][");
    info!("[{}] {}", formatted_channels, message);
}
