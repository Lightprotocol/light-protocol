use anchor_lang::{prelude::ProgramError, pubkey};
use light_account_checks::{
    checks::{check_discriminator, check_owner},
    AccountInfoTrait, AccountIterator,
};
use light_compressible::{config::CompressibleConfig, rent::get_rent_exemption_lamports};
use light_ctoken_types::state::{CompressedToken, ZExtensionStructMut};
use light_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
use pinocchio::{account_info::AccountInfo, sysvars::Sysvar};
use spl_pod::{bytemuck, solana_msg::msg};

use crate::shared::transfer_lamports;

/// Accounts required for the claim instruction
pub struct ClaimAccounts<'a> {
    /// The pool PDA that receives the claimed rent
    pub rent_recipient: &'a AccountInfo,
    /// The rent authority (must be signer)
    pub rent_authority: &'a AccountInfo,
    pub config: &'a AccountInfo,
}

impl<'a> ClaimAccounts<'a> {
    #[inline(always)]
    pub fn validate_and_parse(
        accounts: &'a [AccountInfo],
        pool_pda_bump: u8,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let accounts = Self {
            rent_recipient: iter.next_mut("pool_pda")?,
            rent_authority: iter.next_signer("rent_authority")?,
            config: iter.next_non_mut("compressible config")?,
        };

        check_owner(
            &pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").to_bytes(),
            accounts.config,
        )?;
        let data = accounts.config.try_borrow_data().unwrap();
        check_discriminator::<CompressibleConfig>(&data[..])?;
        let account = bytemuck::pod_from_bytes::<CompressibleConfig>(&data[8..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        if *account.rent_authority.as_array() != *accounts.rent_authority.key() {
            msg!("invalid rent authority");
            return Err(ProgramError::InvalidSeeds);
        }
        if *account.rent_recipient.as_array() != *accounts.rent_recipient.key() {
            msg!("Invalid pool PDA derivation with bump {}", pool_pda_bump);
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(accounts)
    }
}

// Process the claim instruction
#[profile]
pub fn process_claim(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse bump from instruction data
    if instruction_data.is_empty() {
        msg!("Missing pool PDA bump in instruction data");
        return Err(ProgramError::InvalidInstructionData);
    }
    let pool_pda_bump = *instruction_data
        .first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    // Validate and get accounts
    let accounts = ClaimAccounts::validate_and_parse(account_infos, pool_pda_bump)?;

    let current_slot = pinocchio::sysvars::clock::Clock::get()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?
        .slot;

    for token_account in account_infos.iter().skip(3) {
        let amount = validate_and_claim(&accounts, token_account, current_slot)?;
        if let Some(amount) = amount {
            transfer_lamports(amount, token_account, accounts.rent_recipient)
                .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
        }
    }
    Ok(())
}

fn validate_and_claim(
    accounts: &ClaimAccounts,
    token_account: &AccountInfo,
    current_slot: u64,
) -> Result<Option<u64>, ProgramError> {
    // Get current lamports balance
    let current_lamports = AccountInfoTrait::lamports(token_account);
    // Claim rent for completed epochs
    let bytes = token_account.data_len() as u64;
    // Parse and process the token account
    let mut token_account_data = AccountInfoTrait::try_borrow_mut_data(token_account)?;
    let (mut compressed_token, _) = CompressedToken::zero_copy_at_mut(&mut token_account_data)?;

    // Find compressible extension
    if let Some(extensions) = compressed_token.extensions.as_mut() {
        for extension in extensions {
            if let ZExtensionStructMut::Compressible(compressible_ext) = extension {
                if compressible_ext.rent_authority != *accounts.rent_authority.key() {
                    msg!("Rent authority mismatch");
                    return Ok(None);
                }
                if compressible_ext.rent_recipient != *accounts.rent_recipient.key() {
                    msg!("Pool PDA does not match rent recipient");
                    return Ok(None);
                }
                let base_lamports = get_rent_exemption_lamports(bytes).unwrap();

                return compressible_ext
                    .claim(bytes, current_slot, current_lamports, base_lamports)
                    .map_err(|_| ProgramError::InvalidAccountData);
            }
        }
    }

    msg!("No compressible extension found");
    Ok(None)
}
