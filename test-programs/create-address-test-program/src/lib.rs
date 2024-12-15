#![allow(clippy::too_many_arguments)]

use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_system_program::invoke::processor::CompressedProof;
pub mod create_pda;
pub use create_pda::*;
use light_system_program::NewAddressParamsPacked;

declare_id!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

#[program]

pub mod system_cpi_test {

    use super::*;

    pub fn create_compressed_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
        data: [u8; 31],
        proof: Option<CompressedProof>,
        new_address_parameters: NewAddressParamsPacked,
        bump: u8,
    ) -> Result<()> {
        process_create_pda(ctx, data, proof, new_address_parameters, bump)
    }
}
