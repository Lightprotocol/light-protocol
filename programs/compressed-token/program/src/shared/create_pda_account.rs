use anchor_lang::solana_program::{
    program_error::ProgramError, rent::Rent, system_instruction, sysvar::Sysvar,
};
use arrayvec::ArrayVec;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Seed, Signer},
    pubkey::Pubkey,
};
use spl_pod::solana_msg::msg;

/// Configuration for creating a PDA account
#[derive(Debug)]
pub struct CreatePdaAccountConfig<'a> {
    /// The seeds used to derive the PDA (without bump)
    pub seeds: &'a [&'a [u8]],
    /// The bump seed for PDA derivation
    pub bump: u8,
    /// Size of the account in bytes
    pub account_size: usize,
    /// Program that will own the created account
    pub owner_program_id: &'a pinocchio::pubkey::Pubkey,
    /// Program used to derive the PDA (usually our program ID)
    pub derivation_program_id: &'a pinocchio::pubkey::Pubkey,
}

/// Creates a PDA account with the specified configuration.
///
/// This function abstracts the common PDA account creation pattern used across
/// create_associated_token_account, create_mint_account, and create_token_pool.
///
/// ## Process
/// 1. Calculates rent based on account size
/// 2. Builds seed array with bump
/// 3. Creates account via system program with specified owner
/// 4. Signs transaction with derived PDA seeds
pub fn create_pda_account(
    fee_payer: &AccountInfo,
    new_account: &AccountInfo,
    system_program: &AccountInfo,
    config: CreatePdaAccountConfig,
    fee_payer_config: Option<CreatePdaAccountConfig>,
    additional_lamports: Option<u64>,
) -> Result<(), ProgramError> {
    // Calculate rent
    let rent = Rent::get()?;
    let lamports =
        rent.minimum_balance(config.account_size) + additional_lamports.unwrap_or_default();

    let bump_bytes = [config.bump];
    let mut seed_vec: ArrayVec<Seed, 8> = ArrayVec::new();

    for &seed in config.seeds {
        seed_vec.push(Seed::from(seed));
    }
    seed_vec.push(Seed::from(bump_bytes.as_ref()));

    let signer = Signer::from(seed_vec.as_slice());
    let create_account_ix = system_instruction::create_account(
        &solana_pubkey::Pubkey::new_from_array(*fee_payer.key()),
        &solana_pubkey::Pubkey::new_from_array(*new_account.key()),
        lamports,
        config.account_size as u64,
        &solana_pubkey::Pubkey::new_from_array(*config.owner_program_id),
    );

    let pinocchio_instruction = pinocchio::instruction::Instruction {
        program_id: &create_account_ix.program_id.to_bytes(),
        accounts: &[
            AccountMeta::new(fee_payer.key(), true, true),
            AccountMeta::new(new_account.key(), true, true),
            pinocchio::instruction::AccountMeta::readonly(system_program.key()),
        ],
        data: &create_account_ix.data,
    };
    let bump_bytes;
    let mut seed_vec: ArrayVec<Seed, 8> = ArrayVec::new();
    let signers: ArrayVec<Signer, 2> = if let Some(config) = fee_payer_config {
        bump_bytes = [config.bump];

        for &seed in config.seeds {
            seed_vec.push(Seed::from(seed));
        }
        seed_vec.push(Seed::from(bump_bytes.as_ref()));

        let signer0 = Signer::from(seed_vec.as_slice());
        let mut signers: ArrayVec<Signer, 2> = ArrayVec::new();
        signers.push(signer0);
        signers.push(signer);
        signers
    } else {
        let mut signers: ArrayVec<Signer, 2> = ArrayVec::new();
        signers.push(signer);
        signers
    };
    msg!("seed_vec {:?}", seed_vec);
    msg!("signers {:?}", signers);
    match pinocchio::program::invoke_signed(
        &pinocchio_instruction,
        &[fee_payer, new_account, system_program],
        signers.as_slice(),
    ) {
        Ok(()) => Ok(()),
        Err(e) => Err(ProgramError::Custom(u64::from(e) as u32)),
    }
}

/// Verifies that the provided account matches the expected PDA
pub fn verify_pda<const N: usize>(
    account_key: &[u8; 32],
    seeds: &[&[u8]; N],
    bump: u8,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    let expected_pubkey = pinocchio_pubkey::derive_address(seeds, Some(bump), program_id);

    if account_key != &expected_pubkey {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}
