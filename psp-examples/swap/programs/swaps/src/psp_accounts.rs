use crate::u256;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_merkle_tree_program::{
    program::LightMerkleTreeProgram, transaction_merkle_tree::state::TransactionMerkleTree,
    utils::constants::TOKEN_AUTHORITY_SEED, EventMerkleTree,
};
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
    pub rpc_fee: u64,
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
#[derive(Accounts)]
pub struct LightInstructionThird<'info, const NR_CHECKED_INPUTS: usize> {
    #[account(mut, address=verifier_state.load().unwrap().signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, LightMerkleTreeProgram>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds = [LightMerkleTreeProgram::id().to_bytes().as_ref()], bump, seeds::program=LightPsp4in4outAppStorage::id())]
    pub authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender_spl: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient_spl: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender_sol: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient_sol: UncheckedAccount<'info>,
    /// CHECK:` Is not checked the rpc has complete freedom.
    #[account(mut)]
    pub rpc_recipient_sol: UncheckedAccount<'info>,
    /// CHECK:` Is not checked the rpc has complete freedom.
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program=LightMerkleTreeProgram::id())]
    pub token_authority: UncheckedAccount<'info>,
    /// CHECK: Verifier config pda which needs ot exist Is not checked the rpc has complete freedom.
    #[account(mut, seeds= [LightPsp4in4outAppStorage::id().to_bytes().as_ref()], bump, seeds::program=LightMerkleTreeProgram::id())]
    pub registered_verifier_pda: UncheckedAccount<'info>, //Account<'info, RegisteredVerifier>,
    pub verifier_program: Program<'info, LightPsp4in4outAppStorage>,
    /// CHECK:` It get checked inside the event_call
    pub log_wrapper: UncheckedAccount<'info>,
    #[account(mut)]
    pub event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
    // #[account(mut)]
    // pub swap_pda: Box<Account<'info, SwapPda>>,
}

#[allow(non_snake_case)]
#[derive(Accounts)]
#[instruction(swap_commitment_hash: Vec<u8>)]
pub struct CreateSwapInstruction<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[allow(non_snake_case)]
    #[account(
    init,
    seeds = [&swap_commitment_hash[swap_commitment_hash.len()-64..swap_commitment_hash.len()-32]],
    bump,
    payer = signer,
    space = 3000)
    ]
    pub swap_pda: Account<'info, SwapPda>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseSwap<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
    mut,
    close=signer
    )]
    pub swap_pda: Account<'info, SwapPda>,
}

#[derive(Accounts)]
pub struct JoinSwapInstruction<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    // #[allow(non_snake_case)]
    #[account(mut)]
    pub swap_pda: Account<'info, SwapPda>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug)]
#[account]
pub struct SwapPda {
    pub swap: Swap,
}

#[allow(non_snake_case)]
#[derive(Debug, Copy, AnchorDeserialize, AnchorSerialize, PartialEq, Clone)]
pub struct UtxoInternal {
    pub amounts: [u64; 2],
    pub spl_asset_index: u64,
    pub verifier_address_index: u64,
    pub blinding: u256,
    pub app_data_hash: u256,
    pub account_compression_public_key: u256,
    pub account_encryption_public_key: [u8; 32],
    pub swapCommitmentHash: u256,
    pub userPubkey: u256,
}

#[derive(Debug, AnchorDeserialize, AnchorSerialize, Clone)]
pub struct Swap {
    pub swap_maker_program_utxo: UtxoInternal,
    pub swap_taker_program_utxo: Option<UtxoInternal>,
    _padding: [u8; 7],
    pub slot: Option<u64>,
    pub is_joinable: bool,
    pub _padding2: [u8; 7],
}

impl Swap {
    pub fn new(swap_maker_program_utxo: UtxoInternal) -> Self {
        Self {
            swap_maker_program_utxo,
            swap_taker_program_utxo: None,
            slot: None,
            _padding: [0u8; 7],
            is_joinable: true,
            _padding2: [0u8; 7],
        }
    }

    pub fn join(&mut self, swap_taker_program_utxo: UtxoInternal, slot: u64) {
        self.swap_taker_program_utxo = Some(swap_taker_program_utxo);
        self.is_joinable = false;
        self.slot = Some(slot);
    }
}

#[derive(Debug)]
#[account]
pub struct InstructionDataLightInstructionThird {
    pub proof_a_app: [u8; 32],
    pub proof_b_app: [u8; 64],
    pub proof_c_app: [u8; 32],
    pub proof_a: [u8; 32],
    pub proof_b: [u8; 64],
    pub proof_c: [u8; 32],
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
    pub checked_public_inputs: [[u8; 32]; 2],
}
