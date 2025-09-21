#![allow(clippy::all)] // TODO: Remove.

use light_compressed_account::address::derive_compressed_address;
use light_hasher::DataHasher;
use light_sdk_types::instruction::account_meta::{
    CompressedAccountMeta, CompressedAccountMetaNoLamportsNoAddress,
};
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::Sysvar;

use crate::{
    account::sha::LightAccount, compressible::compression_info::HasCompressionInfo,
    cpi::CpiAccountsSmall, error::LightSdkError, AnchorDeserialize, AnchorSerialize,
    LightDiscriminator,
};

/// Helper to invoke create_account on heap.
#[inline(never)]
#[cold]
fn invoke_create_account_with_heap<'info>(
    rent_payer: &AccountInfo<'info>,
    solana_account: &AccountInfo<'info>,
    rent_minimum_balance: u64,
    space: u64,
    program_id: &Pubkey,
    seeds: &[&[u8]],
    system_program: &AccountInfo<'info>,
) -> Result<(), LightSdkError> {
    let create_account_ix = system_instruction::create_account(
        rent_payer.key,
        solana_account.key,
        rent_minimum_balance,
        space,
        program_id,
    );
    let accounts = vec![
        rent_payer.clone(),
        solana_account.clone(),
        system_program.clone(),
    ];

    invoke_signed(&create_account_ix, &accounts, &[seeds])
        .map_err(|e| LightSdkError::ProgramError(e))
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
    let derived_c_pda = derive_compressed_address(
        &solana_account.key.into(),
        &address_space.into(),
        &program_id.into(),
    );

    let meta_with_address = CompressedAccountMeta {
        tree_info: compressed_meta_no_lamports_no_address.tree_info,
        address: derived_c_pda,
        output_state_tree_index: compressed_meta_no_lamports_no_address.output_state_tree_index,
    };

    meta_with_address
}

/// Helper function to decompress multiple compressed accounts into PDAs
/// idempotently with seeds. Does not invoke the zk compression CPI. This
/// function processes accounts of a single type and returns
/// CompressedAccountInfo for CPI batching. It's idempotent, meaning it can be
/// called multiple times with the same compressed accounts and it will only
/// decompress them once.
#[inline(never)]
pub fn prepare_account_for_decompression_idempotent<'a, 'info, T>(
    program_id: &Pubkey,
    data: T,
    compressed_meta: CompressedAccountMeta,
    solana_account: &AccountInfo<'info>,
    rent_payer: &AccountInfo<'info>,
    cpi_accounts: &CpiAccountsSmall<'a, 'info>,
    signer_seeds: &[&[u8]],
) -> Result<
    Option<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
    LightSdkError,
>
where
    T: Clone
        + crate::account::Size
        + DataHasher
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

    let mut light_account = LightAccount::<'_, T>::new_mut(&program_id, &compressed_meta, data)?;

    let space = T::size(&light_account.account);
    let rent_minimum_balance = rent.minimum_balance(space);

    invoke_create_account_with_heap(
        rent_payer,
        solana_account,
        rent_minimum_balance,
        space as u64,
        &cpi_accounts.self_program_id(),
        signer_seeds,
        cpi_accounts.system_program()?,
    )?;

    // set compression info
    let mut decompressed_pda = light_account.account.clone();
    *decompressed_pda.compression_info_mut_opt() =
        Some(super::CompressionInfo::new_decompressed()?);

    // serialize onchain account
    let mut account_data = solana_account.try_borrow_mut_data()?;
    let discriminator_len = T::LIGHT_DISCRIMINATOR.len();
    account_data[..discriminator_len].copy_from_slice(&T::LIGHT_DISCRIMINATOR);
    decompressed_pda
        .serialize(&mut &mut account_data[discriminator_len..])
        .map_err(|err| {
            msg!("Failed to serialize decompressed PDA: {:?}", err);
            LightSdkError::Borsh
        })?;

    light_account.remove_data();
    Ok(Some(light_account.to_account_info()?))
}
