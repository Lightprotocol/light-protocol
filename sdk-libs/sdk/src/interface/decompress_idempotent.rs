#![allow(clippy::all)] // TODO: Remove.

use light_compressed_account::{
    address::derive_address, instruction_data::with_account_info::OutAccountInfo,
};
use light_compressible::DECOMPRESSED_PDA_DISCRIMINATOR;
use light_hasher::{sha256::Sha256BE, Hasher};
use light_sdk_types::instruction::account_meta::{
    CompressedAccountMeta, CompressedAccountMetaNoLamportsNoAddress,
};
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::{rent::Rent, Sysvar};

use crate::{
    account::sha::LightAccount,
    compressible::compression_info::{CompressionInfo, HasCompressionInfo},
    cpi::v2::CpiAccounts,
    error::LightSdkError,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

/// Set output for decompressed PDA format.
/// Isolated in separate function to reduce stack usage.
#[inline(never)]
#[cfg(feature = "v2")]
fn set_decompressed_pda_output(
    output: &mut OutAccountInfo,
    pda_pubkey_bytes: &[u8; 32],
) -> Result<(), LightSdkError> {
    output.data = pda_pubkey_bytes.to_vec();
    output.data_hash = Sha256BE::hash(pda_pubkey_bytes)?;
    output.discriminator = DECOMPRESSED_PDA_DISCRIMINATOR;
    Ok(())
}

/// Convert a `CompressedAccountMetaNoLamportsNoAddress` to a
/// `CompressedAccountMeta` by deriving the compressed address from the solana
/// account's pubkey.
pub fn into_compressed_meta_with_address<'info>(
    compressed_meta_no_lamports_no_address: &CompressedAccountMetaNoLamportsNoAddress,
    solana_account: &AccountInfo<'info>,
    address_space: Pubkey,
    program_id: &Pubkey,
) -> CompressedAccountMeta {
    let derived_c_pda = derive_address(
        &solana_account.key.to_bytes(),
        &address_space.to_bytes(),
        &program_id.to_bytes(),
    );

    let meta_with_address = CompressedAccountMeta {
        tree_info: compressed_meta_no_lamports_no_address.tree_info,
        address: derived_c_pda,
        output_state_tree_index: compressed_meta_no_lamports_no_address.output_state_tree_index,
    };

    meta_with_address
}

// TODO: consider folding into main fn.
/// Helper to invoke create_account on heap.
/// Both the rent_sponsor (funder) and the new account (PDA) need to sign.
#[inline(never)]
fn invoke_create_account_with_heap<'info>(
    rent_sponsor: &AccountInfo<'info>,
    rent_sponsor_seeds: &[&[u8]],
    solana_account: &AccountInfo<'info>,
    rent_minimum_balance: u64,
    space: u64,
    program_id: &Pubkey,
    pda_seeds: &[&[u8]],
    system_program: &AccountInfo<'info>,
) -> Result<(), LightSdkError> {
    let create_account_ix = system_instruction::create_account(
        rent_sponsor.key,
        solana_account.key,
        rent_minimum_balance,
        space,
        program_id,
    );

    invoke_signed(
        &create_account_ix,
        &[
            rent_sponsor.clone(),
            solana_account.clone(),
            system_program.clone(),
        ],
        &[rent_sponsor_seeds, pda_seeds],
    )
    .map_err(|e| LightSdkError::ProgramError(e))
}

/// Helper function to decompress a compressed account into a PDA
/// idempotently with seeds.
#[inline(never)]
#[cfg(feature = "v2")]
pub fn prepare_account_for_decompression_idempotent<'a, 'info, T>(
    program_id: &Pubkey,
    data: T,
    compressed_meta: CompressedAccountMeta,
    solana_account: &AccountInfo<'info>,
    rent_sponsor: &AccountInfo<'info>,
    rent_sponsor_seeds: &[&[u8]],
    cpi_accounts: &CpiAccounts<'a, 'info>,
    pda_signer_seeds: &[&[u8]],
) -> Result<
    Option<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
    LightSdkError,
>
where
    T: Clone
        + crate::account::Size
        + LightDiscriminator
        + Default
        + AnchorSerialize
        + AnchorDeserialize
        + HasCompressionInfo
        + 'info,
{
    if !solana_account.data_is_empty() {
        msg!("Account already initialized, skipping");
        return Ok(None);
    }
    let rent = Rent::get().map_err(|err| {
        msg!("Failed to get rent: {:?}", err);
        LightSdkError::Borsh
    })?;

    let light_account = LightAccount::<T>::new_close(program_id, &compressed_meta, data)?;

    // Account space needs to include discriminator + serialized data
    // T::size() already includes the full Option<CompressionInfo> footprint
    let discriminator_len = T::LIGHT_DISCRIMINATOR.len();
    let space = discriminator_len + T::size(&light_account.account)?;
    let rent_minimum_balance = rent.minimum_balance(space);

    invoke_create_account_with_heap(
        rent_sponsor,
        rent_sponsor_seeds,
        solana_account,
        rent_minimum_balance,
        space as u64,
        &cpi_accounts.self_program_id(),
        pda_signer_seeds,
        cpi_accounts.system_program()?,
    )?;

    let mut decompressed_pda = light_account.account.clone();
    *decompressed_pda.compression_info_mut_opt() = Some(CompressionInfo::new_decompressed()?);

    let mut account_data = solana_account.try_borrow_mut_data()?;
    let discriminator_len = T::LIGHT_DISCRIMINATOR.len();
    account_data[..discriminator_len].copy_from_slice(&T::LIGHT_DISCRIMINATOR);
    decompressed_pda
        .serialize(&mut &mut account_data[discriminator_len..])
        .map_err(|err| {
            msg!("Failed to serialize decompressed PDA: {:?}", err);
            LightSdkError::Borsh
        })?;

    let mut account_info_result = light_account.to_account_info()?;

    // Set output to use decompressed PDA format:
    // - discriminator: DECOMPRESSED_PDA_DISCRIMINATOR
    // - data: PDA pubkey (32 bytes)
    // - data_hash: Sha256BE(pda_pubkey)
    if let Some(output) = account_info_result.output.as_mut() {
        set_decompressed_pda_output(output, &solana_account.key.to_bytes())?;
    }

    Ok(Some(account_info_result))
}
