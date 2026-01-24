use light_compressed_account::instruction_data::with_account_info::{
    CompressedAccountInfo, InAccountInfo,
};
use light_compressible::{rent::AccountRentState, DECOMPRESSED_PDA_DISCRIMINATOR};
use light_hasher::{sha256::Sha256BE, DataHasher, Hasher};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_sysvar::{rent::Rent, Sysvar};

use crate::{
    account::sha::LightAccount,
    compressible::compression_info::{CompressAs, HasCompressionInfo},
    cpi::v2::CpiAccounts,
    error::LightSdkError,
    instruction::account_meta::CompressedAccountMeta,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

/// Set input for decompressed PDA format.
/// Isolated in separate function to reduce stack usage.
#[inline(never)]
#[cfg(feature = "v2")]
fn set_decompressed_pda_input(
    input: &mut InAccountInfo,
    pda_pubkey_bytes: &[u8; 32],
) -> Result<(), ProgramError> {
    input.data_hash = Sha256BE::hash(pda_pubkey_bytes)
        .map_err(LightSdkError::from)
        .map_err(ProgramError::from)?;
    input.discriminator = DECOMPRESSED_PDA_DISCRIMINATOR;
    Ok(())
}
/// Prepare account for compression.
///
/// # Arguments
/// * `program_id` - The program that owns the account
/// * `account_info` - The account to compress
/// * `account_data` - Mutable reference to the deserialized account data
/// * `compressed_account_meta` - Metadata for the compressed account
/// * `cpi_accounts` - Accounts for CPI to light system program
/// * `address_space` - Address space for validation
#[cfg(feature = "v2")]
pub fn prepare_account_for_compression<'info, A>(
    program_id: &Pubkey,
    account_info: &AccountInfo<'info>,
    account_data: &mut A,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    cpi_accounts: &CpiAccounts<'_, 'info>,
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
        + crate::interface::compression_info::CompressedInitSpace,
{
    use light_compressed_account::address::derive_address;

    // v2 address derive using PDA as seed
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
    // Rent-function gating: account must be compressible w.r.t. rent function (current+next epoch)
    let bytes = account_info.data_len() as u64;
    let current_lamports = account_info.lamports();
    let rent_exemption_lamports = Rent::get()
        .map_err(|_| LightSdkError::ConstraintViolation)?
        .minimum_balance(bytes as usize);
    let ci = account_data.compression_info()?;
    let last_claimed_slot = ci.last_claimed_slot();
    let rent_cfg = ci.rent_config;
    let state = AccountRentState {
        num_bytes: bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
    };
    if state
        .is_compressible(&rent_cfg, rent_exemption_lamports)
        .is_none()
    {
        msg!(
            "prepare_account_for_compression failed: \
            Account is not compressible by rent function. \
            slot: {}, lamports: {}, bytes: {}, rent_exemption_lamports: {}, last_claimed_slot: {}, rent_config: {:?}",
            current_slot,
            current_lamports,
            bytes,
            rent_exemption_lamports,
            last_claimed_slot,
            rent_cfg
        );
        return Err(LightSdkError::ConstraintViolation.into());
    }

    account_data.compression_info_mut()?.set_compressed();
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
        use crate::interface::compression_info::CompressedInitSpace;
        let __lp_size = 8 + <A::Output as CompressedInitSpace>::COMPRESSED_INIT_SPACE;
        if __lp_size > 800 {
            msg!(
                "Compressed account would exceed 800-byte limit ({} bytes)",
                __lp_size
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
    }

    let mut account_info_result = compressed_account.to_account_info()?;

    // Fix input to use the decompressed PDA format:
    // - discriminator: DECOMPRESSED_PDA_DISCRIMINATOR
    // - data_hash: Sha256BE(pda_pubkey)
    if let Some(input) = account_info_result.input.as_mut() {
        set_decompressed_pda_input(input, &account_info.key.to_bytes())?;
    }

    Ok(account_info_result)
}
