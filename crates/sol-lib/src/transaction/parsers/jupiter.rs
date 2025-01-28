use crate::transaction::parsers::Parser;
use crate::transaction::wrapper::TransactionWrapper;
use crate::transaction::InstructionWrapper;
use crate::utils::{format_with_decimals, WSOL};
use anyhow::anyhow;
use arctis_types::{BlockInfo, DexType, ParserResult, ParserResultData, SwapInfo, SwapType};
use carbon_core::deserialize::CarbonDeserialize;
use carbon_jupiter_swap_decoder::instructions::swap_event::SwapEvent;
use solana_transaction_status::UiInstruction;
use std::cmp::Ordering;
use std::collections::HashMap;

pub struct JupiterV6Parser;

impl Parser for JupiterV6Parser {
    fn parse(
        &self,
        ix: &InstructionWrapper,
        tx: &TransactionWrapper,
        block: &BlockInfo,
    ) -> anyhow::Result<ParserResult> {
        // take the inner instructions for the jupiter program index
        // these instructions contain swap events.
        let inner_instructions = tx
            .get_transaction_meta()
            .clone()
            .inner_instructions
            .unwrap()
            .into_iter()
            .filter_map(|inner| {
                if inner.index == ix.ix_idx as u8 {
                    Some(inner.instructions)
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<UiInstruction>>();

        let mut swap_events = vec![];
        for inner_instruction in inner_instructions {
            match inner_instruction {
                UiInstruction::Compiled(ix)
                    if let Ok(data) = solana_sdk::bs58::decode(&ix.data).into_vec() =>
                {
                    if let Some(swap_event) = SwapEvent::deserialize(&data) {
                        swap_events.push(swap_event);
                    }
                }
                _ => continue,
            }
        }

        match swap_events.len().cmp(&1) {
            // if there are no swap events, nothing to do here
            Ordering::Less => Ok(ParserResult {
                parsed: false,
                ix_type: "".to_string(),
                data: ParserResultData::NoData,
            }),
            // if there is one swap event, then there are no intermediate swaps
            // single swap event only
            Ordering::Equal => parse_swap_instruction(swap_events.pop().unwrap(), block, tx),
            Ordering::Greater => {
                // if there are multiple swap events,
                // for example, token_1 -> SOL -> token_2 -> token_3
                // we only care about the token_1 and token_3 swap
                // capture all the input and output swap event
                // example: inputs -> [token_1, SOL, token_2]
                //          outputs -> [SOL, token_2, token_3]
                let (mut inputs, mut outputs) = swap_events.into_iter().fold(
                    (HashMap::new(), HashMap::new()),
                    |mut acc, swap| {
                        acc.0
                            .entry(swap.input_mint)
                            .and_modify(|val| *val += swap.input_amount)
                            .or_insert(swap.input_amount);

                        acc.1
                            .entry(swap.output_mint)
                            .and_modify(|val| *val += swap.output_amount)
                            .or_insert(swap.output_amount);

                        acc
                    },
                );

                // collect all the comment tokens between inputs and output
                // Example: common keys -> [SOL, token_2]
                let common_keys: Vec<_> = inputs
                    .iter()
                    .filter_map(|(key, _)| {
                        if outputs.contains_key(key) {
                            Some(*key)
                        } else {
                            None
                        }
                    })
                    .collect();

                // remove all the common tokens between inputs and outputs
                for key in common_keys {
                    inputs.remove(&key);
                    outputs.remove(&key);
                }

                // once the common keys are removed from inputs and output
                // inputs and outputs will contain just first token_in and last token_out respectively
                // Example: Inputs -> [token_1]
                //          Outputs -> [token_3]
                // we create a swap event with input token and output token.
                // we just the first token from input and output using `next`
                let input = inputs.into_iter().next().ok_or(anyhow!("Invalid swap"))?;
                let output = outputs.into_iter().next().ok_or(anyhow!("Invalid swap"))?;
                let swap_event = SwapEvent {
                    amm: Default::default(),
                    input_mint: input.0,
                    input_amount: input.1,
                    output_mint: output.0,
                    output_amount: output.1,
                };
                parse_swap_instruction(swap_event, block, tx)
            }
        }
    }
}

fn parse_swap_instruction(
    swap_event: SwapEvent,
    block: &BlockInfo,
    tx: &TransactionWrapper,
) -> anyhow::Result<ParserResult> {
    let BlockInfo { slot, block_time } = *block;
    let signer = tx.get_signer();
    let signature = tx.get_signature();
    let SwapEvent {
        amm: _,
        input_mint,
        input_amount,
        output_mint,
        output_amount,
    } = swap_event;

    let token_in = input_mint.to_string();
    let token_out = output_mint.to_string();

    let swap_type = if token_in == WSOL {
        SwapType::Buy
    } else if token_out == WSOL {
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
        dex: DexType::Jupiterv6,
        swap_type,
        amount_in: format_with_decimals(input_amount, tx.get_token_decimals(&token_in)?),
        token_in,
        amount_out: format_with_decimals(output_amount, tx.get_token_decimals(&token_out)?),
        token_out,
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
    use crate::transaction::InstructionWrapper;
    use crate::utils::{get_test_data, TestData};
    use arctis_types::{DexType, ParserResult, ParserResultData, SwapInfo, SwapType};

    #[tokio::test]
    async fn test_jup_parse_swap() {
        // token for token swap
        let sig = "5fSkM83WUxgFbqwfwenfuLdygyCpjqMBzsMJPV7kv6AT6vau6ZygW3eimcFHX8wukM5YcgjV37EH5TzKvbmfqk3d";
        let ix_index = 3;
        let jup_ix_index = 0;

        let TestData { tx, block_info, ix } = get_test_data(sig, ix_index).await;
        let ix = InstructionWrapper::new(&ix, ix_index, jup_ix_index);

        let parser = get_parser("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
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
                dex: DexType::Jupiterv6,
                swap_type: SwapType::Token,
                amount_in: 0.008978724,
                token_in: "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn".to_string(),
                amount_out: 41.24039,
                token_out: "ZEXy1pqteRu3n13kdyh4LwPQknkFk3GzmMYMuNadWPo".to_string(),
                block_time: block_info.block_time,
            })
        );
    }

    #[tokio::test]
    async fn test_jup_parse_swap_with_create() {
        // token for token swap
        let sig = "SMXkMA9X7tTBFfa1PKATXSFgE1mwXZnCdQymUmgBJwT2YHeZ7d5tZ7DjE7po9wrturs1647gRDAR9K3UXYB9Cho";
        let ix_index = 4; // this call creates a token account
        let jup_ix_index = 0;

        let TestData { tx, block_info, ix } = get_test_data(sig, ix_index).await;
        let ix = InstructionWrapper::new(&ix, ix_index, jup_ix_index);

        let parser = get_parser("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
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
                dex: DexType::Jupiterv6,
                swap_type: SwapType::Buy,
                amount_in: 0.127,
                token_in: "So11111111111111111111111111111111111111112".to_string(),
                amount_out: 771988.318850934,
                token_out: "uXZ7KL88jMaTLwutH9cF6xkp7dZY9JAP5Xx55Y3AyAc".to_string(),
                block_time: block_info.block_time,
            })
        );
    }

