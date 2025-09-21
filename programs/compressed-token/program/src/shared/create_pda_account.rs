use anchor_lang::solana_program::program_error::ProgramError;
use arrayvec::ArrayVec;
use light_profiler::profile;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::Pubkey,
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;

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
    pub owner_program_id: &'a Pubkey,
    /// Program used to derive the PDA (usually our program ID)
    pub derivation_program_id: &'a Pubkey,
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
#[profile]
pub fn create_pda_account(
    fee_payer: &AccountInfo,
    new_account: &AccountInfo,
    config: CreatePdaAccountConfig,
    fee_payer_config: Option<CreatePdaAccountConfig>,
    additional_lamports: Option<u64>,
) -> Result<(), ProgramError> {
    spl_pod::solana_msg::msg!("fee_payer: {:?}", fee_payer);
    spl_pod::solana_msg::msg!("config: {:?}", config);
    spl_pod::solana_msg::msg!("fee_payer_config: {:?}", fee_payer_config);
    // Calculate rent
    let rent = Rent::get().map_err(|_| ProgramError::UnsupportedSysvar)?;
    let lamports =
        rent.minimum_balance(config.account_size) + additional_lamports.unwrap_or_default();

    let create_account = CreateAccount {
        from: fee_payer,
        to: new_account,
        lamports,
        space: config.account_size as u64,
        owner: config.owner_program_id,
    };

    let bump_bytes = [config.bump];
    let mut seed_vec: ArrayVec<Seed, 8> = ArrayVec::new();
    for &seed in config.seeds {
        seed_vec.push(Seed::from(seed));
    }
    seed_vec.push(Seed::from(bump_bytes.as_ref()));

    let signer = Signer::from(seed_vec.as_slice());

    let bump_bytes;
    let mut seed_vec: ArrayVec<Seed, 8> = ArrayVec::new();
    let signers: ArrayVec<Signer, 2> = if let Some(config) = fee_payer_config {
        bump_bytes = [config.bump];

        for &seed in config.seeds {
            seed_vec.push(Seed::from(seed));
        }
        seed_vec.push(Seed::from(bump_bytes.as_ref()));

        let signer0 = Signer::from(seed_vec.as_slice());
        let signers = [signer0, signer];
        signers.into()
    } else {
        let mut signers: ArrayVec<Signer, 2> = ArrayVec::new();
        signers.push(signer);
        signers
    };
    create_account
        .invoke_signed(signers.as_slice())
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))
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
