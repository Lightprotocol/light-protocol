use std::str::FromStr;

use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};

use crate::{utxo::Utxo, InstructionDataTransfer, TransferInstruction};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PublicTransactionEvent {
    pub in_utxos: Vec<Utxo>,
    pub out_utxos: Vec<Utxo>,
    pub de_compress_amount: Option<u64>,
    pub out_utxo_indices: Vec<u64>,
    pub relay_fee: Option<u64>,
    pub message: Option<Vec<u8>>,
}

#[inline(never)]
pub fn invoke_indexer_transaction_event<T>(event: &T, noop_program: &AccountInfo) -> Result<()>
where
    T: AnchorSerialize,
{
    if noop_program.key()
        != Pubkey::from_str("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV").unwrap()
    {
        return err!(crate::ErrorCode::InvalidNoopPubkey);
    }
    let instruction = Instruction {
        program_id: noop_program.key(),
        accounts: vec![],
        data: event.try_to_vec()?,
    };
    invoke(&instruction, &[noop_program.to_account_info()])?;
    Ok(())
}

pub fn emit_state_transition_event<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    out_utxos: &[Utxo],
    out_utxo_indices: &[u32],
) -> anchor_lang::Result<PublicTransactionEvent> {
    let event = PublicTransactionEvent {
        in_utxos: inputs
            .in_utxos
            .iter()
            .map(|in_utxo_tuple: &crate::utxo::InUtxoTuple| in_utxo_tuple.in_utxo.clone())
            .collect(),
        out_utxos: out_utxos.to_vec(),
        out_utxo_indices: out_utxo_indices.iter().map(|x| *x as u64).collect(),
        de_compress_amount: None,
        relay_fee: inputs.relay_fee,
        message: None,
    };
    invoke_indexer_transaction_event(&event, &ctx.accounts.noop_program)?;
    Ok(event)
}

#[test]
fn create_test_data_no_tlv() {
    let in_utxo = Utxo {
        owner: crate::ID,
        blinding: [1u8; 32],
        lamports: 3u64,
        data: None,
        address: None,
    };

    println!("in_utxo data {:?}", in_utxo.try_to_vec().unwrap());
    let out_utxo = Utxo {
        owner: account_compression::ID,
        blinding: [2u8; 32],
        lamports: 4u64,
        data: None,
        address: None,
    };
    println!("out_utxo data {:?}", out_utxo.try_to_vec().unwrap());

    let event = PublicTransactionEvent {
        in_utxos: vec![in_utxo],
        out_utxos: vec![out_utxo],
        out_utxo_indices: vec![1],
        de_compress_amount: None,
        relay_fee: None,
        message: None,
    };
    let data = event.try_to_vec().unwrap();
    println!("event data {:?}", data);
}
