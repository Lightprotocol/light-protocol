use anchor_lang::prelude::*;
use light_macros::light_verifier_accounts;
use light_psp4in4out_app_storage::{self, program::LightPsp4in4outAppStorage};
use light_verifier_sdk::light_transaction::VERIFIER_STATE_SEED;

// Send and stores data.
#[derive(Accounts)]
pub struct LightInstructionFirst<'info, const NR_CHECKED_INPUTS: usize> {
    /// First transaction, therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(init, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, space= 3000, payer = signing_address )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
}

#[derive(Debug)]
#[account]
pub struct InstructionDataLightInstructionFirst {
    pub public_amount_spl: [u8; 32],
    pub input_nullifier: [[u8; 32]; 4],
    pub output_commitment: [[u8; 32]; 4],
    pub public_amount_sol: [u8; 32],
    pub transaction_hash: [u8; 32],
    pub root_index: u64,
    pub relayer_fee: u64,
    pub encrypted_utxos: Vec<u8>,
}

#[derive(Accounts)]
pub struct LightInstructionSecond<'info, const NR_CHECKED_INPUTS: usize> {
    /// First transaction, therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
}

/// Executes light transaction with state created in the first instruction.
#[light_verifier_accounts(
    sol,
    spl,
    signing_address=verifier_state.load().unwrap().signer,
    verifier_program_id=LightPsp4in4outAppStorage::id()
)]
#[derive(Accounts)]
pub struct LightInstructionThird<'info, const NR_CHECKED_INPUTS: usize> {
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    pub verifier_program: Program<'info, LightPsp4in4outAppStorage>,
}

#[derive(Debug)]
#[account]
pub struct InstructionDataLightInstructionThird {
    pub proof_a_app: [u8; 64],
    pub proof_b_app: [u8; 128],
    pub proof_c_app: [u8; 64],
    pub proof_a: [u8; 64],
    pub proof_b: [u8; 128],
    pub proof_c: [u8; 64],
}

#[derive(Accounts)]
pub struct CloseVerifierState<'info, const NR_CHECKED_INPUTS: usize> {
    #[account(mut, address=verifier_state.load().unwrap().signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
}

#[account(zero_copy)]
pub struct VerifierState {
    pub signer: Pubkey,
    pub verifier_state_data: [u8; 1024], //VerifierState,
    // 2 + PublicZ (VerifierAddress, TransactionHash, PublicZ)
    // Verifier address, and transaction hash are mandatory psp public inputs
    // Verifier address is hashed and truncated to fit the field size of the circuit
    pub checked_public_inputs: [[u8; 32]; 3],
}
