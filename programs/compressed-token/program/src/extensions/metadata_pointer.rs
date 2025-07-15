use anchor_lang::prelude::ProgramError;
use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    instruction_data::data::ZOutputCompressedAccountWithPackedContextMut, Pubkey,
};
use light_ctoken_types::extensions::metadata_pointer::ZInitMetadataPointer;
use light_hasher::{
    hash_to_field_size::hashv_to_bn254_field_size_be_const_array, DataHasher, Hasher, HasherError,
};
use light_zero_copy::{ZeroCopy, ZeroCopyMut, ZeroCopyNew};

use crate::{extensions::ExtensionType, shared::context::TokenContext};

pub fn create_output_metadata_pointer<'a>(
    metadata_pointer_data: &ZInitMetadataPointer<'a>,
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    start_offset: usize,
) -> Result<([u8; 32], usize), ProgramError> {
    if metadata_pointer_data.authority.is_none() && metadata_pointer_data.metadata_address.is_none()
    {
        return Err(anchor_lang::prelude::ProgramError::InvalidInstructionData);
    }

    let cpi_data = output_compressed_account
        .compressed_account
        .data
        .as_mut()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let config = MetadataPointerConfig {
        authority: (metadata_pointer_data.authority.is_some(), ()),
        metadata_address: (metadata_pointer_data.metadata_address.is_some(), ()),
    };
    let byte_len = MetadataPointer::byte_len(&config);
    let end_offset = start_offset + byte_len;

    println!("MetadataPointer::new_zero_copy - start_offset: {}, end_offset: {}, total_data_len: {}, slice_len: {}",
             start_offset, end_offset, cpi_data.data.len(), end_offset - start_offset);
    println!(
        "Data slice at offset: {:?}",
        &cpi_data.data[start_offset..std::cmp::min(start_offset + 32, cpi_data.data.len())]
    );
    let (metadata_pointer, _) =
        MetadataPointer::new_zero_copy(&mut cpi_data.data[start_offset..end_offset], config)?;
    if let Some(mut authority) = metadata_pointer.authority {
        *authority = *metadata_pointer_data
            .authority
            .ok_or(ProgramError::InvalidInstructionData)?;
    }
    if let Some(mut metadata_address) = metadata_pointer.metadata_address {
        *metadata_address = *metadata_pointer_data
            .metadata_address
            .ok_or(ProgramError::InvalidInstructionData)?;
    }

    // Create the actual MetadataPointer struct for hashing
    let metadata_pointer_for_hash = MetadataPointer {
        authority: metadata_pointer_data.authority.map(|a| *a),
        metadata_address: metadata_pointer_data.metadata_address.map(|a| *a),
    };

    let hash = metadata_pointer_for_hash
        .hash::<light_hasher::Poseidon>()
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok((hash, end_offset))
}
// TODO: add update
