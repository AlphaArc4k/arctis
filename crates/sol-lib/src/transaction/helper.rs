use std::collections::HashMap;

use serde::Serialize;
use solana_transaction_status::{EncodedTransactionWithStatusMeta, UiCompiledInstruction, UiInstruction, UiRawMessage, UiTransaction, UiTransactionStatusMeta};
use anyhow::{Result, anyhow};


pub fn get_transaction_data(transaction : &EncodedTransactionWithStatusMeta) -> &UiTransaction {
  match &transaction.transaction {
    solana_transaction_status::EncodedTransaction::Json(ui_transaction) => {
      ui_transaction
    }
    _ => panic!("Transaction not json encoded"),
  }
}

pub fn get_transaction_message(transaction : &EncodedTransactionWithStatusMeta) -> &UiRawMessage {
  let tx = get_transaction_data(transaction);
  match &tx.message {
    solana_transaction_status::UiMessage::Raw(message) => {
      message
    }
    _ => panic!("Transaction message not raw - parsed?"),
  }
}

pub fn get_transaction_meta(transaction : &EncodedTransactionWithStatusMeta) -> &UiTransactionStatusMeta {
  match &transaction.meta {
    Some(meta) => {
      meta
    }
    _ => panic!("Transaction meta not found"),
  }
}

pub fn has_error(transaction : &EncodedTransactionWithStatusMeta) -> bool {
  transaction.meta.as_ref().map_or(false, |meta| meta.status.is_err())
}

pub fn get_transaction_signature(transaction : &EncodedTransactionWithStatusMeta) -> String {
  let tx = get_transaction_data(transaction);
  tx.signatures[0].clone()
}

pub fn get_transaction_signatures(transaction : &EncodedTransactionWithStatusMeta) -> Vec<String> {
  let tx = get_transaction_data(transaction);
  tx.signatures.clone()
}

pub fn get_accounts(message : &UiRawMessage, meta: &UiTransactionStatusMeta) -> Vec<String> {
  // append meta.loadedAddresses to account_keys : https://solana.stackexchange.com/a/12073
  let mut account_keys: Vec<String> = message.account_keys.clone();
  let loaded_addresses = meta.loaded_addresses.as_ref().unwrap();
  account_keys.extend(loaded_addresses.writable.iter().cloned());
  account_keys.extend(loaded_addresses.readonly.iter().cloned());

  account_keys
}

// TODO we can give this function a hint: e.g. "buy" or "sell" to change order where to search for -> perf improvement
pub fn get_token_decimals(transaction : &EncodedTransactionWithStatusMeta, mint: &str) -> Result<u8> {
  let pre_token_balances = transaction.meta.as_ref().unwrap().pre_token_balances.clone().unwrap();
  for token_balance in pre_token_balances {
    if token_balance.mint == mint {
      let decimals = token_balance.ui_token_amount.decimals;
      return Ok(decimals);
    }
  }
  let post_token_balances = transaction.meta.as_ref().unwrap().post_token_balances.clone().unwrap();
  for token_balance in post_token_balances {
    if token_balance.mint == mint {
      let decimals = token_balance.ui_token_amount.decimals;
      return Ok(decimals);
    }
  }
  return Err(anyhow!("Token decimals not found"));
}

#[derive(Serialize, Debug)]
pub struct TokenAccountInfo {
  pub address: String,
  pub mint: String,
  pub amount_pre: f64,
  pub amount_post: f64,
  pub owner: Option<String>,
  pub decimals: u8,
  pub is_closed: bool,
}

/**
 * Combines pre and post token balances into a lookup table
 */
