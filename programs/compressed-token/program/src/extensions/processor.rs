use anchor_lang::prelude::ProgramError;
use borsh::BorshDeserialize;
use light_compressed_account::{
    compressed_account::ZCompressedAccountDataMut,
    instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut,
};

use crate::{
    extensions::{
        metadata_pointer::InitializeMetadataPointerInstructionData,
        token_metadata::{TokenMetadata, TOKEN_METADATA_DISCRIMINATOR},
        ExtensionType,
    },
    mint::instructions::ZExtensionInstructionData,
};

// Applying extension(s) to compressed accounts.
pub fn process_create_extensions<'a>(
    extensions: &[ZExtensionInstructionData],
    cpi_data: &mut ZInstructionDataInvokeCpiWithReadOnlyMut<'a>,
    mint_data_len: usize,
) -> Result<(), ProgramError> {
    for extension in extensions {
        // match ExtensionType::try_from(extension.extension_type).unwrap() {
        //     ExtensionType::MetadataPointer => {
        //         // deserialize metadata pointer ix data
        //         let has_address = create_metadata_pointer(extension.data, cpi_data, mint_data_len)?;
        //         // only go ahed if has address, probably duplicate
        //         if has_address.1 {
        //             create_token_metadata_account(
        //                 extension.data,
        //                 cpi_data.output_compressed_accounts[0]
        //                     .compressed_account
        //                     .data
        //                     .as_mut()
        //                     .unwrap(),
        //             )?;
        //         }
        //     }
        //     _ => return Err(ProgramError::InvalidInstructionData),
        // }
    }
    Ok(())
}

// TODO: do compatibility token 22 deserialization for all accounts.
// TODO: fix
fn create_metadata_pointer<'a>(
    instruction_data: &[u8],
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut<'a>,
    mint_data_len: usize,
) -> Result<([u8; 32], bool), ProgramError> {
    use light_zero_copy::borsh::Deserialize;
    // 1. Deserialize the metadata pointer instruction data
    let (metadata_pointer_data, _) =
        InitializeMetadataPointerInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
    if let Some(metadata_address_params) = metadata_pointer_data.metadata_address_params.as_ref() {
        **cpi_instruction_struct.output_compressed_accounts[1]
            .compressed_account
            .address
            .as_mut()
            .unwrap() = metadata_address_params.address;

        cpi_instruction_struct.new_address_params[1].seed = metadata_address_params.seed;
        cpi_instruction_struct.new_address_params[1].address_merkle_tree_root_index =
            metadata_address_params.address_merkle_tree_root_index;
        cpi_instruction_struct.new_address_params[1].assigned_account_index = 1;
        // Note we can skip address derivation since we are assigning it to the account in index 0.
        cpi_instruction_struct.new_address_params[1].assigned_to_account = 1;
        cpi_instruction_struct.new_address_params[1].address_merkle_tree_account_index =
            metadata_address_params.address_merkle_tree_account_index;
    }

    let cpi_data = cpi_instruction_struct.output_compressed_accounts[1]
        .compressed_account
        .data
        .as_mut()
        .ok_or(ProgramError::InvalidInstructionData)?;

    if metadata_pointer_data.authority.is_none()
        && metadata_pointer_data.metadata_address_params.is_none()
    {
        return Err(anchor_lang::prelude::ProgramError::InvalidInstructionData);
    }
    let start_offset = mint_data_len;
    let mut end_offset = start_offset;
    if metadata_pointer_data.authority.is_some() {
        end_offset += 33;
    } else {
        end_offset += 1;
    }
    let hash_address = metadata_pointer_data.metadata_address_params.is_some();
    if metadata_pointer_data.metadata_address_params.is_some() {
        end_offset += 33;
    } else {
        end_offset += 1;
    }
    // TODO: double test this is risky but should be ok
    // The layout is also Option<[u8;32]>, Option<[u8;32], ..> but we cut off after 32 bytes.
    cpi_data.data[start_offset..end_offset].copy_from_slice(&instruction_data);

    Ok(([0u8; 32], hash_address))
}

// Could be ok
fn create_token_metadata_account<'a>(
    mut instruction_data: &[u8],
    cpi_data: &mut ZCompressedAccountDataMut<'a>,
) -> Result<(), ProgramError> {
    // TODO: use zero copy (need to add string support or manual impl)
    let token_metadata = TokenMetadata::deserialize(&mut instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let hash = TokenMetadata::hash(&token_metadata)?;
    *cpi_data.data_hash = hash;
    cpi_data.discriminator = TOKEN_METADATA_DISCRIMINATOR;
    (*cpi_data.data).copy_from_slice(instruction_data);
    Ok(())
}
