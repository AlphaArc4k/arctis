use super::Parser;
use crate::transaction::wrapper::TransactionWrapper;
use crate::transaction::{parse_ui_instruction, InstructionWrapper};
use anyhow::{anyhow, Result};
use arctis_types::{AccountInfo, BlockInfo, ParserResult, ParserResultData};

pub struct AssociatedTokenAccountProgramParser;

// https://github.com/solana-labs/solana-program-library/blob/master/associated-token-account/program/src/instruction.rs
impl Parser for AssociatedTokenAccountProgramParser {
    fn parse(
        &self,
        ix: &InstructionWrapper,
        tx: &TransactionWrapper,
        _block: &BlockInfo,
    ) -> Result<ParserResult> {
        // let BlockInfo{ slot, block_time } = block;
        let ix = &ix.ix;

        let accounts = tx.get_accounts();
        let ix_parsed = parse_ui_instruction(&ix, &accounts).unwrap();

        if ix_parsed.program_id != "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" {
            return Err(anyhow!("Invalid program id: {}", ix_parsed.program_id));
        }

        let ix_type = ix_parsed.parsed["type"].as_str().unwrap();
        match ix_type {
            "create" => {
                let parsed = &ix_parsed.parsed["info"];
                // println!("{:?}", parsed);
                let signature = tx.get_signature();
                let account_info = parse_create(parsed, signature);
                return Ok(ParserResult {
                    parsed: true,
                    ix_type: "createIdempotent".to_string(),
                    data: ParserResultData::Account(account_info),
                });
            }
            "createIdempotent" => {
                let parsed = &ix_parsed.parsed["info"];
                // println!("{:?}", parsed);
                let signature = tx.get_signature();
                let account_info = parse_create(parsed, signature);
                return Ok(ParserResult {
                    parsed: true,
                    ix_type: "createIdempotent".to_string(),
                    data: ParserResultData::Account(account_info),
                });
            }
            "recoverNested" => {
                return Ok(ParserResult {
                    parsed: false,
                    ix_type: "recoverNested".to_string(),
                    data: ParserResultData::NoData,
                });
            }
            _ => {
                return Ok(ParserResult {
                    parsed: false,
                    ix_type: ix_type.to_string(),
                    data: ParserResultData::NoData,
                });
            }
        }
    }
}

fn parse_create(parsed: &serde_json::Value, sig: String) -> AccountInfo {
    let account = parsed["account"].as_str().unwrap();
    let mint = parsed["mint"].as_str().unwrap();
    let wallet = parsed["wallet"].as_str().unwrap();
    // TODO do we need these values?
    let _source = parsed["source"].as_str().unwrap();
    let _token_program = parsed["tokenProgram"].as_str().unwrap();
    let _system_program = parsed["systemProgram"].as_str().unwrap();

    AccountInfo {
        account: account.to_string(),
        owner: wallet.to_string(),
        open_tx: Some(sig.to_string()),
        init_tx: Some(sig),
        close_tx: None,
        close_destination: None,
        mint: Some(mint.to_string()),
        decimals: None,
    }
}
