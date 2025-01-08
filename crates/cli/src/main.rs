use arctis::{config::get_settings, run::{parse_block, parse_transaction, ExecutionContext}};
use clap::{Parser, Subcommand};
use anyhow::{Result};

#[derive(Parser)]
#[command(author, version, about = "AlphaArc Arctis CLI", long_about = None)]
struct Cli {
    /// A command to run
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Parse blocks, transactions, or programs
  Parse {
    /// Parse a specific block
    #[command(subcommand)]
    subcommand: Parse,
  },
  /*

  /// Fetch information about a token
  Token {
    /// Token address
    address: String,
  },
  /// Monitor blocks or programs
  Monitor {
    /// Monitor blocks
    #[arg(default_value = "BlocksWS")]
    strategy: String,
  },
   */
}

#[derive(Subcommand)]
enum Parse {
  /// Parse a specific block
  Block {
    /// Dataset to print
    #[arg(long, value_name = "DATASET", default_value = "swaps")]
    dataset: String,

    /// Filter to apply
    #[arg(long, value_name = "FILTER", default_value = "pumpfun")]
    filter: String,

    /// Block number to parse
    block_number: u64,
  },
  /*
  /// Parse a range of blocks
  Blocks {
    /// Range of blocks to parse, in the format start:end
    block_range: String,
  },
   */
  /// Parse a specific transaction
  Tx {
    /// Transaction ID to parse
    tx_id: String,
  },
}

/*
fn parse_block_range(range: &str) -> Result<(u64, u64)> {
    let (start, end) = range
        .split_once(':')
        .ok_or_else(|| anyhow!("Invalid block range format. Expected start:end"))?;
    let start = start.parse().context("Failed to parse start of range")?;
    let end = end.parse().context("Failed to parse end of range")?;
    Ok((start, end))
}
*/

fn print_banner() {
  println!("\n");
  println!("#############################################");
  println!("########     AlphaArc Arctis CLI     ########");
  println!("#############################################");
  println!("\n\n");
}

async fn handle_parse_block(block_number: u64, ctx: &ExecutionContext) -> Result<()> {
  println!("Parse block: {}", block_number);
  let sol_db = parse_block(block_number, ctx).await?;
  sol_db.print_table("swaps")?;
  Ok(())
}

/*
async fn handle_parse_blocks(block_range: &str, _ctx: &ExecutionContext) -> Result<()> {
  let (start, end) = parse_block_range(block_range)?;
  println!("Parse blocks: {} to {}", start, end);
  Ok(())
}
*/

async fn handle_parse_transaction(tx_id: &str, ctx: &ExecutionContext) -> Result<()> {
  println!("Parse Transaction: {}", tx_id);
  let result = parse_transaction(tx_id, ctx).await?;
  let result_pretty = serde_json::to_string_pretty(&result)?;
  println!("Transaction: {}", result_pretty);
  Ok(())
}

/*
async fn handle_token(address: &str) -> Result<()> {
  println!("Token: {}", address);
  Ok(())
}

async fn handle_monitor(strategy: &str, ctx: &ExecutionContext) -> Result<()> {
  println!("Monitoring blocks with strategy: {}", strategy);
  monitor_blocks(&ctx).await?;
  Ok(())
}
*/

#[tokio::main]
async fn main() -> Result<()> {
    print_banner();

    let settings = get_settings()?;
    let ctx = ExecutionContext {
      rpc_url: settings.rpc.solana_rpc_url,
      ws_url: settings.rpc.solana_ws_url,
    };

    let cli = Cli::parse();

    match cli.command {
      Commands::Parse { subcommand } => match subcommand {
        Parse::Block { block_number, dataset: _, filter: _ } => handle_parse_block(block_number, &ctx).await?,
        // Parse::Blocks { block_range } => handle_parse_blocks(&block_range, &ctx).await?,
        Parse::Tx { tx_id } => handle_parse_transaction(&tx_id, &ctx).await?,
      },
      // Commands::Token { address } => handle_token(&address).await?,
      // Commands::Monitor { strategy } => handle_monitor(&strategy, &ctx).await?,
    };

    Ok(())
}
