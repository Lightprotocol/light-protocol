use anchor_lang::prelude::ProgramError;
use borsh::BorshDeserialize;
use light_account_checks::AccountIterator;
use light_compressed_account::Pubkey;
use light_program_profiler::profile;
use light_token_interface::instructions::create_token_account::CreateTokenAccountInstructionData;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::{
    extensions::has_mint_extensions,
    shared::{
        create_compressible_account,
        initialize_ctoken_account::{initialize_ctoken_account, CTokenInitConfig},
        next_config_account,
    },
};

/// Process the create token account instruction
#[profile]
pub fn process_create_token_account(
    account_infos: &[AccountInfo],
    mut instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // SPL compatibility: if instruction_data is exactly 32 bytes, treat as owner-only (no compressible config)
    // This matches SPL Token's initialize_account3 which only sends the owner pubkey
    let inputs = if instruction_data.len() == 32 {
        let owner_bytes: [u8; 32] = instruction_data[..32]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        CreateTokenAccountInstructionData {
            owner: Pubkey::from(owner_bytes),
            compressible_config: None,
        }
    } else {
        CreateTokenAccountInstructionData::deserialize(&mut instruction_data)
            .map_err(ProgramError::from)?
    };

    let is_compressible = inputs.compressible_config.is_some();

    let mut iter = AccountIterator::new(account_infos);

    // For compressible accounts: token_account must be signer (account created via CPI)
    // For non-compressible accounts: token_account doesn't need to be signer (SPL compatibility)
    let token_account = if is_compressible {
        iter.next_signer_mut("token_account")?
    } else {
        iter.next_mut("token_account")?
    };
    let mint = iter.next_non_mut("mint")?;

    // Check which extensions the mint has (single deserialization)
    let mint_extensions = has_mint_extensions(mint)?;

    // Handle compressible vs non-compressible account creation
    let compressible_init_data = if let Some(ref compressible_config) = inputs.compressible_config {
        let payer = iter.next_signer_mut("payer")?;
        let config_account = next_config_account(&mut iter)?;
        let _system_program = iter.next_non_mut("system_program")?;
        let rent_payer = iter.next_mut("rent_payer")?;

        if let Some(compress_to_pubkey) = compressible_config.compress_to_account_pubkey.as_ref() {
            compress_to_pubkey.check_seeds(token_account.key())?;
        }

        // If restricted extensions exist, compression_only must be set
        if mint_extensions.has_restricted_extensions() && compressible_config.compression_only == 0
        {
            msg!("Mint has restricted extensions - compression_only must be set");
            return Err(anchor_compressed_token::ErrorCode::CompressionOnlyRequired.into());
        }

        // compression_only can only be set for mints with restricted extensions
        if compressible_config.compression_only != 0 && !mint_extensions.has_restricted_extensions()
        {
            msg!("compression_only can only be set for mints with restricted extensions");
            return Err(anchor_compressed_token::ErrorCode::CompressionOnlyNotAllowed.into());
        }

        Some(create_compressible_account(
            compressible_config,
            &mint_extensions,
            config_account,
            rent_payer,
            token_account,
            payer,
            None, // token_account is keypair signer
            false,
        )?)
    } else {
        // Non-compressible account: token_account must already exist and be owned by CToken program.
        // Unlike SPL initialize_account3 (which expects System-owned), this expects a pre-existing
        // CToken-owned account. Ownership is implicitly validated when writing to the account.
        None
    };

    // Initialize the token account
    initialize_ctoken_account(
        token_account,
        CTokenInitConfig {
            owner: &inputs.owner.to_bytes(),
            compressible: compressible_init_data,
            mint_extensions,
            mint_account: mint,
        },
    )
}
