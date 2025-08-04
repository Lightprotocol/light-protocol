use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::{hash_cache::HashCache, instructions::mint_to_compressed::ZMintToAction};

use spl_pod::solana_msg::msg;

use crate::{
    mint_action::accounts::MintActionAccounts,
    shared::{mint_to_token_pool, token_output::set_output_compressed_account},
};

pub fn process_mint_to_action(
    action: &ZMintToAction,
    current_supply: u64,
    validated_accounts: &MintActionAccounts,
    accounts_config: &crate::mint_action::accounts::AccountsConfig,
    cpi_instruction_struct: &mut light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut,
    hash_cache: &mut HashCache,
    mint: Pubkey,
    out_token_queue_index: u8,
) -> Result<u64, ProgramError> {
    let sum_amounts = action
        .recipients
        .iter()
        .map(|x| u64::from(x.amount))
        .sum::<u64>();
    let updated_supply = current_supply
        .checked_add(sum_amounts)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if let Some(system_accounts) = validated_accounts.executing.as_ref() {
        // If mint is decompressed, mint tokens to the token pool to maintain SPL mint supply consistency
        if accounts_config.is_decompressed {
            let sum_amounts: u64 = action.recipients.iter().map(|x| u64::from(x.amount)).sum();
            let mint_account = system_accounts
                .mint
                .ok_or(ProgramError::InvalidAccountData)?;
            let token_pool_account = system_accounts
                .token_pool_pda
                .ok_or(ProgramError::InvalidAccountData)?;
            let token_program = system_accounts
                .token_program
                .ok_or(ProgramError::InvalidAccountData)?;
            msg!("minting {}", sum_amounts);
            mint_to_token_pool(
                mint_account,
                token_pool_account,
                token_program,
                validated_accounts.cpi_authority()?,
                sum_amounts,
            )?;
        }
        // Create output token accounts
        create_output_compressed_token_accounts(
            action,
            cpi_instruction_struct,
            hash_cache,
            mint,
            out_token_queue_index,
        )?;
    }

    Ok(updated_supply)
}

fn create_output_compressed_token_accounts(
    parsed_instruction_data: &ZMintToAction<'_>,
    cpi_instruction_struct: &mut light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut<'_>,
    hash_cache: &mut HashCache,
    mint: Pubkey,
    queue_pubkey_index: u8,
) -> Result<(), ProgramError> {
    let hashed_mint = hash_cache.get_or_hash_mint(&mint.to_bytes())?;

    let lamports = parsed_instruction_data
        .lamports
        .map(|lamports| u64::from(*lamports));
    for (recipient, output_account) in parsed_instruction_data.recipients.iter().zip(
        cpi_instruction_struct
            .output_compressed_accounts
            .iter_mut()
            .skip(1), // Skip the first account which is the mint account.
    ) {
        let output_delegate = None;
        set_output_compressed_account::<false>(
            output_account,
            hash_cache,
            recipient.recipient,
            output_delegate,
            recipient.amount,
            lamports,
            mint,
            &hashed_mint,
            queue_pubkey_index,
            parsed_instruction_data.token_account_version,
        )?;
    }
    Ok(())
}
