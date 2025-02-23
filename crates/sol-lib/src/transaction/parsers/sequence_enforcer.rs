use crate::transaction::wrapper::TransactionWrapper;
use crate::transaction::InstructionWrapper;

use super::Parser;
use anyhow::Result;
use arctis_types::{BlockInfo, ParserResult, ParserResultData};

pub struct SequenceEnforcerParser;

impl Parser for SequenceEnforcerParser {
    fn parse(
        &self,
        _ix: &InstructionWrapper,
        _tx: &TransactionWrapper,
        _block: &BlockInfo,
    ) -> Result<ParserResult> {
        Ok(ParserResult {
            parsed: true,
            ix_type: "sequence_enforcer".to_string(),
            data: ParserResultData::NoData,
        })
    }
}
