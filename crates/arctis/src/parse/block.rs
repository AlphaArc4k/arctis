use std::{collections::HashMap, time::Instant};

use arctis_types::{ComputeBudgetInstruction, ParserResult, ParserResultData, UiConfirmedBlock};
use sol_db::solana_db::{ComputeBudgetProcessed, ProcessedBlock, ProcessedTransaction, ProgramParserData, SolanaDatabase};
use anyhow::{Result, anyhow};

use super::transaction::process_transaction;

pub fn process_block(block: &UiConfirmedBlock, solana_db: &mut SolanaDatabase) -> Result<()> {
  let transactions = block.transactions.as_ref().unwrap();
  let tx_count = transactions.len();

  let slot = block.parent_slot + 1;
  let block_time = block.block_time.unwrap();

  let p_block = ProcessedBlock {
    slot,
    block_time,
    parent_slot: block.parent_slot,
    transaction_count: tx_count as u32,
  };

  let res = solana_db.insert_block(&p_block);
  if res.is_err() {
    return Err(anyhow!("Failed to insert block"));
  }

  let ts_start_process_tx = Instant::now();
  let mut processed_tx = vec![];
  for tx in transactions {
    let ptx = process_transaction(&tx, slot, block_time); 
    match ptx {
      Ok(ptx) => processed_tx.push(ptx),
      Err(_err) => {
        // all or nothing: if we don't fail fast missing tx will go unnoticed for too long in pipeline
        return Err(anyhow!("Failed to process tx"));
      }
    } 
  } // end tx loop
  let _elapsed = ts_start_process_tx.elapsed();

  write_transactions_with_instructions_db(solana_db, slot, block_time, processed_tx)?;

  Ok(())
}


fn write_transactions_with_instructions_db(
  solana_db: &mut SolanaDatabase,
  slot: u64,
  block_time: i64,
  processed_tx: Vec<ProcessedTransaction>
) -> Result<()> {

  let ts_start = Instant::now();
  let res = solana_db.insert_transactions_bulk(&processed_tx);
  if res.is_err() {
    println!("Failed to insert transactions: {:?}", res);
    return Err(anyhow!("Failed to insert transactions"));
  }
  let _elapsed = ts_start.elapsed();
  // println!("Wrote transactions in {:?}", elapsed);

  // handle parsed programs
  let all_parsed_programs: Vec<&ProgramParserData> = processed_tx
    .iter()
    .flat_map(|tx| tx.parsed_programs.iter())
    .collect();

  let res = solana_db.insert_parsed_programs_bulk(&all_parsed_programs);
  if res.is_err() {
    println!("Failed to insert parsed programs: {:?}", res);
    return Err(anyhow!("Failed to insert parsed programs"));
  }

  // handle parsed program instructions
  let all_parsed_program_ix: Vec<(String, &ParserResult)> = processed_tx
    .iter()
    .flat_map(|tx| tx.parsed_ix.iter().map(move |ix| (tx.signature.clone(), ix)) )
    .collect();

  // collect tables
  let mut sol_transfers = vec![];
  let mut token_transfers = vec![];
  let mut swaps = vec![];
  let mut tokens = vec![];
  let mut supply_changes = vec![];

  let mut fees: HashMap<String, ComputeBudgetProcessed> = HashMap::new();

  for (signature, ppd) in all_parsed_program_ix {
    let data = &ppd.data;
    let _ix_type = &ppd.ix_type;
    match &data {
      ParserResultData::SolTransfer(transfer) => {
        sol_transfers.push(transfer);
      },
      ParserResultData::TokenTransfer(transfer) => {
        token_transfers.push(transfer);
      },
      ParserResultData::Swap(swap) => {
        swaps.push(swap);
      },
      ParserResultData::Token(token) => {
        tokens.push(token);
      },
      ParserResultData::Supply(supply_change) => {
        supply_changes.push(supply_change);
      },
      // TODO collect in hashmap
      ParserResultData::ComputeBudget(budget) => {
        match budget {
          // we don't care?
          ComputeBudgetInstruction::RequestHeapFrame => {},
          ComputeBudgetInstruction::SetComputeUnitLimit(c_unit_limit) => {
            // insert or update
            let entry = fees.entry(signature.clone()).or_insert(ComputeBudgetProcessed {
              slot,
              block_time,
              signature: signature.clone(),
              c_unit_limit: 0,
              fee: 0,
            });
            entry.c_unit_limit = *c_unit_limit as u64;
          },
          ComputeBudgetInstruction::SetComputeUnitPrice(fee) => {
            // insert or update
            let entry = fees.entry(signature.clone()).or_insert(ComputeBudgetProcessed {
              slot,
              block_time,
              signature: signature.clone(),
              c_unit_limit: 0,
              fee: 0,
            });
            entry.fee = *fee as u64;
          },
          _ => {
            // println!("Unknown compute budget in {:?}", signature);
          }
        }
      },
      _ => {
        // ignore
      }
    }
  }

  // insert sol transfers bulk
  let res = solana_db.insert_sol_transfer_bulk(&sol_transfers);
  if res.is_err() {
    return Err(anyhow!("Failed to insert sol transfers"));
  }

  // insert token transfers bulk
  let res = solana_db.insert_token_transfers_bulk(&token_transfers);
  if res.is_err() {
    return Err(anyhow!("Failed to insert token transfers"));
  }

  // insert swaps bulk
  let res = solana_db.insert_swaps_bulk(&swaps);
  if res.is_err() {
    return Err(anyhow!("Failed to insert swaps"));
  }

  // insert tokens bulk
  let res = solana_db.insert_tokens_bulk(&tokens);
  if res.is_err() {
    return Err(anyhow!("Failed to insert tokens"));
  }

  // insert supply changes bulk
  let res = solana_db.insert_supply_changes_bulk(&supply_changes);
  if res.is_err() {
    return Err(anyhow!("Failed to insert supply changes"));
  }

  // insert fees
  let fees: Vec<ComputeBudgetProcessed> = fees.into_iter().map(|(_, v)| v).collect();
  let res = solana_db.insert_compute_budget_bulk(&fees);
  if res.is_err() {
    return Err(anyhow!("Failed to insert fees"));
  }

  Ok(())
}