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
    pub out_utxo_indices: Vec<u64>,
    pub de_compress_amount: Option<u64>,
    pub rpc_fee: Option<u64>,
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
            .map(|(utxo, _, _)| utxo.clone())
            .collect(),
        out_utxos: out_utxos.to_vec(),
        out_utxo_indices: out_utxo_indices.iter().map(|x| *x as u64).collect(),
        de_compress_amount: None,
        rpc_fee: inputs.rpc_fee,
        message: None,
    };
    invoke_indexer_transaction_event(&event, &ctx.accounts.noop_program)?;
    Ok(event)
}
