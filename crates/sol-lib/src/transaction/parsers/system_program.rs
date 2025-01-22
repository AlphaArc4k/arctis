use super::base::Parser;
use crate::transaction::wrapper::TransactionWrapper;
use crate::transaction::{parse_ui_instruction, InstructionWrapper};
use anyhow::{anyhow, Result};
use arctis_types::{AccountInfo, BlockInfo, ParserResult, ParserResultData, SolTransfer};
use solana_sdk::native_token::lamports_to_sol;

pub struct SystemProgramParser;

impl Parser for SystemProgramParser {
    fn parse(
        &self,
        ix: &InstructionWrapper,
        tx: &TransactionWrapper,
        block: &BlockInfo,
    ) -> Result<ParserResult> {
        let BlockInfo { slot, block_time } = block;

        let accounts = tx.get_accounts();
        let ix_parsed = parse_ui_instruction(&ix.ix, &accounts).unwrap();

        if ix_parsed.program_id != "11111111111111111111111111111111" {
            return Err(anyhow!("Invalid program id: {}", ix_parsed.program_id));
        }

        let signature = tx.get_signature();

        let ix_type = ix_parsed.parsed["type"].as_str().unwrap();
        match ix_type {
            "transfer" => {
                let sol_transfer = SolTransfer {
                    slot: *slot,
                    block_time: *block_time,
                    signature: signature,
                    from: ix_parsed.parsed["info"]["source"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    to: ix_parsed.parsed["info"]["destination"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    lamports: ix_parsed.parsed["info"]["lamports"].as_u64().unwrap(),
                    sol: lamports_to_sol(ix_parsed.parsed["info"]["lamports"].as_u64().unwrap()),
                };
                return Ok(ParserResult {
                    parsed: true,
                    ix_type: "transfer".to_string(),
                    data: ParserResultData::SolTransfer(sol_transfer),
                });
            }
            "createAccountWithSeed" => {
                let parsed = &ix_parsed.parsed["info"];
                let new_account = &parsed["newAccount"].as_str().unwrap();
                let owner = &parsed["owner"].as_str().unwrap();
                let _base = &parsed["base"].as_str().unwrap();
                let _seed = &parsed["seed"].as_str().unwrap();
                let _source = &parsed["source"].as_str().unwrap();
                let _lamports = parsed["lamports"].as_u64().unwrap();
                let _space = parsed["space"].as_u64().unwrap();

                let account_info = AccountInfo {
                    account: new_account.to_string(),
                    owner: owner.to_string(),
                    open_tx: Some(signature),
                    init_tx: None,
                    close_tx: None,
                    close_destination: None,
                    mint: None,
                    decimals: None,
                };

                return Ok(ParserResult {
                    parsed: true,
                    ix_type: "createAccountWithSeed".to_string(),
                    data: ParserResultData::Account(account_info),
                });
            }
            "createAccount" => {
                return Ok(ParserResult {
                    parsed: false,
                    ix_type: "createAccount".to_string(),
                    data: ParserResultData::NoData,
                });
            }
            "initializeNonce" => {
                return Ok(ParserResult {
                    parsed: false,
                    ix_type: "initializeNonce".to_string(),
                    data: ParserResultData::NoData,
                });
            }
            "advanceNonce" => {
                return Ok(ParserResult {
                    parsed: false,
                    ix_type: "advanceNonce".to_string(),
                    data: ParserResultData::NoData,
                });
            }
            "withdrawFromNonce" => {
                return Ok(ParserResult {
                    parsed: false,
                    ix_type: "withdrawFromNonce".to_string(),
                    data: ParserResultData::NoData,
                });
            }
            "transferWithSeed" => {
                return Ok(ParserResult {
                    parsed: false,
                    ix_type: "transferWithSeed".to_string(),
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
