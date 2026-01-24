use light_compressed_account::instruction_data::{
    data::NewAddressParamsAssignedPacked,
    with_account_info::{CompressedAccountInfo, OutAccountInfo},
};
use light_compressible::DECOMPRESSED_PDA_DISCRIMINATOR;
use light_hasher::{sha256::Sha256BE, DataHasher, Hasher};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_pubkey::Pubkey;

use crate::{
    account::sha::LightAccount, compressible::compression_info::HasCompressionInfo,
    cpi::v2::CpiAccounts, error::LightSdkError, light_account_checks::AccountInfoTrait,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

/// Set output for decompressed PDA format.
/// Isolated in separate function to reduce stack usage.
#[inline(never)]
#[cfg(feature = "v2")]
fn set_decompressed_pda_output(
    output: &mut OutAccountInfo,
    pda_pubkey_bytes: &[u8; 32],
) -> Result<(), ProgramError> {
    output.data = pda_pubkey_bytes.to_vec();
    output.data_hash = Sha256BE::hash(pda_pubkey_bytes)
        .map_err(LightSdkError::from)
        .map_err(ProgramError::from)?;
    output.discriminator = DECOMPRESSED_PDA_DISCRIMINATOR;
    Ok(())
}

/// Prepare a compressed account on init.
///
/// Does NOT close the PDA, does NOT invoke CPI.
///
/// # Arguments
/// * `account_info` - The PDA AccountInfo
/// * `account_data` - Mutable reference to deserialized account data
/// * `address` - The address for the compressed account
/// * `new_address_param` - Address parameters for the compressed account
/// * `output_state_tree_index` - Output state tree index
/// * `cpi_accounts` - Accounts for validation
/// * `address_space` - Address space for validation (can contain multiple tree
///   pubkeys)
/// * `with_data` - If true, copies account data to compressed account, if
///   false, creates empty compressed account
///
/// # Returns
/// CompressedAccountInfo
#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v2")]
pub fn prepare_compressed_account_on_init<'info, A>(
    account_info: &AccountInfo<'info>,
    account_data: &mut A,
    compression_config: &crate::interface::LightConfig,
    address: [u8; 32],
    new_address_param: NewAddressParamsAssignedPacked,
    output_state_tree_index: u8,
    cpi_accounts: &CpiAccounts<'_, 'info>,
    address_space: &[Pubkey],
    with_data: bool,
) -> std::result::Result<CompressedAccountInfo, ProgramError>
where
    A: DataHasher
        + LightDiscriminator
        + AnchorSerialize
        + AnchorDeserialize
        + Default
        + Clone
        + HasCompressionInfo,
{
    // TODO: consider not supporting yet.
    // Fail-fast: with_data=true is not yet supported in macro-generated code
    // if with_data {
    //     msg!("with_data=true is not supported yet");
    //     return Err(LightSdkError::ConstraintViolation.into());
    // }

    let tree = cpi_accounts
        .get_tree_account_info(new_address_param.address_merkle_tree_account_index as usize)
        .map_err(|_| {
            msg!(
                "Failed to get tree account at index {}",
                new_address_param.address_merkle_tree_account_index
            );
            LightSdkError::ConstraintViolation
        })?
        .pubkey();
    if !address_space.iter().any(|a| a == &tree) {
        msg!("Address tree {} not in allowed address space", tree);
        return Err(LightSdkError::ConstraintViolation.into());
    }
    // Initialize CompressionInfo from config
    // Note: Rent sponsor is not stored per-account; compression always sends rent to config's rent_sponsor
    use solana_sysvar::{clock::Clock, Sysvar};
    let current_slot = Clock::get()?.slot;
    *account_data.compression_info_mut_opt() = Some(
        super::compression_info::CompressionInfo::new_from_config(compression_config, current_slot),
    );

    if with_data {
        account_data.compression_info_mut()?.set_compressed();
    } else {
        account_data
            .compression_info_mut()?
            .bump_last_claimed_slot()?;
    }
    {
        let mut data = account_info
            .try_borrow_mut_data()
            .map_err(|_| LightSdkError::ConstraintViolation)?;
        // Skip the 8-byte Anchor discriminator when serializing
        account_data.serialize(&mut &mut data[8..]).map_err(|e| {
            msg!("Failed to serialize account data: {}", e);
            LightSdkError::ConstraintViolation
        })?;
    }

    let owner_program_id = cpi_accounts.self_program_id();

    let mut compressed_account =
        LightAccount::<A>::new_init(&owner_program_id, Some(address), output_state_tree_index);

    if with_data {
        let mut compressed_data = account_data.clone();
        compressed_data.set_compression_info_none()?;
        compressed_account.account = compressed_data;
    } else {
        compressed_account.remove_data();
    }

    let mut account_info_result = compressed_account.to_account_info()?;

    // For decompressed PDAs (with_data = false), store the PDA pubkey in data
    // and set the decompressed discriminator
    if !with_data {
        if let Some(output) = account_info_result.output.as_mut() {
            set_decompressed_pda_output(output, &account_info.key())?;
        }
    }

    Ok(account_info_result)
}
