use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

pub use solana_transaction_status_client_types::EncodedConfirmedTransactionWithStatusMeta;
pub use solana_transaction_status_client_types::EncodedTransactionWithStatusMeta;
pub use solana_transaction_status_client_types::UiConfirmedBlock;

// Define an enum for the type of swap
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SwapType {
  Sell,
  Buy,
  Token,
  // Unknown
}

// map to database enum compatible strings
impl SwapType {
  pub fn from_db(s: &str) -> Result<SwapType> {
    match s {
      "Sell" => Ok(SwapType::Sell),
      "Buy" => Ok(SwapType::Buy),
      "Token" => Ok(SwapType::Token),
      _ => Err(anyhow!("Invalid swap type: {}", s))
    }
  }
  pub fn to_db(&self) -> &str {
    match self {
      SwapType::Sell => "Sell",
      SwapType::Buy => "Buy",
      SwapType::Token => "Token",
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DexType {
  Jupiterv6,
  Pumpfun,
  RaydiumAmm,
  Unknown
}

// map to database enum compatible strings
impl DexType {
  pub fn from_db(s: &str) -> Result<DexType> {
    match s {
      "Jupiterv6" => Ok(DexType::Jupiterv6),
      "Pumpfun" => Ok(DexType::Pumpfun),
      "RaydiumAmm" => Ok(DexType::RaydiumAmm),
      "Unknown" => Ok(DexType::Unknown),
      _ => Err(anyhow!("Invalid dex type: {}", s))
    }
  }
  pub fn to_db(&self) -> &str {
    match self {
      DexType::Jupiterv6 => "Jupiterv6",
      DexType::Pumpfun => "Pumpfun",
      DexType::RaydiumAmm => "RaydiumAmm",
      DexType::Unknown => "Unknown",
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwapInfo {
  pub slot: u64,
  pub block_time: i64,
  pub signer: String,
  pub signature: String,
  pub error: bool, // not used atm: only compat
  pub dex: DexType,
  // TODO beneficiary
  // TODO slippage
  pub swap_type: SwapType,
  pub amount_in: f64,
  pub token_in: String,
  pub amount_out: f64,
  pub token_out: String,
}


#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct NewToken {
  pub block_time: i64,
  pub slot: u64,
  pub signature: String,
  pub signer: String,
  pub factory: String, // factory program id
  pub mint: String,
  pub decimals: u8,
  pub name: String,
  pub symbol: String,
  pub uri: String,
  pub initial_supply: Option<u64>,
  pub supply: Option<u64>,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct SolTransfer {
  pub slot: u64,
  pub block_time: i64,
  pub signature: String,
  pub from: String,
  pub to: String,
  pub lamports: u64,
  
  // derived
  pub sol: f64,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct SplTokenTransfer {
  pub slot: u64,
  pub block_time: i64,
  pub signature: String,

  pub from_acc: String,
  pub to_acc: String,
  pub amount: f64,
  pub authority: Option<String>,

  pub from: Option<String>,
  pub to: Option<String>,
  pub decimals: Option<u8>,
  pub token: Option<String>,
}


#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct AccountInfo {
  pub account: String,
  pub owner: String,
  pub open_tx: Option<String>,
  pub init_tx: Option<String>,
  pub close_tx: Option<String>,
  pub close_destination: Option<String>,
  pub mint: Option<String>,
  pub decimals: Option<u8>,
} 

#[derive(Serialize, Debug, PartialEq, Clone)]
pub enum SupplyChangeType {
  Mint,
  Burn,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct SupplyChange {
  pub signature: String,
  pub ix_index: usize,
  pub account: String,
  pub mint: String,
  pub authority: String,
  // amount could be negative but we might have issues with resolution. 
  // therefore we might want to use u64 + SupplyChangeType
  pub amount: i128,
  // pub change_type: SupplyChangeType,
}


#[derive(Serialize, Debug, PartialEq, Clone)]
pub enum ComputeBudgetInstruction {
  SetComputeUnitLimit(u32),
  SetComputeUnitPrice(f64),
  RequestHeapFrame,
  Unknown
}

pub struct BlockInfo {
  pub slot: u64,
  pub block_time: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct ParserResult {
  pub parsed: bool,
  pub ix_type: String,
  pub data: ParserResultData,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub enum ParserResultData {
  ComputeBudget(ComputeBudgetInstruction),
  SolTransfer(SolTransfer),
  TokenTransfer(SplTokenTransfer),
  Swap(SwapInfo),
  Token(NewToken),
  Account(AccountInfo),
  Supply(SupplyChange),
  NoData,
  NoOp
}


