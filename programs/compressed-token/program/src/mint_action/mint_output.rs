use std::ptr::slice_from_raw_parts_mut;

use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::mint_action::ZMintActionCompressedInstructionData,
    state::{CompressedMint, CompressedMintConfig},
};
use light_hasher::sha256::Sha256BE;
use light_profiler::profile;
use light_zero_copy::ZeroCopyNew;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    extensions::processor::extensions_state_in_output_compressed_account,
    mint_action::{
        accounts::{AccountsConfig, MintActionAccounts},
        processor::process_actions,
        queue_indices::QueueIndices,
    },
};

#[profile]
pub fn process_output_compressed_account<'a>(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
    validated_accounts: &MintActionAccounts,
    accounts_config: &AccountsConfig,
    output_compressed_accounts: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
    mint_size_config: CompressedMintConfig,
    hash_cache: &mut HashCache,
    queue_indices: &QueueIndices,
) -> Result<(), ProgramError> {
    let (mint_account, token_accounts): (
        &mut ZOutputCompressedAccountWithPackedContextMut<'_>,
        &mut [ZOutputCompressedAccountWithPackedContextMut<'_>],
    ) = if output_compressed_accounts.len() == 1 {
        (&mut output_compressed_accounts[0], &mut [])
    } else {
        let (mint_account, token_accounts) = output_compressed_accounts.split_at_mut(1);
        (&mut mint_account[0], token_accounts)
    };

    mint_account.set(
        crate::LIGHT_CPI_SIGNER.program_id.into(),
        0,
        Some(parsed_instruction_data.compressed_address),
        queue_indices.output_queue_index,
        COMPRESSED_MINT_DISCRIMINATOR,
        [0u8; 32],
    )?;

    let compressed_account_data = mint_account
        .compressed_account
        .data
        .as_mut()
        .ok_or(ErrorCode::MintActionOutputSerializationFailed)?;
    let data = unsafe {
        &mut *slice_from_raw_parts_mut(
            compressed_account_data.data.as_mut_ptr(),
            compressed_account_data.data.len(),
        )
    };

    {
        let (mut compressed_mint, _) = CompressedMint::new_zero_copy(data, mint_size_config)
            .map_err(|_| ErrorCode::MintActionOutputSerializationFailed)?;

        {
            compressed_mint.set(
                &parsed_instruction_data.mint,
                // Instruction data is used for the input compressed account.
                // We need to use this value to cover the case that we decompress the mint in this instruction.
                accounts_config.is_decompressed,
            )?;

            if let Some(extensions) = parsed_instruction_data.mint.extensions.as_deref() {
                let z_extensions = compressed_mint
                    .extensions
                    .as_mut()
                    .ok_or(ProgramError::AccountAlreadyInitialized)?;
                extensions_state_in_output_compressed_account(
                    extensions,
                    z_extensions.as_mut_slice(),
                    parsed_instruction_data.mint.metadata.spl_mint,
                )?;
            }
        }

        process_actions(
            parsed_instruction_data,
            validated_accounts,
            accounts_config,
            token_accounts,
            hash_cache,
            queue_indices,
            &validated_accounts.packed_accounts,
            &mut compressed_mint,
        )?;
        use light_hasher::Hasher;
        *compressed_account_data.data_hash = Sha256BE::hash(compressed_account_data.data)?;
    }
    Ok(())
}
