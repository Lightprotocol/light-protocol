use crate::processor::TransactionsConfig;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_verifier_sdk::{light_transaction::VERIFIER_STATE_SEED, state::VerifierState10Ins};
use merkle_tree_program::transaction_merkle_tree::state::TransactionMerkleTree;
use merkle_tree_program::utils::constants::TOKEN_AUTHORITY_SEED;
use merkle_tree_program::{program::MerkleTreeProgram, EventMerkleTree};
use verifier_program_two::{self, program::VerifierProgramTwo};

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
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump, seeds::program= VerifierProgramTwo::id())]
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
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program= MerkleTreeProgram::id())]
    pub token_authority: UncheckedAccount<'info>,
    /// CHECK: Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(mut, seeds= [VerifierProgramTwo::id().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    pub registered_verifier_pda: UncheckedAccount<'info>, //Account<'info, RegisteredVerifier>,
    pub verifier_program: Program<'info, VerifierProgramTwo>,
    /// CHECK:` It get checked inside the event_call
    pub log_wrapper: UncheckedAccount<'info>,
    #[account(mut)]
    pub event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
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

// /// Multisig parameters which are not part of the zero-knowledge proof.
// #[account]
// pub struct MultiSigParameters {
//     pub encryptionPubkeysSigners: [[u8; 32]; 7],
// }

// not necessary because will make this a utils function of the storage verifier
/// encrypted multisig parameters
/// space = 8 (discriminator) + 7 * 32 + 32 + 458 = 722
/// nonces are Sha3(base_nonce||counter), aes256 iv: counter = 8
// #[account]
// pub struct EncryptedMultiSigParameters {
//     // length [48; 7]
//     encryptedAesSecretKey: Vec<Vec<u8>>,
//     base_nonce: [u8; 32],
//     aesCipherText: [u8; 512],
// }

// #[derive(Default, Copy)]
// #[account]
// pub struct Utxo {
//     publicKey: [u8; 32],
//     blinding: [u8; 31],
//     appData: UtxoAppData,
// }

// TODO: check how the solana transaction sig hash is calculated
// it should hash the complete transaction then we can omit any auth encryption
// storage verifier client
// - encryptToNaclBox({recipientsPubkeys[], msg, aesSecret})
// - encryptAes(aesSecret, message)
// - store(msg)
// - getNaclBoxEncrypted()
// - getAesEncrypted()
// - getAllMessages()

#[account]
pub struct CreateMultiSig {
    pub seed: [u8; 32],
    pub public_key_x: [[u8; 32]; 7],
    pub public_key_y: [[u8; 32]; 7],
    pub threshold: u8,
    pub nr_signers: u8,
    pub signers_encryption_public_keys: [[u8; 32]; 7],
    // if update point to the multisig which is being updated
    pub prior_multi_sig_slot: u64,
    pub prior_multi_sig_hash: [u8; 32],
    pub prior_multi_sig_seed: [u8; 32],
}

#[account]
pub struct ApproveTransaction {
    signer_index: u8,
    signature: [u8; 64],
}
