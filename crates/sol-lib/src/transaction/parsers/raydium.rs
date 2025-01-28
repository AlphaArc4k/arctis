use crate::transaction::parsers::Parser;
use crate::transaction::wrapper::TransactionWrapper;
use crate::transaction::InstructionWrapper;
use crate::utils::{format_with_decimals, WSOL};
use anyhow::{anyhow, Result};
use arctis_types::{BlockInfo, DexType, ParserResult, ParserResultData, SwapInfo, SwapType};
use carbon_core::deserialize::CarbonDeserialize;
use carbon_raydium_amm_v4_decoder::instructions::swap_base_in::SwapBaseIn;
use carbon_raydium_amm_v4_decoder::instructions::swap_base_out::SwapBaseOut;
use std::cmp::Ordering;
use std::ops::Mul;

const RAYDIUM_V4_AUTHORITY: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";

pub struct RaydiumAmmParser;

impl Parser for RaydiumAmmParser {
    fn parse(
        &self,
        ix: &InstructionWrapper,
        tx: &TransactionWrapper,
        block: &BlockInfo,
    ) -> Result<ParserResult> {
        let instruction_data = solana_sdk::bs58::decode(&ix.ix.data).into_vec()?;
        if let Some(swap_in) = SwapBaseIn::deserialize(&instruction_data) {
            parse_swap_instruction(Some(swap_in.amount_in), None, block, tx)
        } else if let Some(swap_out) = SwapBaseOut::deserialize(&instruction_data) {
            parse_swap_instruction(None, Some(swap_out.amount_out), block, tx)
        } else {
            Ok(ParserResult {
                parsed: false,
                ix_type: "".to_string(),
                data: ParserResultData::NoData,
            })
        }
    }
}

fn parse_swap_instruction(
    mut amount_in: Option<u64>,
    mut amount_out: Option<u64>,
    block: &BlockInfo,
    tx: &TransactionWrapper,
) -> Result<ParserResult> {
    let BlockInfo { slot, block_time } = *block;
    let accounts = tx.get_account_lookup();
    let signer = tx.get_signer();
    let signature = tx.get_signature();
    let mut token_in = None;
    let mut token_out = None;
    for (_, info) in accounts {
        if let Some(sender) = &info.owner
            && sender == RAYDIUM_V4_AUTHORITY
        {
            let amount_pre = info.amount_pre.mul(10f64.powf(info.decimals as f64)) as u64;
            let amount_post = info.amount_post.mul(10f64.powf(info.decimals as f64)) as u64;
            match amount_post.cmp(&amount_pre) {
                Ordering::Less => {
                    if amount_out.is_none() {
                        amount_out = Some(amount_pre - amount_post);
                    }

                    token_out = Some((info.mint, info.decimals));
                }
                Ordering::Equal => {}
                Ordering::Greater => {
                    if amount_in.is_none() {
                        amount_in = Some(amount_post - amount_pre);
                    }

                    token_in = Some((info.mint, info.decimals));
                }
            }
        }
    }

    if token_in.is_none() {
        token_in = Some((WSOL.to_string(), 9));
    }

    if token_out.is_none() {
        token_out = Some((WSOL.to_string(), 9));
    }

    let (amount_in, amount_out, token_in, token_out) =
        match (amount_in, amount_out, token_in, token_out) {
            (Some(amount_in), Some(amount_out), Some(token_in), Some(token_out)) => {
                (amount_in, amount_out, token_in, token_out)
            }
            _ => {
                return Err(anyhow!(
                    "failed to parse swap data for Raydium in Txn {:?}",
                    tx.get_signature()
                ))
            }
        };

    let swap_type = if token_in.0 == WSOL {
        SwapType::Buy
    } else if token_out.0 == WSOL {
        SwapType::Sell
    } else {
        SwapType::Token
    };

    let swap_info = SwapInfo {
        slot,
        block_time,
        signer,
        signature,
        error: false,
        dex: DexType::RaydiumAmm,
        swap_type,
        amount_in: format_with_decimals(amount_in, token_in.1),
        token_in: token_in.0,
        amount_out: format_with_decimals(amount_out, token_out.1),
        token_out: token_out.0,
    };

    Ok(ParserResult {
        parsed: true,
        ix_type: format!("Trade{}", swap_info.swap_type.to_db()),
        data: ParserResultData::Swap(swap_info),
    })
}

#[cfg(test)]
mod tests {
    use crate::transaction::parsers::get_parser;
    use crate::utils::{get_test_data, TestData};
    use arctis_types::{DexType, ParserResultData, SwapInfo, SwapType};

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

    #[tokio::test]
    async fn test_ray_parse_swap_base_out_wsol_quote_direction_1() {
        // swap base out, base token wsol, direction 2
        let sig = "Rnn5vZ8UxasZFP8qz5VPeJhxAZBpiCbf8f2xdEZTcRH469LvDgmFVD5Aybh4sDncCweDJEowN8F35Dp2gqf1VGK";
        let ix_index = 4;
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
                amount_in: 0.255122577,
                token_in: "So11111111111111111111111111111111111111112".to_string(),
                amount_out: 72_068.28102727,
                token_out: "9HF5nAHD92aGZqZK6aMcQvTNMrsbuFtPNeLL3fJCBUcf".to_string(),
                block_time: block_info.block_time,
            })
        );
    }

    #[tokio::test]
    async fn test_ray_parse_swap_base_out_wsol_base_direction_1() {
        // swap base out, base token wsol, direction 2
        let sig = "4ZzZogn3rfxdESvhR2Zmmr39C4cRG3EkGzoTa6sZqSNueMK9X9zDzjiXU7VTMdUWH5fBZhx7fJWqK6nYwnjmqbxA";
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
                swap_type: SwapType::Sell,
                amount_in: 902.522672,
                token_in: "Df6yfrKC8kZE3KNkrHERKzAetSxbrWeniQfyJY4Jpump".to_string(),
                amount_out: 1.17053854,
                token_out: "So11111111111111111111111111111111111111112".to_string(),
                block_time: block_info.block_time,
            })
        );
    }

    #[tokio::test]
    async fn test_ray_parse_x() {
        let sig = "5msSc7sNb4eRHB8q6g5xXaCUJPirCeRq6gVLmoW2d39Cfp4cqr8wetpU9JqkAJsdtP1eMXhZwALYitZo9t5tmbNn";
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
                amount_in: 0.282836926,
                token_in: "So11111111111111111111111111111111111111112".to_string(),
                amount_out: 8_673_664_150_225.0,
                token_out: "4h9uqNqd9XxE39o5j9ky5XBuQJG1LxMavYsPAXmrDQ9Z".to_string(),
                block_time: block_info.block_time,
            })
        );
    }

    #[tokio::test]
    async fn test_ray_parse_x_1() {
        let sig = "5tMHjBLxgmTZo8wE6MR7NxTJdRCnUMnZVU9tubtfKFZHigUQkhtrwHBSLGtX4f1y5kn8cywN57V8Y5WyCYb2eQij";
        let ix_index = 3;
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
                swap_type: SwapType::Sell,
                amount_in: 49.459194,
                token_in: "9NHCPJ17Jp9AomfGciTMvfSLWpSyWFDuzrQZA5fcsAST".to_string(),
                amount_out: 0.000005,
                token_out: "So11111111111111111111111111111111111111112".to_string(),
                block_time: block_info.block_time,
            })
        );
    }
}
