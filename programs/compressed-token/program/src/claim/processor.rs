use anchor_lang::prelude::ProgramError;
use light_account_checks::{AccountInfoTrait, AccountIterator};
use light_ctoken_types::state::{CompressedToken, ZExtensionStructMut};
use light_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
use pinocchio::{account_info::AccountInfo, sysvars::Sysvar};
use spl_pod::solana_msg::msg;

use crate::create_token_account::processor::transfer_lamports;

/// Accounts required for the claim instruction
pub struct ClaimAccounts<'a> {
    /// The pool PDA that receives the claimed rent
    pub pool_pda: &'a AccountInfo,
    /// The rent authority (must be signer)
    pub rent_authority: &'a AccountInfo,
}

impl<'a> ClaimAccounts<'a> {
    #[inline(always)]
    pub fn validate_and_parse(
        accounts: &'a [AccountInfo],
        pool_pda_bump: u8,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let accounts = Self {
            pool_pda: iter.next_mut("pool_pda")?,
            rent_authority: iter.next_signer("rent_authority")?,
        };
        // Verify pool PDA derivation with provided bump
        // The pool PDA should be derived as: [b"pool", rent_authority]
        let seeds = [b"pool".as_slice(), accounts.rent_authority.key().as_ref()];

        let derived_pda =
            pinocchio_pubkey::derive_address(&seeds, Some(pool_pda_bump), crate::ID.as_array());

        if derived_pda != *accounts.pool_pda.key() {
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

    for token_account in account_infos.iter().skip(2) {
        let amount = validate_and_claim(&accounts, token_account, current_slot)?;
        if let Some(amount) = amount {
            transfer_lamports(amount, token_account, accounts.pool_pda)?;
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
                // Verify rent authority
                if let Some(rent_authority) = compressible_ext.rent_authority.as_ref() {
                    if **rent_authority != *accounts.rent_authority.key() {
                        msg!("Rent authority mismatch");
                        return Ok(None);
                    }
                } else {
                    msg!("No rent authority set");
                    return Ok(None);
                }

                // Verify pool PDA matches rent recipient
                if let Some(rent_recipient) = compressible_ext.rent_recipient.as_ref() {
                    if **rent_recipient != *accounts.pool_pda.key() {
                        msg!("Pool PDA does not match rent recipient");
                        return Ok(None);
                    }
                } else {
                    msg!("No rent recipient set");
                    return Ok(None);
                }

                return Ok(compressible_ext.claim(bytes, current_slot, current_lamports));
            }
        }
    }

    msg!("No compressible extension found");
    Ok(None)
}
