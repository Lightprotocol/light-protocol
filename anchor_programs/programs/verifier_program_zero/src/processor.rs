use anchor_lang::prelude::*;
use solana_program::log::sol_log_compute_units;
use crate::verifying_key::VERIFYINGKEY;
use light_verifier_sdk::{
    light_transaction::{
        TxConfig,
        LightTransaction
    },
    accounts::Accounts
};

use crate::LightInstruction;
struct LightTx;
impl TxConfig for LightTx {
    /// Number of nullifiers to be inserted with the transaction.
	const NR_NULLIFIERS: usize = 2;
	/// Number of output utxos.
	const NR_LEAVES: usize = 2;
	/// Number of checked public inputs.
    const NR_CHECKED_PUBLIC_INPUTS: usize = 0;
    /// ProgramId in bytes.
    const ID: [u8;32]  = [
      252, 178,  75, 149,  78, 219, 142,  17,
       53, 237,  47,   4,  42, 105, 173, 204,
      248,  16, 209,  38, 219, 222, 123, 242,
        5,  68, 240, 131,   3, 211, 184,  81
    ];
}

pub fn process_shielded_transfer_2_inputs<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info,LightInstruction<'info>>,
    proof: [u8; 256],
    merkle_root: [u8; 32],
    public_amount: [u8; 32],
    ext_data_hash: [u8; 32],
    nullifier0: [u8; 32],
    nullifier1: [u8; 32],
    leaf_right: [u8; 32],
    leaf_left: [u8; 32],
    ext_amount: i64,
    fee_amount: [u8; 32],
    mint_pubkey: [u8;32],
    encrypted_utxos: Vec<u8>,
    merkle_tree_index: u64,
    relayer_fee: u64,
) -> Result<()> {

    let accounts = Accounts::new(
        ctx.program_id,
        ctx.accounts.signing_address.to_account_info(),
        &ctx.accounts.system_program,
        &ctx.accounts.program_merkle_tree,
        &ctx.accounts.rent,
        &ctx.accounts.merkle_tree,
        &ctx.accounts.pre_inserted_leaves_index,
        ctx.accounts.authority.to_account_info(),
        &ctx.accounts.token_program,
        ctx.accounts.sender.to_account_info(),
        ctx.accounts.recipient.to_account_info(),
        ctx.accounts.sender_fee.to_account_info(),
        ctx.accounts.recipient_fee.to_account_info(),
        ctx.accounts.relayer_recipient.to_account_info(),
        ctx.accounts.escrow.to_account_info(),
        ctx.accounts.token_authority.to_account_info(),
        &ctx.accounts.registered_verifier_pda,
        ctx.remaining_accounts
    )?;

    let mut tx = LightTransaction::<LightTx>::new(
        &proof,
        &merkle_root,
        &public_amount,
        &ext_data_hash,
        &fee_amount,
        &mint_pubkey,
        Vec::<[u8; 32]>::new(), // checked_public_inputs
        vec![nullifier0.to_vec(), nullifier1.to_vec()],
        vec![[leaf_left, leaf_right]],
        encrypted_utxos,
        relayer_fee,
        merkle_tree_index.try_into().unwrap(),
        Some(&accounts),
        &VERIFYINGKEY
    );

    tx.verify()?;
    tx.check_tx_integrity_hash()?;
    tx.check_root()?;
    tx.insert_leaves()?;
    tx.insert_nullifiers()?;
    tx.transfer_user_funds()?;
    tx.transfer_fee()?;
    tx.check_completion()?;
    Ok(())
}
