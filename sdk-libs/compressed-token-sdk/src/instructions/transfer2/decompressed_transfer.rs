use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_program_error::ProgramError;

use crate::{
    account2::{
        create_ctoken_to_spl_transfer_instruction, create_spl_to_ctoken_transfer_instruction,
    },
    error::TokenSdkError,
};

/// Transfer SPL tokens to compressed tokens
///
/// This function creates the instruction and immediately invokes it.
/// Similar to SPL Token's transfer wrapper functions.
pub fn transfer_spl_to_ctoken<'info>(
    payer: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    source_spl_token_account: AccountInfo<'info>,
    destination_ctoken_account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    spl_token_program: AccountInfo<'info>,
    compressed_token_pool_pda: AccountInfo<'info>,
    compressed_token_pool_pda_bump: u8,
    compressed_token_program_authority: AccountInfo<'info>,
    amount: u64,
) -> Result<(), ProgramError> {
    let instruction = create_spl_to_ctoken_transfer_instruction(
        *payer.key,
        *authority.key,
        *source_spl_token_account.key,
        *destination_ctoken_account.key,
        *mint.key,
        *spl_token_program.key,
        *compressed_token_pool_pda.key,
        compressed_token_pool_pda_bump,
        amount,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    // let mut account_infos = remaining_accounts.to_vec();
    let account_infos = vec![
        authority.clone(),
        compressed_token_program_authority,
        mint,                       // Index 0: Mint
        destination_ctoken_account, // Index 1: Destination owner
        authority,                  // Index 2: Authority (signer)
        source_spl_token_account,   // Index 3: Source SPL token account
        compressed_token_pool_pda,  // Index 4: Token pool PDA
        spl_token_program,          // Index 5: SPL Token program
    ];

    invoke(&instruction, &account_infos)?;
    Ok(())
}

// TODO: must test this.
/// Transfer SPL tokens to compressed tokens via CPI signer.
///
/// This function creates the instruction and invokes it with the provided
/// signer seeds.
pub fn transfer_spl_to_ctoken_signed<'info>(
    payer: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    source_spl_token_account: AccountInfo<'info>,
    destination_ctoken_account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    spl_token_program: AccountInfo<'info>,
    compressed_token_pool_pda: AccountInfo<'info>,
    compressed_token_pool_pda_bump: u8,
    compressed_token_program_authority: AccountInfo<'info>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = create_spl_to_ctoken_transfer_instruction(
        *payer.key,
        *authority.key,
        *source_spl_token_account.key,
        *destination_ctoken_account.key,
        *mint.key,
        *spl_token_program.key,
        *compressed_token_pool_pda.key,
        compressed_token_pool_pda_bump,
        amount,
    )
    .map_err(|_| TokenSdkError::MethodUsed)?;

    let account_infos = vec![
        payer.clone(),
        compressed_token_program_authority,
        mint,                       // Index 0: Mint
        destination_ctoken_account, // Index 1: Destination owner
        authority,                  // Index 2: Authority (signer)
        source_spl_token_account,   // Index 3: Source SPL token account
        compressed_token_pool_pda,  // Index 4: Token pool PDA
        spl_token_program,          // Index 5: SPL Token program
    ];

    invoke_signed(&instruction, &account_infos, signer_seeds)
        .map_err(|_| TokenSdkError::MethodUsed)?;
    Ok(())
}

// TODO: TEST.
/// Transfer compressed tokens to SPL tokens
///
/// This function creates the instruction and invokes it.
pub fn transfer_ctoken_to_spl<'info>(
    payer: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    source_ctoken_account: AccountInfo<'info>,
    destination_spl_token_account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    spl_token_program: AccountInfo<'info>,
    compressed_token_pool_pda: AccountInfo<'info>,
    compressed_token_pool_pda_bump: u8,
    compressed_token_program_authority: AccountInfo<'info>,
    amount: u64,
) -> Result<(), ProgramError> {
    let instruction = create_ctoken_to_spl_transfer_instruction(
        *payer.key,
        *authority.key,
        *source_ctoken_account.key,
        *destination_spl_token_account.key,
        *mint.key,
        *spl_token_program.key,
        *compressed_token_pool_pda.key,
        compressed_token_pool_pda_bump,
        amount,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    let account_infos = vec![
        authority.clone(),
        compressed_token_program_authority,
        mint,                          // Index 0: Mint
        destination_spl_token_account, // Index 1: Destination owner
        authority,                     // Index 2: Authority (signer)
        source_ctoken_account,         // Index 3: Source SPL token account
        compressed_token_pool_pda,     // Index 4: Token pool PDA
        spl_token_program,             // Index 5: SPL Token program
    ];

    invoke(&instruction, &account_infos)?;
    Ok(())
}

/// Transfer compressed tokens to SPL tokens via CPI signer.
///
/// This function creates the instruction and invokes it with the provided
/// signer seeds.
pub fn transfer_ctoken_to_spl_signed<'info>(
    payer: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    source_ctoken_account: AccountInfo<'info>,
    destination_spl_token_account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    spl_token_program: AccountInfo<'info>,
    compressed_token_pool_pda: AccountInfo<'info>,
    compressed_token_pool_pda_bump: u8,
    compressed_token_program_authority: AccountInfo<'info>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = create_ctoken_to_spl_transfer_instruction(
        *payer.key,
        *authority.key,
        *source_ctoken_account.key,
        *destination_spl_token_account.key,
        *mint.key,
        *spl_token_program.key,
        *compressed_token_pool_pda.key,
        compressed_token_pool_pda_bump,
        amount,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    let account_infos = vec![
        payer.clone(),
        compressed_token_program_authority,
        mint,                          // Index 0: Mint
        destination_spl_token_account, // Index 1: Destination owner
        authority,                     // Index 2: Authority (signer)
        source_ctoken_account,         // Index 3: Source SPL token account
        compressed_token_pool_pda,     // Index 4: Token pool PDA
        spl_token_program,             // Index 5: SPL Token program
    ];

    invoke_signed(&instruction, &account_infos, signer_seeds)?;
    Ok(())
}
