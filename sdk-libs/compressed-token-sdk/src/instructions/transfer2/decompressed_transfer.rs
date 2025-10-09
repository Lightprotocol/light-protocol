use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_program_error::ProgramError;

use crate::{
    account2::{
        create_ctoken_to_spl_transfer_instruction, create_spl_to_ctoken_transfer_instruction,
    },
    error::TokenSdkError,
    utils::is_ctoken_account,
};

use super::super::decompressed_transfer::{
    transfer as ctoken_transfer, transfer_signed as ctoken_transfer_signed,
};

/// Transfer SPL tokens to compressed tokens
///
/// This function creates the instruction and immediately invokes it.
/// Similar to SPL Token's transfer wrapper functions.
#[allow(clippy::too_many_arguments)]
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
        *source_spl_token_account.key,
        *destination_ctoken_account.key,
        amount,
        *authority.key,
        *mint.key,
        *payer.key,
        *compressed_token_pool_pda.key,
        compressed_token_pool_pda_bump,
        *spl_token_program.key,
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
#[allow(clippy::too_many_arguments)]
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
        *source_spl_token_account.key,
        *destination_ctoken_account.key,
        amount,
        *authority.key,
        *mint.key,
        *payer.key,
        *compressed_token_pool_pda.key,
        compressed_token_pool_pda_bump,
        *spl_token_program.key,
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
#[allow(clippy::too_many_arguments)]
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
        *source_ctoken_account.key,
        *destination_spl_token_account.key,
        amount,
        *authority.key,
        *mint.key,
        *payer.key,
        *compressed_token_pool_pda.key,
        compressed_token_pool_pda_bump,
        *spl_token_program.key,
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
#[allow(clippy::too_many_arguments)]
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
        *source_ctoken_account.key,
        *destination_spl_token_account.key,
        amount,
        *authority.key,
        *mint.key,
        *payer.key,
        *compressed_token_pool_pda.key,
        compressed_token_pool_pda_bump,
        *spl_token_program.key,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    let account_infos = vec![
        payer.clone(),
        compressed_token_program_authority,
        mint,                          // Index 0: Mint
        destination_spl_token_account, // Index 1: Destination owner
        authority,                     // Index 2: Authority (signer)
        source_ctoken_account,         // Index 3: Source SPL token account
                                       // compressed_token_pool_pda,     // Index 4: Token pool PDA
                                       // spl_token_program,             // Index 5: SPL Token program
    ];

    invoke_signed(&instruction, &account_infos, signer_seeds)?;
    Ok(())
}

/// Unified transfer interface that automatically handles both ctoken<->ctoken and ctoken<->spl transfers
///
/// This function inspects the source and destination accounts to determine the transfer type
/// and validates that the correct optional parameters are provided.
///
/// # Arguments
/// * `source_account` - Source token account (can be ctoken or SPL)
/// * `destination_account` - Destination token account (can be ctoken or SPL)
/// * `authority` - Authority for the transfer (must be signer)
/// * `amount` - Amount to transfer
/// * `payer` - Payer for the transaction
/// * `compressed_token_program_authority` - Compressed token program authority
/// * `mint` - Optional mint account (required for SPL<->ctoken transfers)
/// * `spl_token_program` - Optional SPL token program (required for SPL<->ctoken transfers)
/// * `compressed_token_pool_pda` - Optional token pool PDA (required for SPL<->ctoken transfers)
/// * `compressed_token_pool_pda_bump` - Optional bump seed for token pool PDA
///
/// # Errors
/// * `SplBridgeConfigRequired` - If transferring to/from SPL without required accounts
/// * `UseRegularSplTransfer` - If both source and destination are SPL accounts
/// * `CannotDetermineAccountType` - If account type cannot be determined
#[allow(clippy::too_many_arguments)]
pub fn transfer_interface<'info>(
    source_account: &AccountInfo<'info>,
    destination_account: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
    payer: &AccountInfo<'info>,
    compressed_token_program_authority: &AccountInfo<'info>,
    mint: Option<&AccountInfo<'info>>,
    spl_token_program: Option<&AccountInfo<'info>>,
    compressed_token_pool_pda: Option<&AccountInfo<'info>>,
    compressed_token_pool_pda_bump: Option<u8>,
) -> Result<(), ProgramError> {
    // Determine account types
    let source_is_ctoken =
        is_ctoken_account(source_account).map_err(|_| ProgramError::InvalidAccountData)?;
    let dest_is_ctoken =
        is_ctoken_account(destination_account).map_err(|_| ProgramError::InvalidAccountData)?;

    match (source_is_ctoken, dest_is_ctoken) {
        // ctoken -> ctoken: Direct transfer (bridge accounts not needed)
        (true, true) => ctoken_transfer(source_account, destination_account, authority, amount),

        // ctoken -> spl: Requires bridge accounts
        (true, false) => {
            // Validate all required accounts are provided
            let (mint_acct, spl_program, pool_pda, bump) = match (
                mint,
                spl_token_program,
                compressed_token_pool_pda,
                compressed_token_pool_pda_bump,
            ) {
                (Some(m), Some(p), Some(pd), Some(b)) => (m, p, pd, b),
                _ => {
                    return Err(ProgramError::Custom(
                        TokenSdkError::IncompleteSplBridgeConfig.into(),
                    ))
                }
            };

            transfer_ctoken_to_spl(
                payer.clone(),
                authority.clone(),
                source_account.clone(),
                destination_account.clone(),
                mint_acct.clone(),
                spl_program.clone(),
                pool_pda.clone(),
                bump,
                compressed_token_program_authority.clone(),
                amount,
            )
        }

        // spl -> ctoken: Requires bridge accounts
        (false, true) => {
            // Validate all required accounts are provided
            let (mint_acct, spl_program, pool_pda, bump) = match (
                mint,
                spl_token_program,
                compressed_token_pool_pda,
                compressed_token_pool_pda_bump,
            ) {
                (Some(m), Some(p), Some(pd), Some(b)) => (m, p, pd, b),
                _ => {
                    return Err(ProgramError::Custom(
                        TokenSdkError::IncompleteSplBridgeConfig.into(),
                    ))
                }
            };

            transfer_spl_to_ctoken(
                payer.clone(),
                authority.clone(),
                source_account.clone(),
                destination_account.clone(),
                mint_acct.clone(),
                spl_program.clone(),
                pool_pda.clone(),
                bump,
                compressed_token_program_authority.clone(),
                amount,
            )
        }

        // spl -> spl: Not supported
        (false, false) => Err(ProgramError::Custom(
            TokenSdkError::UseRegularSplTransfer.into(),
        )),
    }
}

