use anchor_lang::prelude::{AccountInfo, ProgramError, SolanaSysvar};
use anchor_lang::solana_program::{
    program::invoke_signed, pubkey::Pubkey, rent::Rent, system_instruction,
};
use light_zero_copy::borsh::Deserialize;
use spl_pod::bytemuck::pod_from_bytes_mut;
use spl_token_2022::pod::PodAccount;
use spl_token_2022::state::AccountState;

use super::{
    accounts::CreateAssociatedTokenAccountAccounts,
    instruction_data::CreateAssociatedTokenAccountInstructionData,
};

/// Note:
/// - we don't validate the mint because it would be very expensive with compressed mints
/// - it is possible to create an associated token account for non existing mints
/// - accounts with non existing mints can never have a balance
/// Process the create associated token account instruction
pub fn process_create_associated_token_account<'info>(
    account_infos: &'info [AccountInfo<'info>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data using zero-copy
    let (inputs, _) = CreateAssociatedTokenAccountInstructionData::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    // Convert to solana pubkeys for validation
    let owner_pubkey = Pubkey::new_from_array(inputs.owner.to_bytes());
    let mint_pubkey = Pubkey::new_from_array(inputs.mint.to_bytes());

    // Validate and get accounts
    let accounts =
        CreateAssociatedTokenAccountAccounts::get_checked(account_infos, &mint_pubkey, false)?;

    {
        // Define the PDA seeds for signing
        let signer_seeds = &[
            owner_pubkey.as_ref(),
            crate::ID.as_ref(),
            mint_pubkey.as_ref(),
            &[inputs.bump],
        ];

        // Calculate rent for SPL token account (165 bytes)
        let token_account_size = 165_usize;
        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(token_account_size);

        // Create the associated token account
        let create_account_instruction = system_instruction::create_account(
            accounts.fee_payer.key,
            accounts.associated_token_account.key,
            rent_lamports,
            token_account_size as u64,
            &crate::ID,
        );

        // Execute the create account instruction with PDA signing
        invoke_signed(
            &create_account_instruction,
            &[
                accounts.fee_payer.clone(),
                accounts.associated_token_account.clone(),
                accounts.system_program.clone(),
            ],
            &[signer_seeds],
        )?;
    }

    // Initialize the token account using spl-pod
    {
        // Access the token account data as mutable bytes
        let mut token_account_data = accounts.associated_token_account.try_borrow_mut_data()?;

        // Use zero-copy PodAccount to initialize the token account
        let pod_account = pod_from_bytes_mut::<PodAccount>(&mut token_account_data)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // Initialize the token account fields
        pod_account.mint = mint_pubkey;
        pod_account.owner = owner_pubkey;
        pod_account.amount = 0u64.into(); // Start with 0 balance
        pod_account.delegate = spl_token_2022::pod::PodCOption::none(); // No delegate
        pod_account.state = AccountState::Initialized as u8; // Set to Initialized state
        pod_account.is_native = spl_token_2022::pod::PodCOption::none(); // Not a native token
        pod_account.delegated_amount = 0u64.into(); // No delegated amount
        pod_account.close_authority = spl_token_2022::pod::PodCOption::none(); // No close authority
    }

    Ok(())
}
