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
use merkle_tree_program::{
    program::MerkleTreeProgram,
    MerkleTreeAuthority,
};
use anchor_spl::token::Token;
use merkle_tree_program::{
    initialize_new_merkle_tree_18::PreInsertedLeavesIndex,
    RegisteredVerifier,
    poseidon_merkle_tree::state::MerkleTree,
    errors::ErrorCode as MerkleTreeError

};
use crate::processor::process_shielded_transfer_first;
use merkle_tree_program::utils::create_pda::create_and_check_pda;

declare_id!("J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i");

#[program]
pub mod verifier_program {
    use super::*;

    /// Initializes the authority which is used to cpi the Merkle tree.
    /// can only be invoked by Merkle tree authority.
    pub fn initialize_authority(
        ctx: Context<InitializeAuthority>
    ) -> anchor_lang::Result<()> {
        let rent = &Rent::from_account_info(&ctx.accounts.rent.to_account_info())?;

        create_and_check_pda(
            &ctx.program_id,
            &ctx.accounts.signing_address.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &rent,
            MerkleTreeProgram::id().to_bytes().as_ref(),
            &Vec::new(),
            0,                  //bytes
            0, //lamports
            true,               //rent_exempt
        )?;

        Ok(())
    }

    /// This instruction is the first step of a shielded transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification.
    pub fn shielded_transfer_first<'a, 'b, 'c, 'info> (
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionFirst<'info>>,
        proof: [u8; 256],
        merkle_root: [u8; 32],
        amount: [u8; 32],
        ext_data_hash: [u8; 32],
        nullifiers: [[u8; 32]; 10], // 10 nullifiers 1072 byts 16 1264 bytes total data sent
        leaves: [[u8; 32]; 2],
        fee_amount: [u8; 32],
        mint_pubkey: [u8;32],
        root_index: u64,
        relayer_fee: u64,
        encrypted_utxos0: [u8; 128],
        encrypted_utxos1: [u8; 128]
    ) -> Result<()> {
        process_shielded_transfer_first(
            ctx,
            proof,
            merkle_root,
            amount, //[vec![0u8;24], amount.to_vec()].concat().try_into().unwrap(),
            ext_data_hash,
            nullifiers,
            leaves,
            fee_amount, //[vec![0u8;24], fee_amount.to_vec()].concat().try_into().unwrap(),
            mint_pubkey,
            encrypted_utxos0.to_vec(),
            encrypted_utxos1.to_vec(),
            root_index,
            relayer_fee
        )

    }

    /// This instruction is the second step of a shieled transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2
    pub fn shielded_transfer_second<'a, 'b, 'c, 'info> (
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionSecond<'info>>,
    ) -> Result<()> {
        process_shielded_transfer_second(
            ctx
        )
    }

}
use crate::processor::LightTx;
use light_verifier_sdk::state::VerifierStateTenNF;

#[derive(Accounts)]
pub struct InitializeAuthority<'info> {
    /// CHECK:` Signer is merkle tree authority.
    #[account(mut, address=merkle_tree_authority_pda.pubkey @MerkleTreeError::InvalidAuthority)]
    pub signing_address: Signer<'info>,
    #[account(seeds = [&b"MERKLE_TREE_AUTHORITY"[..]], bump, seeds::program=MerkleTreeProgram::id())]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    /// CHECK:` Is checked here, but inited with 0 bytes.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>
}


/// Send data and verifies proof.
#[derive( Accounts)]
pub struct LightInstructionFirst<'info> {
    /// First time therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(init, seeds = [b"VERIFIER_STATE"], bump, space= 8 + 776, payer = signing_address )]
    pub verifier_state: Account<'info, VerifierStateTenNF::<LightTx>>
}

/// Executes light transaction with state created in the first instruction.
#[derive( Accounts)]
pub struct LightInstructionSecond<'info> {
    #[account(mut, seeds = [b"VERIFIER_STATE"], bump, close=signing_address )]
    pub verifier_state: Account<'info, VerifierStateTenNF::<LightTx>>,
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
    /// Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(seeds= [program_id.key().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>
}
