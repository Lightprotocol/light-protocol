/*
use solana_security_txt::security_txt;

security_txt! {
    name: "light_protocol_verifier_program",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-program/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-program/program_merkle_tree"
}
*/

pub mod processor;
pub mod verifying_key;
pub use processor::*;

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use merkle_tree_program::{
    initialize_new_merkle_tree_18::PreInsertedLeavesIndex,
    poseidon_merkle_tree::state::MerkleTree,
    program::MerkleTreeProgram,
    utils::constants::TOKEN_AUTHORITY_SEED,
    RegisteredVerifier,
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
        proof: Vec<u8>,
        amount: Vec<u8>,
        nullifiers: [[u8; 32]; 2],
        leaves: [[u8; 32]; 2],
        fee_amount: Vec<u8>,
        root_index: u64,
        relayer_fee: u64,
        encrypted_utxos: Vec<u8>,
    ) -> Result<()> {
        process_shielded_transfer_2_in_2_out(
            ctx,
            proof.to_vec(),
            amount.to_vec(),
            vec![nullifiers[0].to_vec(), nullifiers[1].to_vec()],
            vec![vec![leaves[0].to_vec(), leaves[1].to_vec()]],
            fee_amount.to_vec(),
            [
                encrypted_utxos.to_vec(),
                vec![0u8; 256 - encrypted_utxos.len()],
            ]
            .concat(),
            root_index,
            relayer_fee,
            Vec::<Vec<u8>>::new(), // checked_public_inputs
            vec![0u8; 32],         //pool_type
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
    pub merkle_tree: AccountLoader<'info, MerkleTree>,
    #[account(mut)]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender_fee: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient_fee: UncheckedAccount<'info>,
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut)]
    pub relayer_recipient: UncheckedAccount<'info>,
    /// CHECK:` Is checked when it is used during sol deposits.
    #[account(mut)]
    pub escrow: UncheckedAccount<'info>,
    /// CHECK:` Is checked when it is used during spl withdrawals.
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program= MerkleTreeProgram::id())]
    pub token_authority: AccountInfo<'info>,
    /// Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(mut, seeds= [program_id.key().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
}
