#[cfg(feature = "anchor")]
use anchor_lang::{prelude::Account, AccountDeserialize, AccountSerialize, AccountsClose};
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_msg::msg;
use solana_sysvar::Sysvar;

#[cfg(feature = "anchor")]
use crate::compressible::compression_info::CompressAs;
use crate::{
    account::sha::LightAccount,
    compressible::{compress_account_on_init_native::close, compression_info::HasCompressionInfo},
    cpi::{CpiAccountsSmall, CpiInputs},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

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
    cpi_accounts: CpiAccountsSmall<'_, 'info>,
    rent_recipient: &AccountInfo<'info>,
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
        + Default,
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
    let mut compressed_account = LightAccount::<'_, A::Output>::new_mut_without_data(
        &owner_program_id,
        compressed_account_meta,
    )?;

    let compressed_data = match solana_account.compress_as() {
        std::borrow::Cow::Borrowed(data) => data.clone(),
        std::borrow::Cow::Owned(data) => data,
    };
    compressed_account.account = compressed_data;

    let cpi_inputs = CpiInputs::new(proof, vec![compressed_account.to_account_info()?]);

    // invoke light system program to update compressed account
    cpi_inputs.invoke_light_system_program_small(cpi_accounts)?;

    // cleanup
    solana_account.close(rent_recipient.clone())?;

    Ok(())
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
pub fn compress_pda_native<'info, A>(
    pda_account_info: &mut AccountInfo<'info>,
    pda_account_data: &mut A,
    compressed_account_meta: &CompressedAccountMeta,
    proof: ValidityProof,
    cpi_accounts: CpiAccountsSmall<'_, 'info>,
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
        + HasCompressionInfo,
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
        LightAccount::<'_, A>::new_mut_without_data(&owner_program_id, compressed_account_meta)?;

    let mut compressed_data = pda_account_data.clone();
    compressed_data.set_compression_info_none();
    compressed_account.account = compressed_data;

    // Create CPI inputs
    let cpi_inputs = CpiInputs::new(proof, vec![compressed_account.to_account_info()?]);

    // Invoke light system program to create the compressed account
    cpi_inputs.invoke_light_system_program_small(cpi_accounts)?;
    // Close PDA account manually
    close(pda_account_info, rent_recipient.clone())?;
    Ok(())
}
