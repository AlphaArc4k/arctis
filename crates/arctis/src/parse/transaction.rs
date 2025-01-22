use anyhow::Result;
use arctis_types::{BlockInfo, EncodedTransactionWithStatusMeta, ParserResultData};
use sol_db::solana_db::{ProcessedTransaction, ProgramParserData};
use sol_lib::transaction::wrapper::TransactionWrapper;
use sol_lib::transaction::InstructionWrapper;
use sol_lib::{self as sol};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum DiscardReason {
    Vote,
    Processed,
    Error,
    Unknown,
}
impl Display for DiscardReason {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            DiscardReason::Vote => write!(f, "Vote"),
            DiscardReason::Processed => write!(f, "Processed"),
            DiscardReason::Error => write!(f, "Error"),
            DiscardReason::Unknown => write!(f, "Unknown"),
        }
    }
}

pub fn process_transaction(
    tx: &EncodedTransactionWithStatusMeta,
    slot: u64,
    block_time: i64,
) -> Result<ProcessedTransaction> {
    let tx = TransactionWrapper::new(tx.clone());
    let signature = tx.get_signature().clone();
    let signer = tx.get_signer();
    let has_error = tx.is_error();
    let compute_units_consumed = tx.get_compute_units_consumed();
    let fee = tx.get_fee();
    let version = tx.get_version();

    // store all programs and and if they were parsed or had errors etc
    let mut parsed_programs = vec![];

    // store all parser results
    let mut parsed_ix = vec![];

    let mut discard_reason = None;

    if tx.is_error() {
        // TODO we might not want to discard failed tx
        let processed_tx = ProcessedTransaction {
            slot,
            block_time,
            signature,
            signer,
            has_error,
            top_level_ix_count: 0,
            inner_ix_count: 0,
            compute_units_consumed,
            fee,
            version,
            parsed_programs,
            parsed_ix,
            is_discarded: true,
            discard_reason: Some(DiscardReason::Error.to_string()),
            data: None,
        };
        return Ok(processed_tx);
    }

    // a flag if we have extracted all necessary information and can discard the tx
    // discarded transactions will be removed from the block before persisting it
    // e.g. we want to remove all vote tx to reduce ~30% data
    let mut can_discard = true;

    let top_level_instructions = tx.get_instructions();
    let ix_len = top_level_instructions.len();
    let inner_ix_count = tx.get_inner_ix_count();

    let block_info = BlockInfo {
        slot: slot,
        block_time: block_time,
    };

    /* can happen: see https://solscan.io/tx/X571pNgdt4ny636Gtefyhibg2ezqZ7WQHpoUachTrrRYE12hC4f1UT1hMBbbR9QXJYHB35qYjv4LHatHsdQ6gQa
    if ix_len == 0 {
      return Err(anyhow!("No instructions found for tx: {}", signature));
    }
    */

    // process unfiltered tx
    let mut program_indexes = HashMap::new();

    let accounts = tx.get_accounts();

    for ix_idx in 0..ix_len {
        let ix = &top_level_instructions[ix_idx];
        let program_id = accounts[ix.program_id_index as usize].clone();
        let ix_idx = ix_idx as u8;

        // we're throwing away vote tx
        match program_id.as_str() {
            "Vote111111111111111111111111111111111111111" => {
                discard_reason = Some(DiscardReason::Vote);
                can_discard = true;
                continue;
            }
            _ => {}
        }

        // if e.g. raydium has multiple swaps, we need to keep track which one we are processing to find the correct log
        let program_ix_index = program_indexes
            .entry(program_id.clone())
            .and_modify(|e| *e += 1)
            .or_insert(0);

        // get parser for program based on id
        let parser = sol::transaction::parsers::get_parser(&program_id);
        if parser.is_none() {
            parsed_programs.push(ProgramParserData {
                signature: signature.clone(),
                ix_idx,
                program_id: program_id.clone(),
                ix_type: "no_parser".to_string(),
                parsed: false,
                error: false,
            });
            can_discard = false;
            continue;
        }

        // parse program instruction
        let parser = parser.unwrap();
        let ix_wrapped = InstructionWrapper::new(&ix, ix_idx as usize, *program_ix_index);
        let result = parser.parse(&ix_wrapped, &tx, &block_info);
        if result.is_err() {
            // TODO log errors println!("Failed to parse: program {}  sig {} ix: {} err {:?}", program_id, signature, ix_idx, result.err().unwrap());
            parsed_programs.push(ProgramParserData {
                signature: signature.clone(),
                ix_idx,
                program_id: program_id.clone(),
                ix_type: "unknown".to_string(),
                parsed: false,
                error: true,
            });
            can_discard = false;
            continue;
        }
        let result = result.unwrap();
        let parsed = result.parsed;
        parsed_programs.push(ProgramParserData {
            signature: signature.clone(),
            ix_idx,
            program_id: program_id.clone(),
            ix_type: result.ix_type.clone(),
            parsed,
            error: false,
        });

        let ix_type = &result.ix_type;
        let _can_discard = match &result.data {
            ParserResultData::NoData => match ix_type.as_str() {
                "syncNative" => true,
                "sequence_enforcer" => true,
                _ => false,
            },
            // TODO handle unknown data type
            _ => true,
        };

        parsed_ix.push(result);

        // let _can_discard = write_parsed_ix(&result, &signature, slot, block_time, solana_db);
        // if all programs + instructions are parsed, we can discard the tx
        can_discard &= _can_discard;
    }

    if discard_reason.is_none() {
        if can_discard {
            discard_reason = Some(DiscardReason::Processed);
        } else {
            discard_reason = Some(DiscardReason::Unknown);
        }
    }

    let mut processed_tx = ProcessedTransaction {
        slot: slot,
        block_time: block_time,
        signature: signature,
        signer: signer.clone(),
        has_error: has_error,
        top_level_ix_count: ix_len as u8,
        inner_ix_count: inner_ix_count,
        compute_units_consumed: compute_units_consumed,
        fee: fee,
        version: version,
        parsed_programs,
        parsed_ix,
        is_discarded: true,
        discard_reason: Some(discard_reason.unwrap().to_string()),
        data: None,
    };

    if can_discard == false {
        processed_tx.is_discarded = false;
        processed_tx.discard_reason = None;
        // TODO make setting
        processed_tx.data = None; // Some(tx.tx); // don't write all the data during testing
    }
    return Ok(processed_tx);
}
