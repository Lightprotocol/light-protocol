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

// TODO: unit test.
/// Processes the create mint action by validating parameters and setting up the new address
pub fn process_create_mint_action(
    parsed_instruction_data: &ZMintActionCompressedInstructionData<'_>,
    validated_accounts: &MintActionAccounts,
    cpi_instruction_struct: &mut light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut<'_>,
    mint_size_config: &CompressedMintConfig,
) -> Result<(), ProgramError> {
    // 1. Create spl mint PDA using provided bump
    // - The compressed address is derived from the spl_mint_pda.
    // - The spl mint pda is used as mint in compressed token accounts.
    // Note: we cant use pinocchio_pubkey::derive_address because don't use the mint_pda in this ix.
    //  The pda would be unvalidated and an invalid bump could be used.
    let mint_signer = validated_accounts
        .mint_signer
        .ok_or(CTokenError::ExpectedMintSignerAccount)?;
    let spl_mint_pda: Pubkey = solana_pubkey::Pubkey::create_program_address(
        &[
            COMPRESSED_MINT_SEED,
            mint_signer.key().as_slice(),
            &[parsed_instruction_data.mint_bump],
        ],
        &crate::ID,
    )?
    .into();
    msg!("post mint_size_config {:?}", mint_size_config);
    if spl_mint_pda.to_bytes() != parsed_instruction_data.mint.spl_mint.to_bytes() {
        msg!("Invalid mint PDA derivation");
        return Err(ProgramError::InvalidAccountData);
    }
    // 2. Create NewAddressParams
    let address_merkle_tree_account_index =
        if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref() {
            cpi_context.in_tree_index
        } else {
            1 // Address tree is at index 1 after out_output_queue
        };
    cpi_instruction_struct.new_address_params[0].set(
        spl_mint_pda.to_bytes(),
        parsed_instruction_data.root_index.into(),
        Some(0),
        address_merkle_tree_account_index,
    );
    // Validate mint parameters
    if u64::from(parsed_instruction_data.mint.supply) != 0 {
        msg!("Initial supply must be 0 for new mint creation");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Validate version is supported
    if parsed_instruction_data.mint.version > 1 {
        msg!("Unsupported mint version");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Validate is_decompressed is false for new mint creation
    if parsed_instruction_data.mint.is_decompressed() {
        msg!("New mint must start as compressed (is_decompressed=false)");
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(())
}
