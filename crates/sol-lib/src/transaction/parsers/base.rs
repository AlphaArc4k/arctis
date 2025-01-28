use super::associated_token_account::AssociatedTokenAccountProgramParser;
use super::compute_budget::ComputeBudgetProgramParser;
use super::pumpfun::PumpfunParser;
use super::raydium::RaydiumAmmParser;
use super::sequence_enforcer::SequenceEnforcerParser;
use super::system_program::SystemProgramParser;
use super::token_program::TokenProgramParser;
use crate::transaction::parsers::jupiter::JupiterV6Parser;
use crate::transaction::wrapper::TransactionWrapper;
use crate::transaction::InstructionWrapper;
use anyhow::Result;
use arctis_types::{BlockInfo, ParserResult, ParserResultData};

pub trait Parser {
    // oix is the program-specific instruction index (relative to program not transaction)
    fn parse(
        &self,
        ix: &InstructionWrapper,
        tx: &TransactionWrapper,
        block: &BlockInfo,
    ) -> Result<ParserResult>;
}

struct NoopParser;
impl Parser for NoopParser {
    fn parse(
        &self,
        _ix: &InstructionWrapper,
        _tx: &TransactionWrapper,
        _block: &BlockInfo,
    ) -> Result<ParserResult> {
        Ok(ParserResult {
            parsed: false,
            ix_type: "NoOp".to_string(),
            data: ParserResultData::NoData,
        })
    }
}

pub fn get_parser(program_id: &str) -> Option<Box<dyn Parser>> {
    match program_id {
        "11111111111111111111111111111111" => Some(Box::new(SystemProgramParser)),
        "ComputeBudget111111111111111111111111111111" => Some(Box::new(ComputeBudgetProgramParser)),

        // ########################## SPL ##########################
        // Associated Token Account Program
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" => {
            Some(Box::new(AssociatedTokenAccountProgramParser))
        }
        // Token Program
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => Some(Box::new(TokenProgramParser)),
        // MEMO
        "Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo" => Some(Box::new(NoopParser)),
        "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr" => Some(Box::new(NoopParser)),
        // Sequence Enforcer
        "GDDMwNyyx8uB6zrqwBFHjLLG3TBYk2F8Az4yrQC5RzMp" => Some(Box::new(SequenceEnforcerParser)),

        // ########################## DEXES ##########################
        // Raydium v4
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => Some(Box::new(RaydiumAmmParser)),
        // Openbook V2
        "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb" => Some(Box::new(NoopParser)),
        // Jupiter Aggregator v6
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => Some(Box::new(JupiterV6Parser)),
        // Jupiter Aggregator v4
        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => Some(Box::new(NoopParser)),
        // Jupiter DCA program
        // "DCA265Vj8a9CEuX1eb1LWRnDT7uK6q1xMipnNyatn23M" => Some(Box::new(JupiterDCAParser)),
        // Pumpfun
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => Some(Box::new(PumpfunParser)),
        // Raydium AMM Router
        "routeUGWgWzqBWFcrCfv8tritsqukccJPu3q5GPP3xS" => Some(Box::new(NoopParser)),
        // https://github.com/Ellipsis-Labs/phoenix-v1
        "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY" => Some(Box::new(NoopParser)),
        // OKX DEX: Aggregation Router V2
        "6m2CDdhRgxpH4WjvdzxAYbGxwdGUz5MziiL5jek2kBma" => Some(Box::new(NoopParser)),

        // ########################## GAMING ##########################
        // star atlas sage
        "SAGE2HAwep459SNq61LHvjxPk4pLPEJLoMETef7f7EE" => Some(Box::new(NoopParser)),

        // ########################## PERPS ##########################
        // https://www.drift.trade/
        "dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH" => Some(Box::new(NoopParser)),
        // https://www.zeta.markets/
        "ZETAxsqBRek56DhiGXrn75yj2NHU3aYUnxvHXpkf3aD" => Some(Box::new(NoopParser)),

        // ########################## ORACLES ##########################
        // chainlink data store
        "cjg3oHmg9uuPsP8D6g29NWvhySJkdYdAo9D25PRbKXJ" => Some(Box::new(NoopParser)),
        // pyth oracle
        "pythWSnswVUd12oZpeFP8e9CVaEqJg25g1Vtc2biRsT" => Some(Box::new(NoopParser)),

        // ########################## DeFi ##########################
        // monaco liquidity network : https://www.monacoprotocol.xyz/
        "monacoUXKtUi6vKsQwaLyxmXKSievfNWEcYXTgkbCih" => Some(Box::new(NoopParser)),

        // ########################## Trading Bots ##########################
        // Trojan
        "tro46jTMkb56A3wPepo5HT7JcvX9wFWvR8VaJzgdjEf" => Some(Box::new(NoopParser)),

        // ########################## OTHERS ##########################
        // JITO tip program
        "T1pyyaTNZsKv2WcRAB8oVnk93mLJw2XzjtVYqCsaHqt" => Some(Box::new(NoopParser)),
        // SOL incinerator
        "F6fmDVCQfvnEq2KR8hhfZSEczfM9JK9fWbCsYJNbTGn7" => Some(Box::new(NoopParser)),

        _ => None,
    }
}