/// Unified transfer interface with signer seeds for CPI
///
/// Same as `transfer_interface` but uses invoke_signed for CPI calls
#[allow(clippy::too_many_arguments)]
pub fn transfer_interface_signed<'info>(
    source_account: &AccountInfo<'info>,
    destination_account: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
    payer: &AccountInfo<'info>,
    compressed_token_program_authority: &AccountInfo<'info>,
    mint: Option<&AccountInfo<'info>>,
    spl_token_program: Option<&AccountInfo<'info>>,
    compressed_token_pool_pda: Option<&AccountInfo<'info>>,
    compressed_token_pool_pda_bump: Option<u8>,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    // Determine account types
    let source_is_ctoken =
        is_ctoken_account(source_account).map_err(|_| ProgramError::InvalidAccountData)?;
    let dest_is_ctoken =
        is_ctoken_account(destination_account).map_err(|_| ProgramError::InvalidAccountData)?;

    match (source_is_ctoken, dest_is_ctoken) {
        // ctoken -> ctoken: Direct transfer (bridge accounts not needed)
        (true, true) => ctoken_transfer_signed(
            source_account,
            destination_account,
            authority,
            amount,
            signer_seeds,
        ),

        // ctoken -> spl: Requires bridge accounts
        (true, false) => {
            // Validate all required accounts are provided
            let (mint_acct, spl_program, pool_pda, bump) = match (
                mint,
                spl_token_program,
                compressed_token_pool_pda,
                compressed_token_pool_pda_bump,
            ) {
                (Some(m), Some(p), Some(pd), Some(b)) => (m, p, pd, b),
                _ => {
                    return Err(ProgramError::Custom(
                        TokenSdkError::IncompleteSplBridgeConfig.into(),
                    ))
                }
            };

            transfer_ctoken_to_spl_signed(
                payer.clone(),
                authority.clone(),
                source_account.clone(),
                destination_account.clone(),
                mint_acct.clone(),
                spl_program.clone(),
                pool_pda.clone(),
                bump,
                compressed_token_program_authority.clone(),
                amount,
                signer_seeds,
            )
        }

        // spl -> ctoken: Requires bridge accounts
        (false, true) => {
            // Validate all required accounts are provided
            let (mint_acct, spl_program, pool_pda, bump) = match (
                mint,
                spl_token_program,
                compressed_token_pool_pda,
                compressed_token_pool_pda_bump,
            ) {
                (Some(m), Some(p), Some(pd), Some(b)) => (m, p, pd, b),
                _ => {
                    return Err(ProgramError::Custom(
                        TokenSdkError::IncompleteSplBridgeConfig.into(),
                    ))
                }
            };

            transfer_spl_to_ctoken_signed(
                payer.clone(),
                authority.clone(),
                source_account.clone(),
                destination_account.clone(),
                mint_acct.clone(),
                spl_program.clone(),
                pool_pda.clone(),
                bump,
                compressed_token_program_authority.clone(),
                amount,
                signer_seeds,
            )
        }

        // spl -> spl: Not supported
        (false, false) => Err(ProgramError::Custom(
            TokenSdkError::UseRegularSplTransfer.into(),
        )),
    }
}
