use crate::transaction::parsers::Parser;
use crate::transaction::wrapper::TransactionWrapper;
use crate::transaction::InstructionWrapper;
use crate::utils::{format_with_decimals, WSOL};
use anyhow::anyhow;
use arctis_types::{BlockInfo, DexType, ParserResult, ParserResultData, SwapInfo, SwapType};
use carbon_core::deserialize::CarbonDeserialize;
use carbon_jupiter_swap_decoder::instructions::swap_event::SwapEvent;
use indexmap::IndexMap;
use std::cmp::Ordering;

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
        let mut swap_events = tx
            .get_compiled_inner_instructions_for_instruction(ix.ix_idx as u8)?
            .into_iter()
            .filter_map(|ix| {
                if let Ok(data) = solana_sdk::bs58::decode(&ix.data).into_vec() {
                    SwapEvent::deserialize(&data)
                } else {
                    None
                }
            })
            .collect::<Vec<SwapEvent>>();

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
                // merge amounts from swap with same input + output
                let merged_swap_events = swap_events.into_iter().fold(
                    IndexMap::<String, SwapEvent>::new(),
                    |mut acc, swap| {
                        let key = format!("{}-{}", swap.input_mint, swap.output_mint);
                        acc.entry(key)
                            .and_modify(|val| {
                                val.input_amount += swap.input_amount;
                                val.output_amount += swap.output_amount;
                            })
                            .or_insert(swap);
                        acc
                    },
                );

                // we create a swap event with first and last swap
                let (_, first_swap) = merged_swap_events
                    .first()
                    .ok_or(anyhow!("failed to get first swap"))?;
                let (_, last_swap) = merged_swap_events
                    .last()
                    .ok_or(anyhow!("failed to get last swap"))?;

                let swap_event = SwapEvent {
                    amm: Default::default(),
                    input_mint: first_swap.input_mint,
                    input_amount: first_swap.input_amount,
                    output_mint: last_swap.output_mint,
                    output_amount: last_swap.output_amount,
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

    #[tokio::test]
    async fn test_jup_parse_arbitrage() {
        // token for token swap
        let sig = "4LyPRzAnyWAQZ8ZpE9TD4QRu3NF2vG2XWeSW2G4ekLM6SzztRfuLVmDuabcQTCjyExxhetAk2E3uPaXmYUpxiC5W";
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
                swap_type: SwapType::Buy,
                amount_in: 50.507282721,
                token_in: "So11111111111111111111111111111111111111112".to_string(),
                amount_out: 50.615414038,
                token_out: "So11111111111111111111111111111111111111112".to_string(),
                block_time: block_info.block_time,
            })
        );
    }
}
