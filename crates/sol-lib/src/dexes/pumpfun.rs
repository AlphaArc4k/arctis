use anchor_lang::prelude::{borsh, Pubkey};
use anchor_lang::{event, AnchorDeserialize, AnchorSerialize};
use anyhow::{anyhow, Result};
use arctis_types::{DexType, SwapInfo, SwapType};
use base64::Engine;

use crate::transaction::wrapper::TransactionWrapper;
use crate::utils::{format_with_decimals, WSOL};

pub const PUMPFUN_SWAP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

#[event]
#[derive(Debug)]
pub struct TradeEvent {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
}

#[event]
#[derive(Debug)]
pub struct CreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
}

#[event]
#[derive(Debug)]
pub struct CompleteEvent {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
}

#[event]
#[derive(Debug)]
pub struct SetParamsEvent {
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u64,
}

#[derive(Debug)]
pub enum PumpfunEventType {
    Trade(TradeEvent),
    Create(CreateEvent),
    Complete(CompleteEvent),
    SetParams(SetParamsEvent),
}

/// Parse a pumpfun log into a pumpfun event
/// log: base64 encoded log without prefix
pub fn parse_pumpfun_log(log: &str) -> Result<PumpfunEventType> {
    const DISCRIMINATOR_SIZE: usize = 8;

    let bytes = base64::prelude::BASE64_STANDARD
        .decode(log)
        .ok()
        .filter(|bytes| bytes.len() >= DISCRIMINATOR_SIZE);

    if bytes.is_none() {
        return Err(anyhow!("Invalid base64 log"));
    }

    let bytes = bytes.unwrap();

    let (discriminator, buffer) = bytes.split_at(DISCRIMINATOR_SIZE);
    match discriminator {
        [189, 219, 127, 211, 78, 230, 97, 238] => {
            let event = TradeEvent::try_from_slice(buffer)?;
            Ok(PumpfunEventType::Trade(event))
        }
        [27, 114, 169, 77, 222, 235, 99, 118] => {
            let event = CreateEvent::try_from_slice(buffer)?;
            Ok(PumpfunEventType::Create(event))
        }
        [95, 114, 97, 156, 212, 46, 152, 8] => {
            let event = CompleteEvent::try_from_slice(buffer)?;
            Ok(PumpfunEventType::Complete(event))
        }
        [223, 195, 159, 246, 62, 48, 143, 131] => {
            let event = SetParamsEvent::try_from_slice(buffer)?;
            Ok(PumpfunEventType::SetParams(event))
        }
        _ => Err(anyhow!("Invalid pumpfun event discriminator")),
    }
}

pub fn pumpfun_event_to_swap(
    trade_event: &TradeEvent,
    tx: &TransactionWrapper,
    slot: u64,
    block_time: i64,
) -> Result<Option<SwapInfo>> {
    let accounts = tx.get_accounts();

    let swap_type = if trade_event.is_buy {
        SwapType::Buy
    } else {
        SwapType::Sell
    };
    let mint = trade_event.mint.to_string();

    let amount_in;
    let amount_out;
    let token_in;
    let token_out;

    let decimals = tx.get_token_decimals(&mint)?;
    let token_amount = format_with_decimals(trade_event.token_amount, decimals);
    let sol_amount = format_with_decimals(trade_event.sol_amount, 9);

    if swap_type == SwapType::Buy {
        token_in = WSOL.to_string();
        token_out = mint;
        amount_in = sol_amount;
        amount_out = token_amount;
    } else {
        token_in = mint;
        token_out = WSOL.to_string();
        amount_in = token_amount;
        amount_out = sol_amount;
    }

    let signature = tx.get_signature();
    let swap_info = SwapInfo {
        slot,
        block_time,
        signer: accounts[0].clone(),
        signature,
        error: false,
        dex: DexType::Pumpfun,
        swap_type,
        amount_in,
        token_in,
        amount_out,
        token_out,
    };

    Ok(Some(swap_info))
}
