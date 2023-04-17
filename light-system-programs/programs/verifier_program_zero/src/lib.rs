#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_protocol_verifier_program_zero",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-onchain/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-onchain"
}

pub mod processor;
pub mod verifying_key;
pub use processor::*;

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use merkle_tree_program::{
    program::MerkleTreeProgram, transaction_merkle_tree::state::TransactionMerkleTree,
    utils::constants::TOKEN_AUTHORITY_SEED, RegisteredVerifier,
};

declare_id!("J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i");

#[program]
pub mod verifier_program_zero {
    use super::*;

    /// This instruction is the first step of a shieled transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2
    pub fn shielded_transfer_inputs<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstruction<'info>>,
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
        public_amount_spl: [u8; 32],
        nullifiers: [[u8; 32]; 2],
        leaves: [[u8; 32]; 2],
        public_amount_sol: [u8; 32],
        root_index: u64,
        relayer_fee: u64,
        encrypted_utxos: Vec<u8>,
    ) -> Result<()> {
        let len_missing_bytes = 256 - encrypted_utxos.len();
        let mut enc_utxos = encrypted_utxos;
        enc_utxos.append(&mut vec![0u8; len_missing_bytes]);
        process_shielded_transfer_2_in_2_out(
            ctx,
            &proof_a,
            &proof_b,
            &proof_c,
            &public_amount_spl,
            &nullifiers,
            &[leaves; 1],
            &public_amount_sol,
            &enc_utxos,
            root_index,
            relayer_fee,
            &Vec::<Vec<u8>>::new(), // checked_public_inputs
            &[0u8; 32],             //pool_type
        )
    }
}

#[derive(Accounts)]
pub struct LightInstruction<'info> {
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump)]
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
    /// CHECK:` Is checked when it is used during spl withdrawals.
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program= MerkleTreeProgram::id())]
    pub token_authority: AccountInfo<'info>,
    /// Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(mut, seeds= [program_id.key().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
}
#[allow(non_camel_case_types)]
// helper struct to create anchor idl with u256 type
#[account]
pub struct u256 {
    x: [u8; 32],
}

#[account]
pub struct Utxo {
    amounts: [u64; 2],
    spl_asset_index: u64,
    blinding: u256,
    account_shielded_public_key: u256,
    account_encryption_public_key: [u8; 32],
    verifier_address: Pubkey,
}

#[account]
pub struct TransactionParameters {
    input_utxos_bytes: Vec<Vec<u8>>,
    // outputUtxos should be checked
    // TODO: write function which checks and displays how much multisig funds are spent, to whom, etc
    output_utxos_bytes: Vec<Vec<u8>>,
    // integrityHashInputs
    recipient_spl: Pubkey,
    recipient_sol: Pubkey,
    relayer_pubkey: Pubkey,
    relayer_fee: u64,
    // for determinitic encryption, nonces are derived from commitment hashes thus no need to save separately
    transaction_index: u64,
}

#[allow(non_camel_case_types)]
#[constant]
pub const programId: &str = "J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i";
