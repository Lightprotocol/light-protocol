use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut;
use light_ctoken_types::{
    instructions::mint_action::ZMintActionCompressedInstructionData, CMINT_ADDRESS_TREE,
    COMPRESSED_MINT_SEED,
};
use light_program_profiler::profile;
use pinocchio::pubkey::pubkey_eq;
use spl_pod::solana_msg::msg;

/// Processes the create mint action by validating parameters and setting up the new address.
/// Note, the compressed output account creation is unified with other actions in a different function.
#[profile]
pub fn process_create_mint_action(
    parsed_instruction_data: &ZMintActionCompressedInstructionData<'_>,
    mint_signer: &pinocchio::pubkey::Pubkey,
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut<'_>,
    address_merkle_tree_account_index: u8,
) -> Result<(), ProgramError> {
    // 1. Create spl mint PDA using provided bump
    // - The compressed address is derived from the spl_mint_pda.
    // - The spl mint pda is used as mint in compressed token accounts.
    // Note: we cant use pinocchio_pubkey::derive_address because don't use the mint_pda in this ix.
    //  The pda would be unvalidated and an invalid bump could be used.
    let spl_mint_pda = solana_pubkey::Pubkey::create_program_address(
        &[
            COMPRESSED_MINT_SEED,
            mint_signer.as_slice(),
            &[parsed_instruction_data
                .create_mint
                .as_ref()
                .ok_or(ProgramError::InvalidInstructionData)?
                .mint_bump],
        ],
        &crate::ID,
    )?
    .to_bytes();

    if spl_mint_pda != parsed_instruction_data.mint.metadata.mint.to_bytes() {
        msg!("Invalid mint PDA derivation");
        return Err(ErrorCode::MintActionInvalidMintPda.into());
    }
    // With cpi context this program does not have access
    // to the address Merkle tree account that is used in the cpi to the light system program.
    // This breaks the implicit check of new_address_params_assigned.
    // -> Therefore we manually verify the compressed address derivation here.
    //
    // else is not required since for new_address_params_assigned
    // the light system program checks correct address derivation and we check the
    if let Some(cpi_context) = &parsed_instruction_data.cpi_context {
        if !pubkey_eq(&cpi_context.address_tree_pubkey, &CMINT_ADDRESS_TREE) {
            msg!("Invalid address tree pubkey in cpi context");
            return Err(ErrorCode::MintActionInvalidCpiContextAddressTreePubkey.into());
        }
        let address = light_compressed_account::address::derive_address(
            &spl_mint_pda,
            &cpi_context.address_tree_pubkey,
            &crate::LIGHT_CPI_SIGNER.program_id,
        );
        if address != parsed_instruction_data.compressed_address {
            msg!("Invalid compressed mint address derivation");
            return Err(ErrorCode::MintActionInvalidCompressedMintAddress.into());
        }
    }

    // 2. Create NewAddressParams
    cpi_instruction_struct.new_address_params[0].set(
        spl_mint_pda,
        parsed_instruction_data.root_index,
        Some(
            parsed_instruction_data
                .cpi_context
                .as_ref()
                .map(|ctx| ctx.assigned_account_index)
                .unwrap_or(0),
        ),
        address_merkle_tree_account_index,
    );
    // Validate mint parameters
    if parsed_instruction_data.mint.supply != 0 {
        msg!("Initial supply must be 0 for new mint creation");
        return Err(ErrorCode::MintActionInvalidInitialSupply.into());
    }

    // Validate version is supported
    // Version 3 (ShaFlat) is required for new mints because:
    // 1. Only SHA256 hashing is implemented for compressed mints
    // 2. Version 3 is consistent with TokenDataVersion::ShaFlat used for compressed token accounts
    if parsed_instruction_data.mint.metadata.version != 3 {
        msg!(
            "Unsupported mint version {}",
            parsed_instruction_data.mint.metadata.version
        );
        return Err(ErrorCode::MintActionUnsupportedVersion.into());
    }

    // Validate spl_mint_initialized is false for new mint creation
    if parsed_instruction_data.mint.metadata.spl_mint_initialized != 0 {
        msg!("New mint must start without SPL mint initialized");
        return Err(ErrorCode::MintActionInvalidCompressionState.into());
    }

    // Validate extensions - only TokenMetadata is supported and at most one extension allowed
    if let Some(extensions) = &parsed_instruction_data.mint.extensions {
        if extensions.len() != 1 {
            msg!(
                "Only one extension allowed for compressed mints, found {}",
                extensions.len()
            );
            return Err(ErrorCode::MintActionUnsupportedOperation.into());
        }
        // Extension type validation happens during allocation/creation
        // TokenMetadata is the only supported extension type
    }

    // Unchecked mint instruction data
    // 1. decimals
    // 2. mint authority
    // 3. freeze_authority

    Ok(())
}
