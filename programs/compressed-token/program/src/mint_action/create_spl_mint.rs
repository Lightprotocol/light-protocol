use anchor_lang::solana_program::program_error::ProgramError;
use arrayvec::ArrayVec;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_compressed_account::{
    instruction_data::with_readonly::{
        InstructionDataInvokeCpiWithReadOnly, InstructionDataInvokeCpiWithReadOnlyConfig,
    },
    Pubkey,
};
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::{
        mint_actions::{
            MintActionCompressedInstructionData, ZAction, ZMintActionCompressedInstructionData,
        },
        mint_to_compressed::ZMintToAction,
    },
    state::{CompressedMint, CompressedMintConfig},
    CTokenError, COMPRESSED_MINT_SEED,
};
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;
use spl_token::solana_program::log::sol_log_compute_units;

use light_hasher::{Hasher, Poseidon, Sha256};

use crate::mint_action::accounts::determine_accounts_config;
use crate::mint_action::zero_copy_config::get_zero_copy_configs;
use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    create_spl_mint::processor::{
        create_mint_account, create_token_pool_account_manual, initialize_mint_account_for_action,
        initialize_token_pool_account_for_action,
    },
    extensions::processor::create_extension_hash_chain,
    mint::mint_output::create_output_compressed_mint_account,
    mint_action::accounts::MintActionAccounts,
    shared::{
        cpi::execute_cpi_invoke,
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
        mint_to_token_pool,
        token_output::set_output_compressed_account,
    },
};

/// Helper function for processing CreateSplMint action
pub fn process_create_spl_mint_action(
    create_spl_action: &light_ctoken_types::instructions::mint_actions::ZCreateSplMintAction<'_>,
    validated_accounts: &MintActionAccounts,
    mint_data: &light_ctoken_types::instructions::create_compressed_mint::ZCompressedMintInstructionData<'_>,
) -> Result<(), ProgramError> {
    let executing_accounts = validated_accounts
        .executing
        .as_ref()
        .ok_or(ProgramError::InvalidAccountData)?;

    // Check mint authority if it exists
    if let Some(ix_data_mint_authority) = mint_data.mint_authority {
        if *validated_accounts.authority.key() != ix_data_mint_authority.to_bytes() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // Verify mint PDA matches the spl_mint field in compressed mint inputs
    let expected_mint: [u8; 32] = mint_data.spl_mint.to_bytes();
    if executing_accounts
        .mint
        .ok_or(ProgramError::InvalidAccountData)?
        .key()
        != &expected_mint
    {
        return Err(ProgramError::InvalidAccountData);
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
    create_token_pool_account_manual(executing_accounts, &crate::LIGHT_CPI_SIGNER.program_id)?;

    // 4. Initialize the token pool account
    initialize_token_pool_account_for_action(executing_accounts)?;

    // 5. Mint the existing supply to the token pool if there's any supply
    if mint_data.supply > 0 {
        crate::shared::mint_to_token_pool(
            executing_accounts
                .mint
                .ok_or(ProgramError::InvalidAccountData)?,
            executing_accounts
                .token_pool_pda
                .ok_or(ProgramError::InvalidAccountData)?,
            executing_accounts
                .token_program
                .ok_or(ProgramError::InvalidAccountData)?,
            executing_accounts.system.cpi_authority_pda,
            mint_data.supply.into(),
        )?;
    }

    Ok(())
}
