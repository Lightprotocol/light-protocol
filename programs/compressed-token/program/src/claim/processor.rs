use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::{AccountInfoTrait, AccountIterator};
use light_compressible::{config::CompressibleConfig, rent::get_rent_exemption_lamports};
use light_ctoken_types::state::{CToken, ZExtensionStructMut};
use light_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
use pinocchio::{account_info::AccountInfo, sysvars::Sysvar};
use spl_pod::solana_msg::msg;

use crate::{create_token_account::parse_config_account, shared::transfer_lamports};
// TODO: refactor into file instead of dir
/// Accounts required for the claim instruction
pub struct ClaimAccounts<'a> {
    /// The rent_sponsor PDA that receives the claimed rent
    pub rent_sponsor: &'a AccountInfo,
    /// The rent authority (must be signer)
    pub compression_authority: &'a AccountInfo,
    /// Parsed CompressibleConfig for accessing RentConfig
    pub config_account: CompressibleConfig,
}

impl<'a> ClaimAccounts<'a> {
    #[inline(always)]
    pub fn validate_and_parse(accounts: &'a [AccountInfo]) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let rent_sponsor = iter.next_mut("rent_sponsor")?;
        let compression_authority = iter.next_signer("compression_authority")?;
        let config = iter.next_non_mut("compressible config")?;

        // Use the shared parse_config_account function
        let config_account = parse_config_account(config)?;

        // Validate config is not inactive (active or deprecated allowed for claim)
        config_account
            .validate_not_inactive()
            .map_err(ProgramError::from)?;

        if *config_account.compression_authority.as_array() != *compression_authority.key() {
            msg!("invalid rent authority");
            return Err(ErrorCode::InvalidCompressAuthority.into());
        }
        if *config_account.rent_sponsor.as_array() != *rent_sponsor.key() {
            msg!("Invalid rent sponsor PDA"); // TODO: add custom error
            return Err(ErrorCode::InvalidCompressAuthority.into());
        }

        Ok(Self {
            rent_sponsor,
            compression_authority,
            config_account: *config_account,
        })
    }
}

// Process the claim instruction
#[profile]
pub fn process_claim(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse bump from instruction data
    if !instruction_data.is_empty() {
        msg!("Instruction data must be empty.");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Validate and get accounts
    let accounts = ClaimAccounts::validate_and_parse(account_infos)?;

    let current_slot = pinocchio::sysvars::clock::Clock::get()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?
        .slot;

    for token_account in account_infos.iter().skip(3) {
        let amount = validate_and_claim(
            &accounts,
            &accounts.config_account,
            token_account,
            current_slot,
        )?;
        if let Some(amount) = amount {
            transfer_lamports(amount, token_account, accounts.rent_sponsor)
                .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
        }
    }
    Ok(())
}

fn validate_and_claim(
    accounts: &ClaimAccounts,
    config_account: &CompressibleConfig,
    token_account: &AccountInfo,
    current_slot: u64,
) -> Result<Option<u64>, ProgramError> {
    // Get current lamports balance
    let current_lamports = AccountInfoTrait::lamports(token_account);
    // Claim rent for completed epochs
    let bytes = token_account.data_len() as u64;
    // Parse and process the token account
    let mut token_account_data = AccountInfoTrait::try_borrow_mut_data(token_account)?;
    let (mut compressed_token, _) = CToken::zero_copy_at_mut(&mut token_account_data)?;

    // Find compressible extension
    if let Some(extensions) = compressed_token.extensions.as_mut() {
        for extension in extensions {
            if let ZExtensionStructMut::Compressible(compressible_ext) = extension {
                // TODO: extract
                if compressible_ext.compression_authority != *accounts.compression_authority.key() {
                    msg!("Rent authority mismatch");
                    return Ok(None);
                }
                if compressible_ext.rent_sponsor != *accounts.rent_sponsor.key() {
                    msg!("Rent sponsor PDA does not match rent recipient");
                    return Ok(None);
                }

                // Verify config version matches
                let account_version: u16 = compressible_ext.config_account_version.into();
                let config_version = config_account.version;

                if account_version != config_version {
                    msg!(
                        "Config version mismatch: account has v{}, config is v{}",
                        account_version,
                        config_version
                    );
                    return Err(ProgramError::InvalidAccountData);
                }

                let base_lamports = get_rent_exemption_lamports(bytes).unwrap();

                // Calculate claim with current RentConfig
                let claim_result = compressible_ext
                    .claim(bytes, current_slot, current_lamports, base_lamports)
                    .map_err(|_| ProgramError::InvalidAccountData)?;

                // Update RentConfig after claim calculation (even if claim_result is None)
                compressible_ext
                    .rent_config
                    .set(&config_account.rent_config);

                return Ok(claim_result);
            }
        }
    }

    msg!("No compressible extension found");
    Ok(None)
}
