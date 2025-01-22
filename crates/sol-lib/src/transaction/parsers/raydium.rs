use arctis_types::{BlockInfo, ParserResult};

use crate::transaction::parsers::Parser;
use crate::transaction::wrapper::TransactionWrapper;
use crate::transaction::InstructionWrapper;
use anyhow::{anyhow, Result};

pub struct RaydiumAmmParser;

impl Parser for RaydiumAmmParser {
    fn parse(
        &self,
        ix: &InstructionWrapper,
        tx: &TransactionWrapper,
        block: &BlockInfo,
    ) -> Result<ParserResult> {
        let BlockInfo { slot, block_time } = block;
        let _event = parse_raydium_event(ix, tx, *slot, *block_time)?;

        Err(anyhow!("Not implemented"))
    }
}

pub fn parse_raydium_event(
    _ix: &InstructionWrapper,
    _tx: &TransactionWrapper,
    _slot: u64,
    _block_time: i64,
) -> Result<()> {
    Err(anyhow!("Not implemented"))
}

#[cfg(test)]
mod tests {
    use arctis_types::{DexType, ParserResultData, SwapInfo, SwapType};

    use crate::transaction::parsers::get_parser;
    use crate::utils::{get_test_data, TestData};

    use super::*;

    #[tokio::test]
    async fn test_ray_parse_swap_base_out_wsol_base_direction_2() {
        // swap base out, base token wsol, direction 2
        let sig = "5RbkAPyAxV6nx4hafHXyz5JDHB62MMjyG3x8dkrzH5ZfYDaWHfhQAsw1y4k5qARARBtYqzsmcGtAKdD8nLrrVHsa";
        let ix_index = 2;
        let ray_ix_index = 0;

        let TestData { tx, block_info, ix } = get_test_data(sig, ix_index).await;
        let ix = InstructionWrapper::new(&ix, ix_index, ray_ix_index);

        let parser = get_parser("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap();
        let res = parser.parse(&ix, &tx, &block_info).unwrap();

        let ParserResult {
            parsed,
            ix_type: _,
            data,
        } = res;

        assert!(parsed);

        assert_eq!(
            data,
            ParserResultData::Swap(SwapInfo {
                slot: block_info.slot,
                signer: tx.get_signer(),
                signature: tx.get_signature(),
                error: false,
                dex: DexType::RaydiumAmm,
                swap_type: SwapType::Buy,
                amount_in: 2.239416485,
                token_in: "So11111111111111111111111111111111111111112".to_string(),
                amount_out: 1_428.217952,
                token_out: "A8C3xuqscfmyLrte3VmTqrAq8kgMASius9AFNANwpump".to_string(),
                block_time: block_info.block_time,
            })
        );
    }
}
