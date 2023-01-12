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

use crate::processor::process_shielded_transfer;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use merkle_tree_program::{
    initialize_new_merkle_tree_18::PreInsertedLeavesIndex,
    poseidon_merkle_tree::state::MerkleTree, RegisteredVerifier,
};
use merkle_tree_program::{program::MerkleTreeProgram};


declare_id!("GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8");

#[program]
pub mod verifier_program_two {
    use super::*;

    /// This instruction is used to invoke this system verifier and can only be invoked via cpi.
    pub fn shielded_transfer_inputs<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstruction<'info>>,
        proof: Vec<u8>,
        // merkle_root: Vec<u8>,
        // amount: Vec<u8>,
        // ext_data_hash: Vec<u8>,
        // nullifiers: [[u8; 32]; 4],
        // leaves: [[u8; 32]; 4],
        // fee_amount: Vec<u8>,
        // mint_pubkey: Vec<u8>,
        // root_index: u64,
        // relayer_fee: u64,
        // encrypted_utxos: Vec<u8>,
        // kyc_root: Vec<u8>,
        app_hash: Vec<u8>,
    ) -> Result<()> {
        msg!("shielded_transfer_inputs");
        process_shielded_transfer(
            ctx,
            proof,
            app_hash,
        )?;
        //     vec![kyc_root, app_hash, ctx.accounts.invoking_verifier.pubkey.to_bytes().to_vec()],
        //     ctx,
        //     proof,
        //     merkle_root,
        //     amount, //[vec![0u8;24], amount.to_vec()].concat().try_into().unwrap(),
        //     ext_data_hash,
        //     vec![nullifiers[0].to_vec(),nullifiers[1].to_vec(),nullifiers[2].to_vec(),nullifiers[3].to_vec()],
        //     vec![vec![leaves[0].to_vec(),leaves[1].to_vec()],vec![leaves[2].to_vec(),leaves[3].to_vec()]],
        //     0,          //ext_amount,
        //     fee_amount, //[vec![0u8;24], fee_amount.to_vec()].concat().try_into().unwrap(),
        //     mint_pubkey,
        //     encrypted_utxos,
        //     root_index,
        //     relayer_fee,
        // )
        Ok(())
    }
}

#[derive(Accounts)]
pub struct LightInstruction<'info> {
    /// CHECK: Cannot be checked with Account because it assumes this program to be the owner
    #[account(mut)]
    pub verifier_state: UncheckedAccount<'info>,
    /// First time therefore the signing address is not checked but saved to be checked in future instructions.
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    /// CHECK: Is the same as in integrity hash.
    pub system_program: Program<'info, System>,
    /// CHECK: Is the same as in integrity hash.
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    /// CHECK: Is the same as in integrity hash.
    // #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[usize::try_from(self.load()?.merkle_tree_index).unwrap()].0))]
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, MerkleTree>,
    // #[account(
    //     mut,
    //     address = anchor_lang::prelude::Pubkey::find_program_address(&[merkle_tree.key().to_bytes().as_ref()], &MerkleTreeProgram::id()).0
    // )]
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    // #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump)]
    #[account(mut)]
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
    /// Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    // #[account(seeds= [program_id.key().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
    // #[account(seeds= [invoking_verifier.to_account_info().owner.key().to_bytes().as_ref()], bump, seeds::program=invoking_verifier.to_account_info().owner)]
    /// CHECK: Signer check to acertain the invoking program ID to be used as a public input.
    pub invoking_verifier: UncheckedAccount<'info>,//Account<'info, RegisteredVerifier>,
}
