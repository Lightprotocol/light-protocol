use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    instruction_data::{
        data::ZOutputCompressedAccountWithPackedContextMut, invoke_cpi::InstructionDataInvokeCpi,
    },
    Pubkey,
};

use light_zero_copy::ZeroCopyNew;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    mint::{
        instructions::ZCreateCompressedMintInstructionData,
        state::{CompressedMint, CompressedMintConfig},
    },
};

pub fn create_output_compressed_mint_account(
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut,
    mint_pda: Pubkey,
    parsed_instruction_data: ZCreateCompressedMintInstructionData,
    program_id: &Pubkey,
    mint_config: CompressedMintConfig,
    compressed_account_address: [u8; 32],
) -> Result<(), ProgramError> {
    // 3. Create output compressed account
    {
        // TODO: create helper to assign output_compressed_account
        output_compressed_account.compressed_account.owner = *program_id;

        if let Some(address) = output_compressed_account
            .compressed_account
            .address
            .as_deref_mut()
        {
            *address = compressed_account_address;
        } else {
            panic!("Compressed account address is required");
        }
        *output_compressed_account.merkle_tree_index = 1;
    }
    // 4. Create CompressedMint account data & compute hash
    {
        // TODO: create helper to assign compressed account data
        let compressed_account_data = output_compressed_account
            .compressed_account
            .data
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        compressed_account_data.discriminator = COMPRESSED_MINT_DISCRIMINATOR;
        let (mut compressed_mint, _) =
            CompressedMint::new_zero_copy(compressed_account_data.data, mint_config)
                .map_err(ProgramError::from)?;
        compressed_mint.spl_mint = mint_pda;
        compressed_mint.decimals = parsed_instruction_data.decimals;
        if let Some(z_freeze_authority) = compressed_mint.freeze_authority.as_deref_mut() {
            *z_freeze_authority = *(parsed_instruction_data
                .freeze_authority
                .as_deref()
                .ok_or(ProgramError::InvalidAccountData)?);
        }
        if let Some(z_mint_authority) = compressed_mint.mint_authority.as_deref_mut() {
            *z_mint_authority = parsed_instruction_data.mint_authority;
        }

        *compressed_account_data.data_hash = compressed_mint
            .hash()
            .map_err(|_| ProgramError::InvalidAccountData)?;
    }

    Ok(())
}
