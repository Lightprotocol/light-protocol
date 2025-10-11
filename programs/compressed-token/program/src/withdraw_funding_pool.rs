use anchor_lang::prelude::ProgramError;
use light_account_checks::{AccountInfoTrait, AccountIterator};
use light_program_profiler::profile;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
};
use pinocchio_system::instructions::Transfer;
use spl_pod::solana_msg::msg;

use crate::create_token_account::parse_config_account;

/// Accounts required for the withdraw funding pool instruction
pub struct WithdrawFundingPoolAccounts<'a> {
    /// The pool PDA that holds the funds
    pub rent_sponsor: &'a AccountInfo,
    /// The compression_authority (must be signer and match PDA derivation)
    pub compression_authority: &'a AccountInfo,
    /// The destination account to receive the withdrawn funds
    pub destination: &'a AccountInfo,
    /// System program
    pub system_program: &'a AccountInfo,
    pub config: &'a AccountInfo,
}

impl<'a> WithdrawFundingPoolAccounts<'a> {
    #[inline(always)]
    pub fn validate_and_parse(
        accounts: &'a [AccountInfo],
    ) -> Result<(Self, u8, [u8; 2]), ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let rent_sponsor = iter.next_mut("rent_sponsor")?;
        let compression_authority = iter.next_signer("compression_authority")?;
        let destination = iter.next_mut("destination")?;
        let system_program = iter.next_account("system_program")?;
        let config = iter.next_non_mut("config")?;

        // Use the shared parse_config_account function
        let config_account = parse_config_account(config)?;

        // Validate config is not inactive (active or deprecated allowed for withdraw)
        config_account
            .validate_not_inactive()
            .map_err(ProgramError::from)?;

        if *config_account.compression_authority.as_array() != *compression_authority.key() {
            msg!("invalid rent compression_authority");
            return Err(ProgramError::InvalidSeeds);
        }
        if *config_account.rent_sponsor.as_array() != *rent_sponsor.key() {
            msg!("Invalid rent_sponsor");
            return Err(ProgramError::InvalidSeeds);
        }
        Ok((
            Self {
                rent_sponsor,
                compression_authority,
                destination,
                system_program,
                config,
            },
            config_account.rent_sponsor_bump,
            config_account.version.to_le_bytes(),
        ))
    }
}

// Process the withdraw funding pool instruction
#[profile]
pub fn process_withdraw_funding_pool(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data: [bump: u8][amount: u64]
    if instruction_data.len() < 8 {
        msg!("Invalid instruction data length");
        return Err(ProgramError::InvalidInstructionData);
    }

    let amount = u64::from_le_bytes(
        instruction_data[0..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    // Validate accounts and check PDA derivation
    let (accounts, rent_sponsor_bump, version_bytes) =
        WithdrawFundingPoolAccounts::validate_and_parse(account_infos)?;

    // Check that pool has sufficient funds
    let pool_lamports = AccountInfoTrait::lamports(accounts.rent_sponsor);
    if pool_lamports < amount {
        msg!(
            "Insufficient funds in pool. Available: {}, Requested: {}",
            pool_lamports,
            amount
        );
        return Err(ProgramError::InsufficientFunds);
    }

    // Prepare seeds for invoke_signed - the pool PDA is derived from [b"pool", compression_authority]
    let bump_bytes = [rent_sponsor_bump];
    let seed_array = [
        Seed::from(b"rent_sponsor".as_slice()),
        Seed::from(version_bytes.as_slice()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&seed_array);

    let transfer = Transfer {
        from: accounts.rent_sponsor,
        to: accounts.destination,
        lamports: amount,
    };

    transfer
        .invoke_signed(&[signer])
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32 + 6000))
}
