use anchor_lang::{prelude::ProgramError, pubkey, solana_program::system_instruction};
use light_account_checks::{
    checks::{check_discriminator, check_owner},
    AccountInfoTrait, AccountIterator,
};
use light_compressible::config::CompressibleConfig;
use light_profiler::profile;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program::invoke_signed,
};
use spl_pod::{bytemuck, solana_msg::msg};

/// Accounts required for the withdraw funding pool instruction
pub struct WithdrawFundingPoolAccounts<'a> {
    /// The pool PDA that holds the funds
    pub rent_recipient: &'a AccountInfo,
    /// The rent_authority (must be signer and match PDA derivation)
    pub rent_authority: &'a AccountInfo,
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
        let rent_recipient = iter.next_mut("rent_recipient")?;
        let rent_authority = iter.next_signer("rent_authority")?;
        let destination = iter.next_mut("destination")?;
        let system_program = iter.next_account("system_program")?;
        let config = iter.next_non_mut("config")?;

        check_owner(
            &pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").to_bytes(),
            config,
        )?;
        let data = config.try_borrow_data().unwrap();
        check_discriminator::<CompressibleConfig>(&data[..])?;
        let account = bytemuck::pod_from_bytes::<CompressibleConfig>(&data[8..])
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // Validate config is not inactive (active or deprecated allowed for withdraw)
        account
            .validate_not_inactive()
            .map_err(ProgramError::from)?;

        if *account.rent_authority.as_array() != *rent_authority.key() {
            msg!("invalid rent rent_authority");
            return Err(ProgramError::InvalidSeeds);
        }
        if *account.rent_recipient.as_array() != *rent_recipient.key() {
            msg!("Invalid rent_recipient");
            return Err(ProgramError::InvalidSeeds);
        }
        Ok((
            Self {
                rent_recipient,
                rent_authority,
                destination,
                system_program,
                config,
            },
            account.rent_recipient_bump,
            account.version.to_le_bytes(),
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
    let (accounts, rent_recipient_bump, version_bytes) =
        WithdrawFundingPoolAccounts::validate_and_parse(account_infos)?;

    // Check that pool has sufficient funds
    let pool_lamports = AccountInfoTrait::lamports(accounts.rent_recipient);
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
        &solana_pubkey::Pubkey::new_from_array(*accounts.rent_recipient.key()),
        &solana_pubkey::Pubkey::new_from_array(*accounts.destination.key()),
        amount,
    );

    // Convert to pinocchio instruction format
    let pinocchio_ix = pinocchio::instruction::Instruction {
        program_id: accounts.system_program.key(),
        accounts: &[
            pinocchio::instruction::AccountMeta::writable_signer(accounts.rent_recipient.key()),
            pinocchio::instruction::AccountMeta::writable(accounts.destination.key()),
        ],
        data: &transfer_ix.data,
    };

    // Prepare seeds for invoke_signed - the pool PDA is derived from [b"pool", rent_authority]
    let bump_bytes = [rent_recipient_bump];
    let seed_array = [
        Seed::from(b"rent_recipient".as_slice()),
        Seed::from(version_bytes.as_slice()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&seed_array);

    // Invoke the system program to transfer lamports with PDA as signer
    invoke_signed(
        &pinocchio_ix,
        &[accounts.rent_recipient, accounts.destination],
        &[signer],
    )
    .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;

    Ok(())
}
