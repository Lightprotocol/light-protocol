use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::mint_action::ZMintActionCompressedInstructionData,
    state::{CompressedMint, CompressedMintConfig},
};
use light_zero_copy::ZeroCopyNew;
use pinocchio::msg;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    extensions::processor::extensions_state_in_output_compressed_account,
    mint_action::{
        accounts::{AccountsConfig, MintActionAccounts},
        processor::process_actions,
        queue_indices::QueueIndices,
    },
};

pub fn process_output_compressed_account<'a>(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
    validated_accounts: &MintActionAccounts,
    accounts_config: &AccountsConfig,
    output_compressed_accounts: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
    mint_size_config: CompressedMintConfig,
    hash_cache: &mut HashCache,
    queue_indices: &QueueIndices,
) -> Result<(), ProgramError> {
    msg!("process_output_compressed_account: ENTRY");
    let (mint_account, token_accounts): (
        &mut ZOutputCompressedAccountWithPackedContextMut<'_>,
        &mut [ZOutputCompressedAccountWithPackedContextMut<'_>],
    ) = if output_compressed_accounts.len() == 1 {
        (&mut output_compressed_accounts[0], &mut [])
    } else {
        let (mint_account, token_accounts) = output_compressed_accounts.split_at_mut(1);
        (&mut mint_account[0], token_accounts)
    };

    msg!("About to call mint_account.set");
    mint_account.set(
        crate::LIGHT_CPI_SIGNER.program_id.into(),
        0,
        Some(parsed_instruction_data.compressed_address),
        queue_indices.output_queue_index,
        COMPRESSED_MINT_DISCRIMINATOR,
        [0u8; 32],
    )?;
    msg!("mint_account.set completed");

    msg!("About to get compressed_account_data");
    let compressed_account_data = mint_account
        .compressed_account
        .data
        .as_mut()
        .ok_or(ErrorCode::MintActionOutputSerializationFailed)?;
    msg!(
        "compressed_account_data obtained, data len: {}",
        compressed_account_data.data.len()
    );

    msg!("About to create CompressedMint::new_zero_copy with mint_size_config");
    let (mut compressed_mint, _) =
        CompressedMint::new_zero_copy(compressed_account_data.data, mint_size_config)
            .map_err(|_| ErrorCode::MintActionOutputSerializationFailed)?;
    msg!("CompressedMint::new_zero_copy completed successfully");
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
                parsed_instruction_data.mint.spl_mint,
            )?;
        }
    }
    msg!(
        "About to call process_actions with {} actions",
        parsed_instruction_data.actions.len()
    );
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
    *compressed_account_data.data_hash = compressed_mint.hash(hash_cache)?;
    Ok(())
}
