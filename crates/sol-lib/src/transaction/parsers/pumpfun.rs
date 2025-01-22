use crate::dexes::pumpfun::{
    parse_pumpfun_log, pumpfun_event_to_swap, PumpfunEventType, PUMPFUN_PROGRAM_ID,
};
use crate::transaction::wrapper::TransactionWrapper;
use crate::transaction::InstructionWrapper;

use super::Parser;
use anyhow::{anyhow, Result};
use arctis_types::{BlockInfo, NewToken, ParserResult, ParserResultData};

pub struct PumpfunParser;

impl Parser for PumpfunParser {
    fn parse(
        &self,
        ix: &InstructionWrapper,
        tx: &TransactionWrapper,
        block: &BlockInfo,
    ) -> Result<ParserResult> {
        let BlockInfo { slot, block_time } = block;

        let pump_idx = ix.pix_idx;

        let logs = tx.get_log_messages().unwrap();
        // FIXME we might have multiple different programs emitting "Program data: " logs. make method get_pumpfun_logs that checks we are in the correct invoke
        let logs = logs
            .iter()
            .filter_map(|log| log.strip_prefix("Program data: "))
            .collect::<Vec<&str>>();

        if logs.is_empty() {
            return Err(anyhow!("No pumpfun logs found"));
        } else if logs.len() <= pump_idx as usize {
            return Err(anyhow!("Pumpfun: Invalid pumpfun index"));
        }
        // else if logs.len() > 1 { return Err(anyhow!("Pumpfun: Multiple logs found")); }

        let log = logs.get(pump_idx as usize).unwrap();

        let event = parse_pumpfun_log(log)?;

        match event {
            PumpfunEventType::Create(create_event) => {
                let create = NewToken {
                    block_time: *block_time,
                    slot: *slot,
                    signer: tx.get_signer(),
                    signature: tx.get_signature(),
                    factory: PUMPFUN_PROGRAM_ID.to_string(),
                    mint: create_event.mint.to_string(),
                    name: create_event.name.to_string(),
                    symbol: create_event.symbol.to_string(),
                    uri: create_event.uri.to_string(),
                    decimals: 6, // TODO get from initializeMint2 inner ix
                    initial_supply: Some(1_000_000_000), // TODO get from MintTo inner ix
                    supply: Some(1_000_000_000),
                };
                Ok(ParserResult {
                    parsed: true,
                    ix_type: "Create".to_string(),
                    data: ParserResultData::Token(create),
                })
            }
            PumpfunEventType::Trade(trade_event) => {
                let swap_info = pumpfun_event_to_swap(&trade_event, tx, *slot, *block_time)?;
                if swap_info.is_none() {
                    return Ok(ParserResult {
                        parsed: false,
                        ix_type: "Trade".to_string(),
                        data: ParserResultData::NoData,
                    });
                }
                let swap_info = swap_info.unwrap();
                let ix_type = format!("Trade{}", swap_info.swap_type.to_db());
                Ok(ParserResult {
                    parsed: true,
                    ix_type,
                    data: ParserResultData::Swap(swap_info),
                })
            }
            PumpfunEventType::SetParams(_) => {
                Ok(ParserResult {
                    parsed: true,
                    ix_type: "SetParams".to_string(),
                    data: ParserResultData::NoData,
                })
            }
            PumpfunEventType::Complete(_) => {
                Ok(ParserResult {
                    parsed: true,
                    ix_type: "Complete".to_string(),
                    data: ParserResultData::NoData,
                })
            }
        }
    }
}
