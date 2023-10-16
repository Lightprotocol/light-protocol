use anchor_lang::{prelude::*, solana_program::hash::hash};
use bytemuck::{Pod, Zeroable};
use light_macros::{light_verifier_accounts, pubkey};
use light_verifier_sdk::light_transaction::{
    Amounts, Config, Proof, Transaction, TransactionInput,
};

pub mod verifying_key;
use verifying_key::VERIFYINGKEY_TRANSACTION_APP4_MAIN;

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_protocol_verifier_program_two",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-onchain/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-onchain"
}

declare_id!("2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86");

#[constant]
pub const PROGRAM_ID: &str = "2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86";

#[error_code]
pub enum ErrorCode {
    #[msg("System program is no valid verifier.")]
    InvalidVerifier,
}

#[derive(Clone)]
pub struct TransactionConfig;
impl Config for TransactionConfig {
    /// ProgramId.
    const ID: Pubkey = pubkey!("2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86");
}

#[program]
pub mod light_psp4in4out_app_storage {
    use super::*;

    /// This instruction is used to invoke this system verifier and can only be invoked via cpi.
    pub fn shielded_transfer_inputs<'info>(
        ctx: Context<'_, '_, '_, 'info, LightInstruction<'info>>,
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
        connecting_hash: [u8; 32],
        start_offset: usize,
    ) -> Result<()> {
        let proof = Proof {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        };
        // + 8 to account for the discriminator
        let start_offset = start_offset;
        let end_offset =
            start_offset + 8 + std::mem::size_of::<Psp4In4OutAppStorageVerifierState>();

        let verifier_state = Psp4In4OutAppStorageVerifierState::try_deserialize_unchecked(
            &mut &ctx.accounts.verifier_state.to_account_info().data.borrow()[32..end_offset],
        )?;

        let public_amount = Amounts {
            sol: verifier_state.public_amount_sol,
            spl: verifier_state.public_amount_spl,
        };

        if *ctx.accounts.verifier_state.owner == ctx.accounts.system_program.key() {
            return err!(crate::ErrorCode::InvalidVerifier);
        };

        let mut owner_hash = hash(&ctx.accounts.verifier_state.owner.to_bytes()).to_bytes();
        owner_hash[0] = 0;
        let checked_inputs = [owner_hash, connecting_hash];
        let leaves = [
            [verifier_state.leaves[0], verifier_state.leaves[1]],
            [verifier_state.leaves[2], verifier_state.leaves[3]],
        ];

        let nullifiers: [[u8; 32]; 4] = verifier_state.nullifiers.to_vec().try_into().unwrap();
        let pool_type = [0u8; 32];
        let input = TransactionInput {
            ctx: &ctx,
            message: None,
            proof: &proof,
            public_amount: &public_amount,
            checked_public_inputs: &checked_inputs,
            nullifiers: &nullifiers,
            leaves: &leaves,
            encrypted_utxos: &verifier_state.encrypted_utxos.to_vec(),
            relayer_fee: verifier_state.relayer_fee,
            merkle_root_index: verifier_state.merkle_root_index as usize,
            pool_type: &pool_type,
            verifyingkey: &VERIFYINGKEY_TRANSACTION_APP4_MAIN,
        };
        let mut tx = Transaction::<2, 2, 4, 15, LightInstruction<'info>>::new(input);

        tx.transact()
    }
}

#[light_verifier_accounts(sol, spl)]
#[derive(Accounts)]
pub struct LightInstruction<'info> {
    pub verifier_state: Signer<'info>,
}

#[derive(Debug, Copy, Zeroable)]
#[account]
pub struct Psp4In4OutAppStorageVerifierState {
    pub nullifiers: [[u8; 32]; 4],
    pub leaves: [[u8; 32]; 4],
    pub public_amount_spl: [u8; 32],
    pub public_amount_sol: [u8; 32],
    pub relayer_fee: u64,
    pub encrypted_utxos: [u8; 512],
    pub merkle_root_index: u64,
}
unsafe impl Pod for Psp4In4OutAppStorageVerifierState {}
