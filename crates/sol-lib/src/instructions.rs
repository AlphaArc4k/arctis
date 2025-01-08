use std::str::FromStr;

use solana_sdk::{instruction::CompiledInstruction, message::AccountKeys, pubkey::Pubkey};
use solana_transaction_status::{parse_instruction::ParsedInstruction, UiCompiledInstruction};
use anyhow::Result;


pub fn parse_compiled_instruction(compiled_instruction: &CompiledInstruction, accounts: &Vec<String>, stack_height: Option<u32>) -> Result<ParsedInstruction> {
  let program_id = accounts[compiled_instruction.program_id_index as usize].clone();
  let program_id = Pubkey::from_str(&program_id).unwrap();

  let account_keys: Vec<Pubkey> = accounts.iter()
    .map(|key| Pubkey::from_str(&key).unwrap())
    .collect();
  let account_keys = AccountKeys::new(&account_keys, None);

  let parsed = solana_transaction_status::parse_instruction::parse(&program_id, &compiled_instruction, &account_keys, stack_height)?;

  Ok(parsed)
}

pub fn parse_ui_instruction(ui_compiled_instruction: &UiCompiledInstruction, accounts: &Vec<String>) -> Result<ParsedInstruction> {
  let compiled_instruction = CompiledInstruction {
    program_id_index: ui_compiled_instruction.program_id_index,
    accounts: ui_compiled_instruction.accounts.clone(),
    data: solana_sdk::bs58::decode(&ui_compiled_instruction.data).into_vec().unwrap(),
  };
  return parse_compiled_instruction(&compiled_instruction, accounts, ui_compiled_instruction.stack_height)
}
