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

/// Helper function for processing authority update actions
pub fn update_authority(
    update_action: &light_ctoken_types::instructions::mint_actions::ZUpdateAuthority<'_>,
    signer_key: &pinocchio::pubkey::Pubkey,
    current_authority: Option<Pubkey>,
    authority_name: &str,
) -> Result<Option<Pubkey>, ProgramError> {
    // Verify that the signer is the current authority
    let current_authority_pubkey = current_authority.ok_or(ProgramError::InvalidArgument)?;
    if *signer_key != current_authority_pubkey.to_bytes() {
        msg!(
            "Invalid authority: signer does not match current {}",
            authority_name
        );
        return Err(ProgramError::InvalidArgument);
    }

    // Update the authority (None = revoke, Some(key) = set new authority)
    Ok(update_action.new_authority.as_ref().map(|auth| **auth))
}
