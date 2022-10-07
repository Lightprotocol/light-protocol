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

pub mod errors;
pub mod verification_key;
pub mod processor;

pub use processor::*;
pub use errors::*;

use anchor_lang::prelude::*;
use merkle_tree_program::program::MerkleTreeProgram;
use anchor_spl::token::Token;
use merkle_tree_program::{
    initialize_new_merkle_tree_18::PreInsertedLeavesIndex,
    RegisteredVerifier,
    poseidon_merkle_tree::state::MerkleTree,

};
use crate::processor::process_shielded_transfer_2_inputs;

declare_id!("J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i");

#[program]
pub mod verifier_program {
    use super::*;

    /// This instruction is the first step of a shieled transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2
    pub fn shielded_transfer_inputs<'a, 'b, 'c, 'info> (
        ctx: Context<'a, 'b, 'c, 'info, LightInstruction<'info>>,
        proof: [u8; 256],
        merkle_root: [u8; 32],
        amount: [u8; 32],
        ext_data_hash: [u8; 32],
        nullifiers: [[u8; 32]; 2],
        leaves: [[u8; 32]; 2],
        fee_amount: [u8; 32],
        mint_pubkey: [u8;32],
        merkle_tree_index: u64,
        _root_index: u64,
        relayer_fee: u64,
        encrypted_utxos0: [u8; 128],
        encrypted_utxos1: [u8; 64],
        encrypted_utxos2: [u8; 32],
        encrypted_utxos3: [u8; 14],
    ) -> Result<()> {
        process_shielded_transfer_2_inputs(
            ctx,
            proof,
            merkle_root,
            amount, //[vec![0u8;24], amount.to_vec()].concat().try_into().unwrap(),
            ext_data_hash,
            nullifiers[0],
            nullifiers[1],
            leaves[0],
            leaves[1],
            0,//ext_amount,
            fee_amount, //[vec![0u8;24], fee_amount.to_vec()].concat().try_into().unwrap(),
            mint_pubkey,
            [encrypted_utxos0.to_vec(), encrypted_utxos1.to_vec(), encrypted_utxos2.to_vec(), encrypted_utxos3.to_vec()].concat(),
            merkle_tree_index,
            relayer_fee
        )
    }

}

#[derive( Accounts)]
#[instruction(
    proof:              [u8;256],
    merkle_root:        [u8;32],
    amount:             [u8;32],
    tx_integrity_hash:  [u8;32]
)]
pub struct LightInstruction<'info> {
    // #[account(init_if_needed, seeds = [tx_integrity_hash.as_ref(), b"storage"], bump,  payer=signing_address, space= 5 * 1024)]
    // pub verifier_state: AccountLoader<'info, VerifierState>,
    /// First time therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: Is the same as in integrity hash.
    // #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[usize::try_from(self.load()?.merkle_tree_index).unwrap()].0))]
    pub merkle_tree: AccountLoader<'info, MerkleTree>,
    #[account(
        mut,
        address = anchor_lang::prelude::Pubkey::find_program_address(&[merkle_tree.key().to_bytes().as_ref()], &MerkleTreeProgram::id()).0
    )]
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
    pub relayer_recipient: AccountInfo<'info>,
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut)]
    pub escrow: AccountInfo<'info>,
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut)]
    pub token_authority: AccountInfo<'info>,
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>
}
