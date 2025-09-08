use anchor_lang::{prelude::ProgramError, solana_program::system_instruction};
use light_account_checks::{AccountInfoTrait, AccountIterator};
use light_profiler::profile;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program::invoke_signed,
};
use spl_pod::solana_msg::msg;

/// Accounts required for the withdraw funding pool instruction
pub struct WithdrawFundingPoolAccounts<'a> {
    /// The pool PDA that holds the funds
    pub pool_pda: &'a AccountInfo,
    /// The authority (must be signer and match PDA derivation)
    pub authority: &'a AccountInfo,
    /// The destination account to receive the withdrawn funds
    pub destination: &'a AccountInfo,
    /// System program
    pub system_program: &'a AccountInfo,
}

impl<'a> WithdrawFundingPoolAccounts<'a> {
    #[inline(always)]
    pub fn validate_and_parse(
        accounts: &'a [AccountInfo],
        pool_pda_bump: u8,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let accounts = Self {
            pool_pda: iter.next_mut("pool_pda")?,
            authority: iter.next_signer("authority")?,
            destination: iter.next_mut("destination")?,
            system_program: iter.next_account("system_program")?,
        };

        // Verify pool PDA derivation with authority and provided bump
        // The pool PDA should be derived as: [b"pool", authority]
        let seeds = [b"pool".as_slice(), accounts.authority.key().as_ref()];

        let derived_pda =
            pinocchio_pubkey::derive_address(&seeds, Some(pool_pda_bump), crate::ID.as_array());

        if derived_pda != *accounts.pool_pda.key() {
            msg!("Invalid pool PDA derivation with bump {}", pool_pda_bump);
            return Err(ProgramError::InvalidSeeds);
        }

        Ok(accounts)
    }
}

// Process the withdraw funding pool instruction
#[profile]
pub fn process_withdraw_funding_pool(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data: [bump: u8][amount: u64]
    if instruction_data.len() < 9 {
        msg!("Invalid instruction data length");
        return Err(ProgramError::InvalidInstructionData);
    }

    let pool_pda_bump = instruction_data[0];
    let amount = u64::from_le_bytes(
        instruction_data[1..9]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    // Validate accounts and check PDA derivation
    let accounts = WithdrawFundingPoolAccounts::validate_and_parse(account_infos, pool_pda_bump)?;

    // Check that pool has sufficient funds
    let pool_lamports = AccountInfoTrait::lamports(accounts.pool_pda);
    if pool_lamports < amount {
        msg!(
            "Insufficient funds in pool. Available: {}, Requested: {}",
            pool_lamports,
            amount
        );
        return Err(ProgramError::InsufficientFunds);
    }

    // Create system transfer instruction
    let transfer_ix = system_instruction::transfer(
        &solana_pubkey::Pubkey::new_from_array(*accounts.pool_pda.key()),
        &solana_pubkey::Pubkey::new_from_array(*accounts.destination.key()),
        amount,
    );

    // Convert to pinocchio instruction format
    let pinocchio_ix = pinocchio::instruction::Instruction {
        program_id: accounts.system_program.key(),
        accounts: &[
            pinocchio::instruction::AccountMeta::writable_signer(accounts.pool_pda.key()),
            pinocchio::instruction::AccountMeta::writable(accounts.destination.key()),
        ],
        data: &transfer_ix.data,
    };

    // Prepare seeds for invoke_signed - the pool PDA is derived from [b"pool", authority]
    let authority_bytes = accounts.authority.key();
    let bump_bytes = [pool_pda_bump];
    let seed_array = [
        Seed::from(b"pool".as_slice()),
        Seed::from(authority_bytes.as_ref()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&seed_array);

    // Invoke the system program to transfer lamports with PDA as signer
    invoke_signed(
        &pinocchio_ix,
        &[accounts.pool_pda, accounts.destination],
        &[signer],
    )
    .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;

    Ok(())
}
