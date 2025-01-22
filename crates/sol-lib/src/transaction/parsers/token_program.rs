use crate::transaction::wrapper::TransactionWrapper;
use crate::transaction::{parse_ui_instruction, InstructionWrapper};
use anyhow::{Ok, Result};
use arctis_types::{
    AccountInfo, BlockInfo, ParserResult, ParserResultData, SplTokenTransfer, SupplyChange,
};

use super::Parser;

pub struct TokenProgramParser;

// https://spl.solana.com/token
impl Parser for TokenProgramParser {
    fn parse(
        &self,
        ix: &InstructionWrapper,
        tx: &TransactionWrapper,
        block: &BlockInfo,
    ) -> Result<ParserResult> {
        let accounts = tx.get_accounts();

        let signature = tx.get_signature();

        let ix_parsed = parse_ui_instruction(ix.ix, &accounts).unwrap();

        let ix_type = ix_parsed.parsed["type"].as_str().unwrap();

        match ix_type {
            // https://github.com/solana-labs/solana-program-library/blob/master/token/program/src/processor.rs#L229
            "transfer" => {
                let parsed = &ix_parsed.parsed["info"];
                let spl_transfer = parse_transfer(parsed, tx, block, signature);
                Ok(ParserResult {
                    parsed: true,
                    ix_type: "transfer".to_string(),
                    data: ParserResultData::TokenTransfer(spl_transfer),
                })
            }
            "transferChecked" => {
                let parsed = &ix_parsed.parsed["info"];
                let spl_transfer = parse_transfer(parsed, tx, block, signature);
                Ok(ParserResult {
                    parsed: true,
                    ix_type: "transfer".to_string(),
                    data: ParserResultData::TokenTransfer(spl_transfer),
                })
            }
            "approve" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "approve".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            // https://github.com/solana-labs/solana-program-library/blob/master/token/program/src/instruction.rs#L214
            "closeAccount" => {
                let lookup = tx.get_account_lookup();

                if ix_parsed.parsed["info"]["owner"].as_str().is_none() {
                    return Err(anyhow::anyhow!(
                        "closeAccount: multisig account not supported"
                    ));
                }

                let mut account_info = AccountInfo {
                    account: ix_parsed.parsed["info"]["account"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    owner: ix_parsed.parsed["info"]["owner"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    open_tx: None,
                    init_tx: None,
                    close_tx: Some(signature),
                    close_destination: Some(
                        ix_parsed.parsed["info"]["destination"]
                            .as_str()
                            .unwrap()
                            .to_string(),
                    ),
                    mint: None,
                    decimals: None,
                };

                // find the account in the lookup
                let account = lookup.get(&account_info.account);
                if account.is_some() {
                    let account = account.unwrap();
                    account_info.mint = Some(account.mint.clone());
                    account_info.decimals = Some(account.decimals);
                }

                Ok(ParserResult {
                    parsed: true,
                    ix_type: "closeAccount".to_string(),
                    data: ParserResultData::Account(account_info),
                })
            }
            // https://github.com/solana-labs/solana-program-library/blob/master/token/program/src/instruction.rs#L65
            "initializeAccount" => {
                //TODO unused ix_parsed RentSysVar

                let lookup = tx.get_account_lookup();

                let mut account_info = AccountInfo {
                    account: ix_parsed.parsed["info"]["account"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    owner: ix_parsed.parsed["info"]["owner"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    open_tx: None,
                    init_tx: Some(signature),
                    close_tx: None,
                    close_destination: None,
                    mint: Some(
                        ix_parsed.parsed["info"]["mint"]
                            .as_str()
                            .unwrap()
                            .to_string(),
                    ),
                    decimals: None,
                };

                // find the account in the lookup
                let account = lookup.get(&account_info.account);
                if account.is_some() {
                    let account = account.unwrap();
                    account_info.decimals = Some(account.decimals);
                }

                Ok(ParserResult {
                    parsed: true,
                    ix_type: "initializeAccount".to_string(),
                    data: ParserResultData::Account(account_info),
                })
            }
            "initializeAccount2" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "initializeAccount2".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            "initializeAccount3" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "initializeAccount3".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            "initializeImmutableOwner" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "initializeImmutableOwner".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            "approveChecked" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "approveChecked".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            // https://github.com/solana-labs/solana-program-library/blob/master/token/program/src/instruction.rs#L378
            "syncNative" => {
                // this does not really impact calculations
                // it is often used by bonk when sol is moved in wrapped sol accounts to sync the balance change
                Ok(ParserResult {
                    parsed: true,
                    ix_type: "syncNative".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            "initializeMint" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "initializeMint".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            "mintTo" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "mintTo".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            "mintToChecked" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "mintToChecked".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            "burn" => {
                let signature = tx.get_signature();

                let parsed = &ix_parsed.parsed["info"];
                let account = parsed["account"].as_str().unwrap();
                let mint = parsed["mint"].as_str().unwrap();
                let authority = parsed["authority"].as_str().unwrap_or("");
                // FIXME might overflow
                let amount = parsed["amount"].as_str().unwrap().parse::<u64>().unwrap();

                let supply_change = SupplyChange {
                    signature,
                    ix_index: ix.ix_idx,
                    account: account.to_string(),
                    mint: mint.to_string(),
                    authority: authority.to_string(),
                    amount: -(amount as i128),
                    // change_type: SupplyChangeType::Burn,
                };

                Ok(ParserResult {
                    parsed: true,
                    ix_type: "burn".to_string(),
                    data: ParserResultData::Supply(supply_change),
                })
            }
            "burnChecked" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "burnChecked".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            "setAuthority" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "setAuthority".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            "revoke" => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: "revoke".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            _ => {
                Ok(ParserResult {
                    parsed: false,
                    ix_type: ix_type.to_string(),
                    data: ParserResultData::NoData,
                })
            }
        }
    }
}

fn parse_transfer(
    parsed: &serde_json::Value,
    tx: &TransactionWrapper,
    block_info: &BlockInfo,
    signature: String,
) -> SplTokenTransfer {
    let BlockInfo { slot, block_time } = block_info;

    let amount = match parsed["amount"].as_str() {
        // transfer:
        Some(a) => a.parse::<f64>().unwrap(),
        // transfer_checked:
        None => parsed["tokenAmount"]["amount"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap(),
    };

    let mut spl_transfer = SplTokenTransfer {
        slot: *slot,
        block_time: *block_time,
        signature,
        from_acc: parsed["source"].as_str().unwrap().to_string(),
        to_acc: parsed["destination"].as_str().unwrap().to_string(),
        amount,
        authority: parsed["authority"].as_str().map(|a| a.to_string()),
        // derived values
        from: None,
        to: None,
        decimals: None,
        token: None,
    };

    let lookup = tx.get_account_lookup();

    if let Some(from_acc) = lookup.get(&spl_transfer.from_acc) {
        if let Some(owner) = from_acc.owner.as_ref() {
            spl_transfer.from = Some(owner.clone());
        }
        spl_transfer.token = Some(from_acc.mint.clone());
        spl_transfer.decimals = Some(from_acc.decimals);
    }

    if let Some(to) = lookup.get(&spl_transfer.to_acc) {
        if let Some(owner) = to.owner.as_ref() {
            spl_transfer.to = Some(owner.clone());
        }
        if spl_transfer.token.is_none() {
            spl_transfer.token = Some(to.mint.clone());
            spl_transfer.decimals = Some(to.decimals);
        }
    }

    spl_transfer
}