pub fn get_token_account_lookup(tx: &EncodedTransactionWithStatusMeta, accounts: &Vec<String>, _include_closed: bool) -> HashMap<String, TokenAccountInfo> {

  let meta = get_transaction_meta(tx);

  let pre_token_balances = meta.pre_token_balances.as_ref().map(|balances| &**balances);
  let post_token_balances = meta.post_token_balances.as_ref().map(|balances| &**balances);
  let mut lookup: HashMap<String, TokenAccountInfo> = HashMap::new();

  if pre_token_balances.is_some() {
    for balance in pre_token_balances.unwrap() {
      let account_index = balance.account_index as usize;
      let address = accounts[account_index].clone();
      let mint = balance.mint.clone();
      let amount = balance.ui_token_amount.ui_amount_string.parse::<f64>().unwrap();
      let decimals = balance.ui_token_amount.decimals;
      let owner = balance.owner.as_ref().map(|s| s.to_string());
      if lookup.contains_key(&address) {
        lookup.entry(address).and_modify(|e| {
          e.amount_pre = amount;
          e.mint = mint;
          e.decimals = decimals;
        });
      }
      else {
        lookup.insert(address.clone(), TokenAccountInfo {
          address: address,
          mint: mint,
          decimals: decimals,
          amount_pre: amount,
          amount_post: 0.0,
          owner: owner,
          is_closed: false,
        });
      }
    }
  }
  if post_token_balances.is_some() {
    for balance in post_token_balances.unwrap() {
      let account_index = balance.account_index as usize;
      let address = accounts[account_index].clone();
      let mint = balance.mint.clone();
      let amount = balance.ui_token_amount.ui_amount_string.parse::<f64>().unwrap();
      let decimals = balance.ui_token_amount.decimals;
      let owner = balance.owner.as_ref().map(|s| s.to_string());
      if lookup.contains_key(&address) {
        lookup.entry(address).and_modify(|e| {
          e.amount_post = amount;
          e.mint = mint;
          e.decimals = decimals;
        });
      }
      else {
        lookup.insert(address.clone(), TokenAccountInfo {
          address: address,
          mint: mint,
          decimals: decimals,
          amount_pre: 0.0,
          amount_post: amount,
          owner: owner,
          is_closed: false,
        });
      }
    }
  }
  return lookup;
}

#[derive(Serialize, Debug, Clone)]
pub struct ExtendedCompiledInstruction {
  pub instruction_index: u8,
  pub program_id_index: u8,
  pub program_id: String,
  pub accounts: Vec<u8>,
  pub data: String,
  pub stack_height: Option<u32>,
  pub inner_instructions: Vec<UiCompiledInstruction>,
}

fn get_transaction_instructions_with_inner (transaction : &EncodedTransactionWithStatusMeta, program_id_filter: Option<&str>) -> Vec<ExtendedCompiledInstruction> {

  let message = get_transaction_message(&transaction);
  let meta = get_transaction_meta(&transaction);
  let accounts = get_accounts(message, meta);

  let inner_instructions = &meta.inner_instructions;

  let mut instructions = vec![];
  let mut i = 0;
  for instruction in &message.instructions {
    let program_id = accounts[instruction.program_id_index as usize].clone();
    // if a program id filter is provided, skip instructions that don't match
    if program_id_filter.is_some() && program_id != program_id_filter.clone().unwrap() {
      i += 1;
      continue;
    }

    // TODO consider moving this in trait get_inner() for performance improvement
    let def = vec![];
    let inner_instruction = inner_instructions.as_ref().unwrap_or(&def).iter().find(|inner_instruction| inner_instruction.index == i);
    let inner_instructions = inner_instruction.map_or(vec![], |inner| inner.instructions.clone());

    let mut inner_compiled_instructions: Vec<UiCompiledInstruction> = vec![];
    for inner_instruction in inner_instructions {
      let inner_compiled_instruction = match inner_instruction {
        UiInstruction::Parsed(_instruction) => {
          panic!("Parsed instruction can not be parsed");
        }
        UiInstruction::Compiled(instruction) => {
          instruction.clone()
        }
      };
      inner_compiled_instructions.push(inner_compiled_instruction);
    }

    let inst_ex = ExtendedCompiledInstruction {
      instruction_index: i,
      program_id_index: instruction.program_id_index,
      program_id: program_id,
      accounts: instruction.accounts.clone(),
      data: instruction.data.clone(),
      stack_height: instruction.stack_height,
      inner_instructions: inner_compiled_instructions,
    };
    instructions.push(inst_ex);
    i += 1;
  }
  instructions
}


pub fn get_inner_instructions(transaction : &EncodedTransactionWithStatusMeta, program_id: &str) -> Result<Vec<UiCompiledInstruction>> {
  let top_level_instructions = get_transaction_instructions_with_inner(transaction, Some(program_id));
  if top_level_instructions.len() != 1 {
    if top_level_instructions.len() == 0 {
      return Err(anyhow!("No top level instruction found"));
    }
    return Err(anyhow!("Too many top-level instructions: expected 1"));
  }
  let top_instruction = &top_level_instructions[0];
  let instruction_inner = top_instruction.inner_instructions.clone();
  return Ok(instruction_inner);
}