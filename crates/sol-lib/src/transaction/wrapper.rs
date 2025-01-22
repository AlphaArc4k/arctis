use std::collections::HashMap;

use anyhow::Result;
use arctis_types::SplTokenTransfer;
use solana_sdk::transaction::TransactionVersion;
use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::{
    EncodedTransactionWithStatusMeta, UiCompiledInstruction, UiRawMessage, UiTransaction,
    UiTransactionStatusMeta,
};

use super::helper::{
    get_accounts, get_inner_instructions, get_token_account_lookup, get_token_decimals,
    get_transaction_data, get_transaction_message, get_transaction_meta, get_transaction_signature,
    get_transaction_signatures, has_error, TokenAccountInfo,
};

pub struct TransactionWrapper {
    pub tx: EncodedTransactionWithStatusMeta,
    pub accounts: Vec<String>,
}

#[allow(dead_code)]
impl TransactionWrapper {
    pub fn new(tx: EncodedTransactionWithStatusMeta) -> TransactionWrapper {
        let message = get_transaction_message(&tx);
        let meta = get_transaction_meta(&tx);
        let accounts = get_accounts(message, meta);

        TransactionWrapper { tx, accounts }
    }

    pub fn get_accounts(&self) -> Vec<String> {
        return self.accounts.clone();
    }

    fn get_tx(&self) -> &EncodedTransactionWithStatusMeta {
        return &self.tx;
    }

    pub fn get_signer(&self) -> String {
        return self.get_accounts()[0].clone();
    }

    pub fn get_signature(&self) -> String {
        return get_transaction_signature(&self.tx);
    }

    pub fn get_signatures(&self) -> Vec<String> {
        return get_transaction_signatures(&self.tx);
    }

    fn is_multisig(&self) -> bool {
        return self.get_signatures().len() > 1;
    }

    pub fn is_error(&self) -> bool {
        return has_error(&self.tx);
    }

    pub fn get_version(&self) -> i8 {
        match &self.tx.version {
            Some(version) => match version {
                TransactionVersion::Legacy(_) => -1,
                TransactionVersion::Number(v) => *v as i8,
            },
            None => -2,
        }
    }

    pub fn get_compute_units_consumed(&self) -> u64 {
        return self
            .tx
            .meta
            .as_ref()
            .unwrap()
            .compute_units_consumed
            .clone()
            .unwrap_or(0);
    }

    pub fn get_fee(&self) -> u64 {
        return self.tx.meta.as_ref().unwrap().fee;
    }

    fn get_transaction_data(&self) -> &UiTransaction {
        return get_transaction_data(&self.tx);
    }

    fn get_transaction_message(&self) -> &UiRawMessage {
        return get_transaction_message(&self.tx);
    }

    pub fn get_transaction_meta(&self) -> &UiTransactionStatusMeta {
        return get_transaction_meta(&self.tx);
    }

    pub fn get_instructions(&self) -> Vec<UiCompiledInstruction> {
        let message = self.get_transaction_message();
        return message.instructions.clone();
    }

    pub fn get_inner_ix_count(&self) -> u8 {
        let meta = self.get_transaction_meta();
        return meta
            .inner_instructions
            .as_ref()
            .map_or(0, |inner| inner.len() as u8);
    }

    pub fn get_inner_instructions(&self, program_id: &str) -> Result<Vec<UiCompiledInstruction>> {
        return get_inner_instructions(&self.tx, program_id);
    }

    pub fn get_account_lookup(&self) -> HashMap<String, TokenAccountInfo> {
        let accounts = self.get_accounts().clone();
        let tx = self.get_tx();
        let lookup = get_token_account_lookup(tx, &accounts, false);
        return lookup;
    }

    pub fn get_inner_token_transfers(&self, _program_id: &str) -> Result<Vec<SplTokenTransfer>> {
        /*
        let accounts = self.get_accounts().clone();
        let tx = self.get_tx();
        */
        // return get_inner_token_transfers(&tx, program_id, &accounts);
        return Err(anyhow::anyhow!("Not implemented"));
    }

    pub fn get_token_decimals(&self, mint: &str) -> Result<u8> {
        return get_token_decimals(&self.tx, mint);
    }

    pub fn get_log_messages(&self) -> Option<Vec<String>> {
        let logs = self.tx.meta.as_ref().unwrap().log_messages.clone();
        let OptionSerializer::Some(logs) = logs else {
            return None;
        };
        return Some(logs);
    }
}
