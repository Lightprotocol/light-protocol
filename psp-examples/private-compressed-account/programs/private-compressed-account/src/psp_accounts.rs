use crate::processor::TransactionsConfig;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_merkle_tree_program::transaction_merkle_tree::state::TransactionMerkleTree;
use light_merkle_tree_program::utils::constants::TOKEN_AUTHORITY_SEED;
use light_merkle_tree_program::{program::LightMerkleTreeProgram, EventMerkleTree};
use light_psp4in4out_app_storage::{self, program::LightPsp4in4outAppStorage};
use light_verifier_sdk::{light_transaction::VERIFIER_STATE_SEED, state::VerifierState10Ins};

// Send and stores data.
#[derive(Accounts)]
pub struct LightInstructionFirst<'info, const NR_CHECKED_INPUTS: usize> {
    /// First transaction, therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(init, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, space= 3000, payer = signing_address )]
    pub verifier_state: Account<'info, VerifierState10Ins<NR_CHECKED_INPUTS, TransactionsConfig>>,
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
    pub verifier_state: Account<'info, VerifierState10Ins<NR_CHECKED_INPUTS, TransactionsConfig>>,
}

/// Executes light transaction with state created in the first instruction.
#[derive(Accounts)]
pub struct LightInstructionThird<'info, const NR_CHECKED_INPUTS: usize> {
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Account<'info, VerifierState10Ins<NR_CHECKED_INPUTS, TransactionsConfig>>,
    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, LightMerkleTreeProgram>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [LightMerkleTreeProgram::id().to_bytes().as_ref()], bump, seeds::program=LightPsp4in4outAppStorage::id())]
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
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut)]
    pub relayer_recipient_sol: UncheckedAccount<'info>,
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program=LightMerkleTreeProgram::id())]
    pub token_authority: UncheckedAccount<'info>,
    /// CHECK: Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(mut, seeds= [LightPsp4in4outAppStorage::id().to_bytes().as_ref()], bump, seeds::program=LightMerkleTreeProgram::id())]
    pub registered_verifier_pda: UncheckedAccount<'info>, //Account<'info, RegisteredVerifier>,
    pub verifier_program: Program<'info, LightPsp4in4outAppStorage>,
    /// CHECK:` It get checked inside the event_call
    pub log_wrapper: UncheckedAccount<'info>,
    #[account(mut)]
    pub event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
    #[account(mut)]
    pub compressed_account_merkle_tree: AccountLoader<'info, CompressedAccountMerkleTree>,
}

/// Initializes a compression merkle tree
#[derive(Accounts)]
#[instruction(tree_index: u64)]
pub struct InitCompressionMerkleTree<'info> {
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(init, seeds = [&tree_index.to_le_bytes(), COMPRESSION_MERKLE_TREE_SEED.as_slice()], bump, space= (TREE_HEIGHT + ROOT_HISTORY_SIZE) * 32 + 32 + 8 + 8 + 8 /*... + sub_tree_hash + current_root_index + discriminator */, payer = signing_address )]
    pub compressed_account_merkle_tree: AccountLoader<'info, CompressedAccountMerkleTree>,
    pub system_program: Program<'info, System>,
}

pub const TREE_HEIGHT: usize = 18;
pub const ROOT_HISTORY_SIZE: usize = 20;
#[constant]
pub const COMPRESSION_MERKLE_TREE_SEED: &[u8; 23] = b"compression_merkle_tree";
pub const ZERO_VALUES_SUB_TREE_HASH: [u8; 32] = [
    21, 144, 175, 251, 238, 246, 252, 33, 134, 94, 11, 37, 245, 29, 195, 217, 89, 38, 172, 126,
    144, 40, 205, 176, 27, 204, 217, 5, 184, 104, 29, 12,
];

#[derive(Debug, Default)]
#[account(zero_copy)]
pub struct CompressedAccountMerkleTree {
    pub filled_sub_trees: [[u8; 32]; TREE_HEIGHT],
    pub sub_tree_hash: [u8; 32],
    pub root_history: [[u8; 32]; ROOT_HISTORY_SIZE],
    pub current_root_index: u64,
    pub next_leaf_index: u64,
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
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Account<'info, VerifierState10Ins<NR_CHECKED_INPUTS, TransactionsConfig>>,
}

#[derive(Accounts)]
pub struct ProveInclusionInstruction<'info> {
    #[account(mut)]
    pub compressed_account_merkle_tree: AccountLoader<'info, CompressedAccountMerkleTree>,
    pub signer: Signer<'info>,
}
