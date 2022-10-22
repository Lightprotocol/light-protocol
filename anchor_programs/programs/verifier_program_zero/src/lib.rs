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

use crate::processor::process_shielded_transfer_2_inputs;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_verifier_sdk::utils::create_pda::create_and_check_pda;
use merkle_tree_program::{
    errors::ErrorCode as MerkleTreeError, initialize_new_merkle_tree_18::PreInsertedLeavesIndex,
    poseidon_merkle_tree::state::MerkleTree, RegisteredVerifier,
};
use merkle_tree_program::{program::MerkleTreeProgram, MerkleTreeAuthority};

declare_id!("J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i");

#[program]
pub mod verifier_program_zero {
    use super::*;

    /// Initializes the authority which is used to cpi the Merkle tree.
    /// can only be invoked by Merkle tree authority.
    pub fn initialize_authority(ctx: Context<InitializeAuthority>) -> anchor_lang::Result<()> {
        let rent = &Rent::from_account_info(&ctx.accounts.rent.to_account_info())?;

        create_and_check_pda(
            &ctx.program_id,
            &ctx.accounts.signing_address.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &rent,
            MerkleTreeProgram::id().to_bytes().as_ref(),
            &Vec::new(),
            0,    //bytes
            0,    //lamports
            true, //rent_exempt
        )?;

        Ok(())
    }

    /// This instruction is the first step of a shieled transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2
    pub fn shielded_transfer_inputs<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstruction<'info>>,
        proof: [u8; 256],
        merkle_root: [u8; 32],
        amount: [u8; 32],
        ext_data_hash: [u8; 32],
        nullifiers: [[u8; 32]; 2],
        leaves: [[u8; 32]; 2],
        fee_amount: [u8; 32],
        mint_pubkey: [u8; 32],
        root_index: u64,
        relayer_fee: u64,
        encrypted_utxos: Vec<u8>,
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
            0,          //ext_amount,
            fee_amount, //[vec![0u8;24], fee_amount.to_vec()].concat().try_into().unwrap(),
            mint_pubkey,
            encrypted_utxos,
            root_index,
            relayer_fee,
        )
    }
}

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
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
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
    /// Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(seeds= [program_id.key().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
}
