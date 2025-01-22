use std::str::FromStr;

use anyhow::Result;
use solana_sdk::instruction::CompiledInstruction;
use solana_sdk::message::AccountKeys;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::parse_instruction::ParsedInstruction;
use solana_transaction_status::UiCompiledInstruction;

pub struct InstructionWrapper<'a> {
    pub ix: &'a UiCompiledInstruction,
    pub ix_idx: usize,
    pub pix_idx: u8,
}

impl<'a> InstructionWrapper<'a> {
    pub fn new(ix: &'a UiCompiledInstruction, ix_idx: usize, pix_idx: u8) -> Self {
        Self {
            ix,
            ix_idx,
            pix_idx,
        }
    }
}

pub fn parse_compiled_instruction(
    compiled_instruction: &CompiledInstruction,
    accounts: &[String],
    stack_height: Option<u32>,
) -> Result<ParsedInstruction> {
    let program_id = accounts[compiled_instruction.program_id_index as usize].clone();
    let program_id = Pubkey::from_str(&program_id).unwrap();

    let account_keys: Vec<Pubkey> = accounts
        .iter()
        .map(|key| Pubkey::from_str(key).unwrap())
        .collect();
    let account_keys = AccountKeys::new(&account_keys, None);

    let parsed = solana_transaction_status::parse_instruction::parse(
        &program_id,
        compiled_instruction,
        &account_keys,
        stack_height,
    )?;

    Ok(parsed)
}

pub fn parse_ui_instruction(
    ui_compiled_instruction: &UiCompiledInstruction,
    accounts: &[String],
) -> Result<ParsedInstruction> {
    let compiled_instruction = CompiledInstruction {
        program_id_index: ui_compiled_instruction.program_id_index,
        accounts: ui_compiled_instruction.accounts.clone(),
        data: solana_sdk::bs58::decode(&ui_compiled_instruction.data)
            .into_vec()
            .unwrap(),
    };
    parse_compiled_instruction(
        &compiled_instruction,
        accounts,
        ui_compiled_instruction.stack_height,
    )
}
