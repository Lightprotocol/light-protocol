use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use borsh::BorshSerialize;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;
use light_ctoken_types::{
    hash_cache::HashCache, instructions::mint_action::ZMintActionCompressedInstructionData,
    state::CompressedMint,
};
use light_hasher::{sha256::Sha256BE, Hasher};
use light_program_profiler::profile;
use spl_pod::solana_msg::msg;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    mint_action::{
        accounts::MintActionAccounts, actions::process_actions, queue_indices::QueueIndices,
    },
};

#[profile]
pub fn process_output_compressed_account<'a>(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
    validated_accounts: &MintActionAccounts,
    output_compressed_accounts: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
    hash_cache: &mut HashCache,
    queue_indices: &QueueIndices,
    mut compressed_mint: CompressedMint,
) -> Result<(), ProgramError> {
    let (mint_account, token_accounts) = split_mint_and_token_accounts(output_compressed_accounts);

    process_actions(
        parsed_instruction_data,
        validated_accounts,
        &mut token_accounts.iter_mut(),
        hash_cache,
        queue_indices,
        &validated_accounts.packed_accounts,
        &mut compressed_mint,
    )?;

    let data_hash = {
        let compressed_account_data = mint_account
            .compressed_account
            .data
            .as_mut()
            .ok_or(ErrorCode::MintActionOutputSerializationFailed)?;

        let data = compressed_mint
            .try_to_vec()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
        if data.len() != compressed_account_data.data.len() {
            msg!("Data allocation for output mint account is wrong");
            return Err(ProgramError::InvalidAccountData);
        }
        compressed_account_data
            .data
            .copy_from_slice(data.as_slice());
        Sha256BE::hash(compressed_account_data.data)?
    };

    // Set mint output compressed account fields except the data.
    mint_account.set(
        crate::LIGHT_CPI_SIGNER.program_id.into(),
        0,
        Some(parsed_instruction_data.compressed_address),
        queue_indices.output_queue_index,
        COMPRESSED_MINT_DISCRIMINATOR,
        data_hash,
    )?;
    Ok(())
}

#[inline(always)]
fn split_mint_and_token_accounts<'a>(
    output_compressed_accounts: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
) -> (
    &'a mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
) {
    if output_compressed_accounts.len() == 1 {
        (&mut output_compressed_accounts[0], &mut [])
    } else {
        let (mint_account, token_accounts) = output_compressed_accounts.split_at_mut(1);
        (&mut mint_account[0], token_accounts)
    }
}
