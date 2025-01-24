


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jup_parse_swap() {
      // token for token swap
      let sig = "5fSkM83WUxgFbqwfwenfuLdygyCpjqMBzsMJPV7kv6AT6vau6ZygW3eimcFHX8wukM5YcgjV37EH5TzKvbmfqk3d";
      let ix_index = 3;
      let jup_ix_index = 0;

      let TestData{ tx, block_info, ix   } = get_test_data(sig, ix_index).await;
      let ix = InstructionWrapper::new(&ix, ix_index, jup_ix_index);

      let parser = get_parser("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
      let res = parser.parse(&ix, &tx, &block_info).unwrap();

      let ParserResult{parsed, ix_type: _,  data} = res;

      assert!(parsed);

      assert_eq!(data, ParserResultData::Swap(SwapInfo {
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

      }));

    }


    #[tokio::test]
    async fn test_jup_parse_swap_with_create() {
      // token for token swap
      let sig = "SMXkMA9X7tTBFfa1PKATXSFgE1mwXZnCdQymUmgBJwT2YHeZ7d5tZ7DjE7po9wrturs1647gRDAR9K3UXYB9Cho";
      let ix_index = 3; // this call creates a token account
      let jup_ix_index = 0;

      let TestData{ tx, block_info, ix   } = get_test_data(sig, ix_index).await;
      let ix = InstructionWrapper::new(&ix, ix_index, jup_ix_index);

      let parser = get_parser("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
      let res = parser.parse(&ix, &tx, &block_info).unwrap();

      let ParserResult{parsed, ix_type: _,  data: _} = res;

      assert!(parsed);

    }



    #[tokio::test]
    async fn test_jup_parse_dca() {
      // token for token swap
      let sig = "5gLuKvF3AB2T3Sg8ng21wPQdpH7FF4etagot5TcCNHUdkSUPjUWyfUNT8UjjPGirvHjqSUYkhdxzoYToKphjEyMs";
      let ix_index = 3; 
      let jup_ix_index = 0;

      let TestData{ tx, block_info, ix   } = get_test_data(sig, ix_index).await;
      let ix = InstructionWrapper::new(&ix, ix_index, jup_ix_index);

      let parser = get_parser("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
      let res = parser.parse(&ix, &tx, &block_info).unwrap();

      let ParserResult{parsed, ix_type: _,  data: _} = res;

      assert!(parsed);

    }
    

    #[tokio::test]
    async fn test_jup_parse_a1() {
      // token for token swap
      let sig = "32XBF9MKCxpFzCqPTAGgpv8rANeFxvzXAJMZW4D8z1AfQCAiMLEdhMimRAN83Jnz446zqZ2ECwaikLEwQdwrbFQs";
      let ix_index = 3; 
      let jup_ix_index = 0;

      let TestData{ tx, block_info, ix   } = get_test_data(sig, ix_index).await;
      let ix = InstructionWrapper::new(&ix, ix_index, jup_ix_index);

      let parser = get_parser("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
      let res = parser.parse(&ix, &tx, &block_info).unwrap();

      let ParserResult{parsed, ix_type: _,  data: _} = res;

      assert!(parsed);

    }

    #[tokio::test]
    async fn test_jup_parse_a2() {
      // token for token swap
      let sig = "31pTT8rFu3ZAKRSD497JbjdZzZVTFBDKBodKaE5eCKyTLjz9qiuT7jvvj7tYUsxDbgJhTXBDcTzCRyhNn8VdVxDt";
      let ix_index = 2; 
      let jup_ix_index = 0;

      let TestData{ tx, block_info, ix   } = get_test_data(sig, ix_index).await;
      let ix = InstructionWrapper::new(&ix, ix_index, jup_ix_index);

      let parser = get_parser("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap();
      let res = parser.parse(&ix, &tx, &block_info).unwrap();

      let ParserResult{parsed, ix_type: _,  data} = res;

      assert!(parsed);

      assert_eq!(data, ParserResultData::Swap(SwapInfo {
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

      }));

    }

    
}