#![allow(clippy::too_many_arguments)]
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use psp_compressed_pda::utils::CompressedProof;
use psp_compressed_token::InputTokenDataWithContext;
use psp_compressed_token::TokenTransferOutputData;
pub mod create_pda;
pub use create_pda::*;
pub mod sdk;
use psp_compressed_pda::compressed_cpi::CompressedCpiContext;
use psp_compressed_pda::NewAddressParamsPacked;

#[error_code]
pub enum EscrowError {
    #[msg("Escrow is locked")]
    EscrowLocked,
}

declare_id!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[program]
pub mod program_owned_account_test {

    use super::*;

    /// Escrows compressed tokens, for a certain number of slots.
    /// Transfers compressed tokens to compressed token account owned by cpi_signer.
    /// Tokens are locked for lock_up_time slots.
    pub fn create_compressed_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
        data: [u8; 31],
        proof: Option<CompressedProof>,
        root_indices: Vec<u16>,
        output_merkle_tree_account_indices: Vec<u8>,
        new_address_parameters: NewAddressParamsPacked,
        cpi_context: CompressedCpiContext,
        owner_program: Pubkey,
    ) -> Result<()> {
        process_create_pda(
            ctx,
            data,
            proof,
            root_indices,
            output_merkle_tree_account_indices,
            new_address_parameters,
            owner_program,
            cpi_context,
        )
    }
}