    #[tokio::test]
    async fn test_jup_parse_dca() {
        // token for token swap
        let sig = "5gLuKvF3AB2T3Sg8ng21wPQdpH7FF4etagot5TcCNHUdkSUPjUWyfUNT8UjjPGirvHjqSUYkhdxzoYToKphjEyMs";
        let ix_index = 2;
        let jup_ix_index = 0;

        let TestData { tx, block_info, ix } = get_test_data(sig, ix_index).await;
        let ix = InstructionWrapper::new(&ix, ix_index, jup_ix_index);

        let parser = get_parser("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
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
                dex: DexType::Jupiterv6,
                swap_type: SwapType::Token,
                amount_in: 32.661936,
                token_in: "BsQCC4D2AZhC9RctuugBKLCWaNycwmZTzwpUjgGHXWbw".to_string(),
                amount_out: 154.873619,
                token_out: "7LFeJiV7cfQhwpxUEECpGKmBisfPWkL8FZXFUFBbka5b".to_string(),
                block_time: block_info.block_time,
            })
        );
    }

    #[tokio::test]
    async fn test_jup_parse_a1() {
        // token for token swap
        let sig = "32XBF9MKCxpFzCqPTAGgpv8rANeFxvzXAJMZW4D8z1AfQCAiMLEdhMimRAN83Jnz446zqZ2ECwaikLEwQdwrbFQs";
        let ix_index = 3;
        let jup_ix_index = 0;

        let TestData { tx, block_info, ix } = get_test_data(sig, ix_index).await;
        let ix = InstructionWrapper::new(&ix, ix_index, jup_ix_index);

        let parser = get_parser("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
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
                dex: DexType::Jupiterv6,
                swap_type: SwapType::Sell,
                amount_in: 4877724.98868,
                token_in: "GnRM2GWje8Ak8J1jkw9T7Q29X8L68TjXfmyf5v4npump".to_string(),
                amount_out: 8.207473814,
                token_out: "So11111111111111111111111111111111111111112".to_string(),
                block_time: block_info.block_time,
            })
        );
    }

    #[tokio::test]
    async fn test_jup_parse_a2() {
        // token for token swap
        let sig = "31pTT8rFu3ZAKRSD497JbjdZzZVTFBDKBodKaE5eCKyTLjz9qiuT7jvvj7tYUsxDbgJhTXBDcTzCRyhNn8VdVxDt";
        let ix_index = 2;
        let jup_ix_index = 0;

        let TestData { tx, block_info, ix } = get_test_data(sig, ix_index).await;
        let ix = InstructionWrapper::new(&ix, ix_index, jup_ix_index);

        let parser = get_parser("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
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
                dex: DexType::Jupiterv6,
                swap_type: SwapType::Token,
                amount_in: 2_451_900.850405,
                token_in: "EBGaJP7srpUUN8eRdta1MsojrNtweuHYsdP3P1TRpump".to_string(),
                amount_out: 266_372.411808,
                token_out: "HNg5PYJmtqcmzXrv6S9zP1CDKk5BgDuyFBxbvNApump".to_string(),
                block_time: block_info.block_time,
            })
        );
    }
}
