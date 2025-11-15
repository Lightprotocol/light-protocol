use light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo;
use light_hasher::DataHasher;
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_sysvar::Sysvar;

use crate::{
    account::sha::LightAccount,
    compressible::compression_info::{CompressAs, HasCompressionInfo},
    cpi::v2::CpiAccounts,
    error::LightSdkError,
    instruction::account_meta::CompressedAccountMeta,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

/// Prepare account for compression.
///
/// # Arguments
/// * `program_id` - The program that owns the account
/// * `account_info` - The account to compress
/// * `account_data` - Mutable reference to the deserialized account data
/// * `compressed_account_meta` - Metadata for the compressed account
/// * `cpi_accounts` - Accounts for CPI to light system program
/// * `compression_delay` - Minimum slots before compression allowed
/// * `address_space` - Address space for validation
#[cfg(feature = "v2")]
pub fn prepare_account_for_compression<'info, A>(
    program_id: &Pubkey,
    account_info: &AccountInfo<'info>,
    account_data: &mut A,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    cpi_accounts: &CpiAccounts<'_, 'info>,
    compression_delay: &u32,
    address_space: &[Pubkey],
) -> std::result::Result<CompressedAccountInfo, ProgramError>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
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
    use light_compressed_account::address::derive_address;

    let derived_c_pda = derive_address(
        &account_info.key.to_bytes(),
        &address_space[0].to_bytes(),
        &program_id.to_bytes(),
    );

    let meta_with_address = CompressedAccountMeta {
        tree_info: compressed_account_meta.tree_info,
        address: derived_c_pda,
        output_state_tree_index: compressed_account_meta.output_state_tree_index,
    };

    let current_slot = Clock::get()?.slot;
    let last_written_slot = account_data.compression_info().last_written_slot();

    if current_slot < last_written_slot + *compression_delay as u64 {
        msg!(
            "prepare_account_for_compression failed: Cannot compress yet. {} slots remaining",
            (last_written_slot + *compression_delay as u64).saturating_sub(current_slot)
        );
        return Err(LightSdkError::ConstraintViolation.into());
    }

    account_data.compression_info_mut().set_compressed();
    {
        let mut data = account_info
            .try_borrow_mut_data()
            .map_err(|_| LightSdkError::ConstraintViolation)?;
        let writer = &mut &mut data[..];
        account_data.serialize(writer).map_err(|e| {
            msg!("Failed to serialize account data: {}", e);
            LightSdkError::ConstraintViolation
        })?;
    }

    let owner_program_id = cpi_accounts.self_program_id();
    let mut compressed_account =
        LightAccount::<A::Output>::new_empty(&owner_program_id, &meta_with_address)?;

    let compressed_data = match account_data.compress_as() {
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

    compressed_account.to_account_info()
}
