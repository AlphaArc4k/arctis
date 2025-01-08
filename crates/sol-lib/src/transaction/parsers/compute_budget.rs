use arctis_types::{BlockInfo, ComputeBudgetInstruction, ParserResult, ParserResultData};
use solana_transaction_status::UiCompiledInstruction;
use crate::transaction::{wrapper::TransactionWrapper, InstructionWrapper};
use super::Parser;
use anyhow::Result;
use solana_sdk::bs58::decode;

pub struct ComputeBudgetProgramParser;

impl Parser for ComputeBudgetProgramParser {
  fn parse(&self, ix: &InstructionWrapper, _tx: &TransactionWrapper, _block: &BlockInfo) -> Result<ParserResult> {
    let parsed = parse_compute_budget_instruction(&ix.ix)?;
    Ok(ParserResult {
      parsed: true,
      ix_type: match parsed {
        ComputeBudgetInstruction::SetComputeUnitLimit(_) => "SetComputeUnitLimit".to_string(),
        ComputeBudgetInstruction::SetComputeUnitPrice(_) => "SetComputeUnitPrice".to_string(),
        ComputeBudgetInstruction::RequestHeapFrame => "RequestHeapFrame".to_string(),
        ComputeBudgetInstruction::Unknown => "Unknown".to_string(),
      },
      data: ParserResultData::ComputeBudget(parsed),
    })
  }
}


pub fn parse_compute_budget_instruction(instruction: &UiCompiledInstruction) -> Result<ComputeBudgetInstruction> {
  let inst_data = instruction.data.clone();
  let data_buf = decode(inst_data).into_vec().unwrap();

  // u8 discriminator: 2 = SetComputeUnitLimit | 3 = SetComputeUnitPrice
  let d=  data_buf.get(0);
  if d == Some(&1) {
    return Ok(ComputeBudgetInstruction::RequestHeapFrame);
  }
  if d == Some(&2) {
    let limit_bytes = &data_buf[1..5];
    let limit = u32::from_le_bytes(limit_bytes.try_into().unwrap());
    return Ok(ComputeBudgetInstruction::SetComputeUnitLimit(limit));
  }
  else if d == Some(&3) {
    let fee_bytes = &data_buf[1..9];
    let fee = u64::from_le_bytes(fee_bytes.try_into().unwrap());
    let fee_lamports = fee as f64 / 1_000_000.0;
    return Ok(ComputeBudgetInstruction::SetComputeUnitPrice(fee_lamports));
  } else {
    return Ok(ComputeBudgetInstruction::Unknown);
  }
}
