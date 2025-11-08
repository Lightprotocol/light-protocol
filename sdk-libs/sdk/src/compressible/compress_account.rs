#[cfg(feature = "anchor")]
use anchor_lang::{prelude::Account, AccountDeserialize, AccountSerialize};
#[cfg(feature = "anchor")]
use light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo;
use light_hasher::DataHasher;
#[cfg(feature = "anchor")]
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_msg::msg;
#[cfg(feature = "anchor")]
use solana_pubkey::Pubkey;
use solana_sysvar::Sysvar;

#[cfg(feature = "anchor")]
use crate::compressible::compression_info::CompressAs;
use crate::{
    account::sha::LightAccount,
    compressible::{compress_account_on_init_native::close, compression_info::HasCompressionInfo},
    cpi::{InvokeLightSystemProgram, LightCpiInstruction},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

#[cfg(feature = "v2")]
use crate::cpi::v2::{CpiAccounts, LightSystemProgramCpi};

/// Helper function to compress a PDA and reclaim rent.
///
/// This function uses the CompressAs trait to determine what data should be
/// stored in the compressed state. For simple cases where you want to store the
/// exact same data, implement CompressAs with `type Output = Self` and return
/// `self.clone()`. For custom compression, you can specify different field
/// values or even a different type entirely.
///
/// This requires the compressed PDA that is tied to the onchain PDA to already
/// exist, and the account type must implement CompressAs.
///
///
/// 1. updates the empty compressed PDA with data from CompressAs::compress_as()
/// 2. transfers PDA lamports to rent_recipient  
/// 1. closes onchain PDA
///
///
/// # Arguments
/// * `solana_account` - The PDA account to compress (will be closed)
/// * `compressed_account_meta` - Metadata for the compressed account (must be
///   empty but have an address)
/// * `proof` - Validity proof
/// * `cpi_accounts` - Accounts needed for CPI
/// * `rent_recipient` - The account to receive the PDA's rent
/// * `compression_delay` - The number of slots to wait before compression is
///   allowed
#[cfg(feature = "anchor")]
pub fn compress_account<'info, A>(
    solana_account: &mut Account<'info, A>,
    compressed_account_meta: &CompressedAccountMeta,
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    _rent_recipient: &AccountInfo<'info>,
    compression_delay: &u32,
) -> Result<(), crate::ProgramError>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + AccountSerialize
        + AccountDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + CompressAs,
    A::Output: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + HasCompressionInfo
        + Default
        + crate::compressible::compression_info::CompressedInitSpace,
{
    let current_slot = Clock::get()?.slot;

    let last_written_slot = solana_account.compression_info().last_written_slot();

    if current_slot < last_written_slot + *compression_delay as u64 {
        msg!(
            "compress_account failed: Cannot compress yet. {} slots remaining",
            (last_written_slot + *compression_delay as u64).saturating_sub(current_slot)
        );
        return Err(LightSdkError::ConstraintViolation.into());
    }
    // ensure re-init attack is not possible
    solana_account.compression_info_mut().set_compressed();

    let owner_program_id = cpi_accounts.self_program_id();
    let mut compressed_account =
        LightAccount::<A::Output>::new_empty(&owner_program_id, compressed_account_meta)?;

    let compressed_data = match solana_account.compress_as() {
        std::borrow::Cow::Borrowed(data) => data.clone(),
        std::borrow::Cow::Owned(data) => data,
    };
    compressed_account.account = compressed_data;

    {
        use crate::compressible::compression_info::CompressedInitSpace;
        let __lp_size = 8 + <A::Output as CompressedInitSpace>::COMPRESSED_INIT_SPACE;
        if __lp_size > 800 {
            msg!(
                "Compressed account would exceed 800-byte limit ({} bytes)",
                __lp_size
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
    }

    LightSystemProgramCpi::new_cpi(cpi_accounts.config().cpi_signer, proof)
        .with_light_account(compressed_account)?
        .invoke(cpi_accounts)?;

    Ok(())
}

#[cfg(all(feature = "anchor", feature = "v2"))]
pub fn prepare_account_for_compression<'info, A>(
    program_id: &Pubkey,
    account: &mut Account<'info, A>,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    cpi_accounts: &CpiAccounts<'_, 'info>,
    compression_delay: &u32,
    address_space: &[Pubkey],
) -> Result<CompressedAccountInfo, crate::ProgramError>
where
    A: LightDiscriminator
        //  + DataHasher
        + AnchorSerialize
        + AnchorDeserialize
        + AccountSerialize
        + AccountDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + CompressAs,
    A::Output: LightDiscriminator
        // + DataHasher
        + AnchorSerialize
        + AnchorDeserialize
        + HasCompressionInfo
        + Default
        + crate::compressible::compression_info::CompressedInitSpace,
{
    use anchor_lang::Key;
    use light_compressed_account::address::derive_address;

    let derived_c_pda = derive_address(
        &account.key().to_bytes(),
        &address_space[0].to_bytes(),
        &program_id.to_bytes(),
    );

    let meta_with_address = CompressedAccountMeta {
        tree_info: compressed_account_meta.tree_info,
        address: derived_c_pda,
        output_state_tree_index: compressed_account_meta.output_state_tree_index,
    };

    let current_slot = Clock::get()?.slot;

    let last_written_slot = account.compression_info().last_written_slot();

    if current_slot < last_written_slot + *compression_delay as u64 {
        msg!(
            "compress_account failed: Cannot compress yet. {} slots remaining",
            (last_written_slot + *compression_delay as u64).saturating_sub(current_slot)
        );
        return Err(LightSdkError::ConstraintViolation.into());
    }
    // ensure re-init attack is not possible
    account.compression_info_mut().set_compressed();

    let owner_program_id = cpi_accounts.self_program_id();
    let mut compressed_account =
        LightAccount::<A::Output>::new_empty(&owner_program_id, &meta_with_address)?;

    // msg!(
    //     "compressed_account before compress_as: {:?}",
    //     compressed_account.owner()
    // );

    // msg!(
    //     "compressed_account in_account_info: {:?}",
    //     compressed_account.in_account_info()
    // );
    // msg!(
    //     "compressed_account discriminator: {:?}",
    //     compressed_account.discriminator()
    // );
    // msg!(
    //     "compressed_account lamports: {:?}",
    //     compressed_account.lamports()
    // );

    // msg!(
    //     "DEBUG compress_account: derived_c_pda address: {:?}",
    //     meta_with_address.address
    // );
    // msg!(
    //     "DEBUG compress_account: in_account: {:?}",
    //     compressed_account.in_account_info()
    // );

    let compressed_data = match account.compress_as() {
        std::borrow::Cow::Borrowed(data) => data.clone(),
        std::borrow::Cow::Owned(data) => data,
    };
    compressed_account.account = compressed_data;

    // CU-cheap runtime safety check using compile-time compressed space
    {
        use crate::compressible::compression_info::CompressedInitSpace;
        let __lp_size = 8 + <A::Output as CompressedInitSpace>::COMPRESSED_INIT_SPACE;
        if __lp_size > 800 {
            msg!(
                "Compressed account would exceed 800-byte limit ({} bytes)",
                __lp_size
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
    }

    Ok(compressed_account.to_account_info()?)
}

/// Native Solana variant of compress_account that works with AccountInfo and pre-deserialized data.
///
/// Helper function to compress a PDA and reclaim rent.
///
/// 1. updates the empty compressed PDA with onchain PDA data
/// 2. transfers PDA lamports to rent_recipient
/// 3. closes onchain PDA
///
/// This requires the compressed PDA that is tied to the onchain PDA to already
/// exist.
///
/// # Arguments
/// * `pda_account_info` - The PDA AccountInfo to compress (will be closed)
/// * `pda_account_data` - The pre-deserialized PDA account data
/// * `compressed_account_meta` - Metadata for the compressed account (must be
///   empty but have an address)
/// * `proof` - Validity proof
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the compressed account
/// * `rent_recipient` - The account to receive the PDA's rent
/// * `compression_delay` - The number of slots to wait before compression is
///   allowed
#[cfg(feature = "v2")]
pub fn compress_pda_native<'info, A>(
    pda_account_info: &mut AccountInfo<'info>,
    pda_account_data: &mut A,
    compressed_account_meta: &CompressedAccountMeta,
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    rent_recipient: &AccountInfo<'info>,
    compression_delay: &u32,
) -> Result<(), crate::ProgramError>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + crate::compressible::compression_info::CompressedInitSpace,
{
    let current_slot = Clock::get()?.slot;

    let last_written_slot = pda_account_data.compression_info().last_written_slot();

    if current_slot < last_written_slot + *compression_delay as u64 {
        msg!(
            "compress_pda_native failed: Cannot compress yet. {} slots remaining",
            (last_written_slot + *compression_delay as u64).saturating_sub(current_slot)
        );
        return Err(LightSdkError::ConstraintViolation.into());
    }
    // ensure re-init attack is not possible
    pda_account_data.compression_info_mut().set_compressed();

    // Create the compressed account with the PDA data
    let owner_program_id = cpi_accounts.self_program_id();
    let mut compressed_account =
        LightAccount::<A>::new_empty(&owner_program_id, compressed_account_meta)?;

    let mut compressed_data = pda_account_data.clone();
    compressed_data.set_compression_info_none();
    compressed_account.account = compressed_data;

    // CU-cheap runtime safety check using compile-time compressed space
    {
        let __lp_size = 8
            + <A as crate::compressible::compression_info::CompressedInitSpace>::COMPRESSED_INIT_SPACE;
        if __lp_size > 800 {
            msg!(
                "Compressed account would exceed 800-byte limit ({} bytes)",
                __lp_size
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
    }

    // Invoke light system program to create the compressed account
    LightSystemProgramCpi::new_cpi(cpi_accounts.config().cpi_signer, proof)
        .with_light_account(compressed_account)?
        .invoke(cpi_accounts)?;
    // Close PDA account manually
    close(pda_account_info, rent_recipient.clone())?;
    Ok(())
}
