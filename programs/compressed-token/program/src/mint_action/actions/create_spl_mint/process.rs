use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_types::{
    instructions::mint_action::{ZCompressedMintInstructionData, ZCreateSplMintAction},
    CTokenError,
};
use light_program_profiler::profile;

use super::{
    create_mint_account, create_token_pool_account_manual, initialize_mint_account_for_action,
    initialize_token_pool_account_for_action,
};
use crate::mint_action::accounts::MintActionAccounts;

#[profile]
pub fn process_create_spl_mint_action(
    create_spl_action: &ZCreateSplMintAction<'_>,
    validated_accounts: &MintActionAccounts,
    mint_data: &ZCompressedMintInstructionData<'_>,
    token_pool_bump: u8,
) -> Result<(), ProgramError> {
    let executing_accounts = validated_accounts
        .executing
        .as_ref()
        .ok_or(ErrorCode::MintActionMissingExecutingAccounts)?;

    // Check mint authority if it exists
    // If no authority exists anyone should be able to create the associated spl mint.
    if let Some(ix_data_mint_authority) = mint_data.mint_authority {
        if *validated_accounts.authority.key() != ix_data_mint_authority.to_bytes() {
            return Err(ErrorCode::MintActionInvalidMintAuthority.into());
        }
    }

    // Verify mint PDA matches the mint field in compressed mint inputs
    let expected_mint: [u8; 32] = mint_data.metadata.mint.to_bytes();
    if executing_accounts
        .mint
        .ok_or(ErrorCode::MintActionMissingMintAccount)?
        .key()
        != &expected_mint
    {
        return Err(ErrorCode::MintActionInvalidMintPda.into());
    }

    // 1. Create the mint account manually (PDA derived from our program, owned by token program)
    let mint_signer = validated_accounts
        .mint_signer
        .ok_or(CTokenError::ExpectedMintSignerAccount)?;
    create_mint_account(
        executing_accounts,
        &crate::LIGHT_CPI_SIGNER.program_id,
        create_spl_action.mint_bump,
        mint_signer,
    )?;

    // 2. Initialize the mint account using Token-2022's initialize_mint2 instruction
    initialize_mint_account_for_action(executing_accounts, mint_data)?;

    // 3. Create the token pool account manually (PDA derived from our program, owned by token program)
    create_token_pool_account_manual(
        executing_accounts,
        &crate::LIGHT_CPI_SIGNER.program_id,
        token_pool_bump,
    )?;

    // 4. Initialize the token pool account
    initialize_token_pool_account_for_action(executing_accounts)?;

    // 5. Mint the existing supply to the token pool if there's any supply
    if mint_data.supply > 0 {
        crate::shared::mint_to_token_pool(
            executing_accounts
                .mint
                .ok_or(ErrorCode::MintActionMissingMintAccount)?,
            executing_accounts
                .token_pool_pda
                .ok_or(ErrorCode::MintActionMissingTokenPoolAccount)?,
            executing_accounts
                .token_program
                .ok_or(ErrorCode::MintActionMissingTokenProgram)?,
            executing_accounts.system.cpi_authority_pda,
            u64::from(mint_data.supply),
        )?;
    }

    Ok(())
}
