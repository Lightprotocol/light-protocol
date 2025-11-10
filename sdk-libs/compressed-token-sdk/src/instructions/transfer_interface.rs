use light_compressed_account::instruction_data::compressed_proof::borsh_compat::ValidityProof;
use light_ctoken_types::instructions::transfer2::{Compression, MultiTokenTransferOutputData};
use light_program_profiler::profile;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::transfer_ctoken::{transfer_ctoken, transfer_ctoken_signed};
use crate::{
    account2::CTokenAccount2,
    error::TokenSdkError,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Config,
        Transfer2Inputs,
    },
    utils::is_ctoken_account,
};

#[allow(clippy::too_many_arguments)]
#[profile]
pub fn create_transfer_spl_to_ctoken_instruction(
    source_spl_token_account: Pubkey,
    to: Pubkey,
    amount: u64,
    authority: Pubkey,
    mint: Pubkey,
    payer: Pubkey,
    token_pool_pda: Pubkey,
    token_pool_pda_bump: u8,
    spl_token_program: Pubkey,
) -> Result<Instruction, TokenSdkError> {
    let packed_accounts = vec![
        // Mint (index 0)
        AccountMeta::new_readonly(mint, false),
        // Destination token account (index 1)
        AccountMeta::new(to, false),
        // Authority for compression (index 2) - signer
        AccountMeta::new_readonly(authority, true),
        // Source SPL token account (index 3) - writable
        AccountMeta::new(source_spl_token_account, false),
        // Token pool PDA (index 4) - writable
        AccountMeta::new(token_pool_pda, false),
        // SPL Token program (index 5) - needed for CPI
        AccountMeta::new_readonly(spl_token_program, false),
    ];

    let wrap_spl_to_ctoken_account = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::compress_spl(
            amount,
            0, // mint
            3, // source or recpient
            2, // authority
            4, // pool_account_index:
            0, // pool_index
            token_pool_pda_bump,
        )),
        delegate_is_set: false,
        method_used: true,
    };

    let ctoken_account = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::decompress_ctoken(amount, 0, 1)),
        delegate_is_set: false,
        method_used: true,
    };

    // Create Transfer2Inputs following the test
    let inputs = Transfer2Inputs {
        validity_proof: ValidityProof::new(None).into(),
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig::new_decompressed_accounts_only(
            payer,
            packed_accounts,
        ),
        in_lamports: None,
        out_lamports: None,
        token_accounts: vec![wrap_spl_to_ctoken_account, ctoken_account],
        output_queue: 0, // Decompressed accounts only, no output queue needed
    };

    create_transfer2_instruction(inputs)
}

#[allow(clippy::too_many_arguments)]
#[profile]
pub fn create_transfer_ctoken_to_spl_instruction(
    source_ctoken_account: Pubkey,
    destination_spl_token_account: Pubkey,
    amount: u64,
    authority: Pubkey,
    mint: Pubkey,
    payer: Pubkey,
    token_pool_pda: Pubkey,
    token_pool_pda_bump: u8,
    spl_token_program: Pubkey,
) -> Result<Instruction, TokenSdkError> {
    let packed_accounts = vec![
        // Mint (index 0)
        AccountMeta::new_readonly(mint, false),
        // Source ctoken account (index 1) - writable
        AccountMeta::new(source_ctoken_account, false),
        // Destination SPL token account (index 2) - writable
        AccountMeta::new(destination_spl_token_account, false),
        // Authority (index 3) - signer
        AccountMeta::new_readonly(authority, true),
        // Token pool PDA (index 4) - writable
        AccountMeta::new(token_pool_pda, false),
        // SPL Token program (index 5) - needed for CPI
        AccountMeta::new_readonly(spl_token_program, false),
    ];

    // First operation: compress from ctoken account to pool using compress_spl
    let compress_to_pool = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::compress_ctoken(
            amount, 0, // mint index
            1, // source ctoken account index
            3, // authority index
        )),
        delegate_is_set: false,
        method_used: true,
    };

    // Second operation: decompress from pool to SPL token account using decompress_spl
    let decompress_to_spl = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::decompress_spl(
            amount,
            0, // mint index
            2, // destination SPL token account index
            4, // pool_account_index
            0, // pool_index (TODO: make dynamic)
            token_pool_pda_bump,
        )),
        delegate_is_set: false,
        method_used: true,
    };

    let inputs = Transfer2Inputs {
        validity_proof: ValidityProof::new(None).into(),
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig::new_decompressed_accounts_only(
            payer,
            packed_accounts,
        ),
        in_lamports: None,
        out_lamports: None,
        token_accounts: vec![compress_to_pool, decompress_to_spl],
        output_queue: 0, // Decompressed accounts only, no output queue needed
    };

    create_transfer2_instruction(inputs)
}

/// Transfer SPL tokens to compressed tokens
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
    let instruction = create_transfer_spl_to_ctoken_instruction(
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
        payer,
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
/// Transfer SPL tokens to compressed tokens via CPI signer
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
    let instruction = create_transfer_spl_to_ctoken_instruction(
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

    let account_infos = vec![
        payer,
        compressed_token_program_authority,
        mint,                       // Index 0: Mint
        destination_ctoken_account, // Index 1: Destination owner
        authority,                  // Index 2: Authority (signer)
        source_spl_token_account,   // Index 3: Source SPL token account
        compressed_token_pool_pda,  // Index 4: Token pool PDA
        spl_token_program,          // Index 5: SPL Token program
    ];

    invoke_signed(&instruction, &account_infos, signer_seeds)?;
    Ok(())
}

// TODO: TEST.
/// Transfer compressed tokens to SPL tokens
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
    let instruction = create_transfer_ctoken_to_spl_instruction(
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
        payer,
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

/// Transfer compressed tokens to SPL tokens via CPI signer
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
    let instruction = create_transfer_ctoken_to_spl_instruction(
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
        payer,
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

/// Unified transfer interface for ctoken<->ctoken and ctoken<->spl transfers
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
    let source_is_ctoken =
        is_ctoken_account(source_account).map_err(|_| ProgramError::InvalidAccountData)?;
    let dest_is_ctoken =
        is_ctoken_account(destination_account).map_err(|_| ProgramError::InvalidAccountData)?;

    match (source_is_ctoken, dest_is_ctoken) {
        (true, true) => transfer_ctoken(source_account, destination_account, authority, amount),

        (true, false) => {
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

        (false, true) => {
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

/// Unified transfer interface with CPI
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
        (true, true) => transfer_ctoken_signed(
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
