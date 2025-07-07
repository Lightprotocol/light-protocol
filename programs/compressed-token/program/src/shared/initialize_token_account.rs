use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use pinocchio::account_info::AccountInfo;
use spl_pod::bytemuck::pod_from_bytes_mut;
use spl_token_2022::pod::PodAccount;
use spl_token_2022::state::AccountState;

/// Initialize a token account using spl-pod with zero balance and default settings
pub fn initialize_token_account(
    token_account_info: &AccountInfo,
    mint_pubkey: &[u8; 32],
    owner_pubkey: &[u8; 32],
) -> Result<(), ProgramError> {
    // Access the token account data as mutable bytes
    let mut token_account_data = AccountInfoTrait::try_borrow_mut_data(token_account_info)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Use zero-copy PodAccount to initialize the token account
    let pod_account = pod_from_bytes_mut::<PodAccount>(&mut token_account_data)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Initialize the token account fields
    pod_account.mint = solana_pubkey::Pubkey::from(*mint_pubkey);
    pod_account.owner = solana_pubkey::Pubkey::from(*owner_pubkey);
    pod_account.amount = 0u64.into(); // Start with 0 balance
    pod_account.delegate = spl_token_2022::pod::PodCOption::none(); // No delegate
    pod_account.state = AccountState::Initialized as u8; // Set to Initialized state
    pod_account.is_native = spl_token_2022::pod::PodCOption::none(); // Not a native token
    pod_account.delegated_amount = 0u64.into(); // No delegated amount
    pod_account.close_authority = spl_token_2022::pod::PodCOption::none(); // No close authority

    Ok(())
}