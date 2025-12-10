//! Instruction builders for registry program operations.
//!
//! This module provides instruction builders that don't depend on the
//! `light_registry` program crate, using manual account metas and discriminators.

use borsh::BorshSerialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use super::types::{
    get_epoch_pda_address, get_forester_epoch_pda, get_forester_pda,
    get_protocol_config_pda_address, CompressAndCloseIndices, ForesterConfig, REGISTRY_PROGRAM_ID,
};

// Anchor discriminators (first 8 bytes of sha256("global:<instruction_name>"))
// These are computed from the instruction names in the registry program

/// Discriminator for `register_forester` instruction
const REGISTER_FORESTER_DISCRIMINATOR: [u8; 8] = [5, 70, 186, 53, 55, 89, 245, 238];

/// Discriminator for `register_forester_epoch` instruction
const REGISTER_FORESTER_EPOCH_DISCRIMINATOR: [u8; 8] = [34, 248, 241, 159, 109, 178, 224, 25];

/// Discriminator for `finalize_registration` instruction
const FINALIZE_REGISTRATION_DISCRIMINATOR: [u8; 8] = [181, 8, 173, 35, 5, 84, 85, 53];

/// Discriminator for `claim` instruction
const CLAIM_DISCRIMINATOR: [u8; 8] = [62, 198, 214, 193, 213, 159, 108, 210];

/// Discriminator for `compress_and_close` instruction
const COMPRESS_AND_CLOSE_DISCRIMINATOR: [u8; 8] = [41, 40, 22, 20, 119, 135, 209, 29];

// ============================================================================
// Instruction Data Structures
// ============================================================================

#[derive(BorshSerialize)]
struct RegisterForesterData {
    _bump: u8,
    authority: Pubkey,
    config: ForesterConfig,
    weight: Option<u64>,
}

#[derive(BorshSerialize)]
struct RegisterForesterEpochData {
    epoch: u64,
}

#[derive(BorshSerialize)]
struct CompressAndCloseData {
    authority_index: u8,
    destination_index: u8,
    indices: Vec<CompressAndCloseIndices>,
}

// ============================================================================
// Instruction Builders
// ============================================================================

/// Creates a register forester instruction
pub fn create_register_forester_instruction(
    fee_payer: &Pubkey,
    governance_authority: &Pubkey,
    forester_authority: &Pubkey,
    config: ForesterConfig,
) -> Instruction {
    let (forester_pda, bump) = get_forester_pda(forester_authority);
    let (protocol_config_pda, _) = get_protocol_config_pda_address();

    let instruction_data = RegisterForesterData {
        _bump: bump,
        authority: *forester_authority,
        config,
        weight: Some(1),
    };

    let mut data = Vec::with_capacity(8 + 200);
    data.extend_from_slice(&REGISTER_FORESTER_DISCRIMINATOR);
    instruction_data.serialize(&mut data).unwrap();

    let accounts = vec![
        AccountMeta::new(forester_pda, false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new_readonly(protocol_config_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
    ];

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts,
        data,
    }
}

/// Creates a register forester epoch PDA instruction
pub fn create_register_forester_epoch_pda_instruction(
    authority: &Pubkey,
    derivation: &Pubkey,
    epoch: u64,
) -> Instruction {
    let (forester_epoch_pda, _) = get_forester_epoch_pda(&get_forester_pda(derivation).0, epoch);
    let (forester_pda, _) = get_forester_pda(derivation);
    let epoch_pda = get_epoch_pda_address(epoch);
    let protocol_config_pda = get_protocol_config_pda_address().0;

    let instruction_data = RegisterForesterEpochData { epoch };

    let mut data = Vec::with_capacity(8 + 8);
    data.extend_from_slice(&REGISTER_FORESTER_EPOCH_DISCRIMINATOR);
    instruction_data.serialize(&mut data).unwrap();

    let accounts = vec![
        AccountMeta::new(*authority, true),
        AccountMeta::new(forester_epoch_pda, false),
        AccountMeta::new_readonly(forester_pda, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new(epoch_pda, false),
        AccountMeta::new_readonly(protocol_config_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
    ];

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts,
        data,
    }
}

/// Creates a finalize registration instruction
pub fn create_finalize_registration_instruction(
    authority: &Pubkey,
    derivation: &Pubkey,
    epoch: u64,
) -> Instruction {
    let (forester_epoch_pda, _) = get_forester_epoch_pda(&get_forester_pda(derivation).0, epoch);
    let epoch_pda = get_epoch_pda_address(epoch);

    // FinalizeRegistration has no additional data
    let data = FINALIZE_REGISTRATION_DISCRIMINATOR.to_vec();

    let accounts = vec![
        AccountMeta::new(forester_epoch_pda, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(epoch_pda, false),
    ];

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts,
        data,
    }
}

/// Creates a claim instruction via the registry program
pub fn create_claim_instruction(
    authority: &Pubkey,
    registered_forester_pda: Pubkey,
    rent_sponsor: Pubkey,
    compression_authority: Pubkey,
    compressible_config: Pubkey,
    compressed_token_program: Pubkey,
    token_accounts: &[Pubkey],
) -> Instruction {
    // Claim has no additional data beyond discriminator
    let data = CLAIM_DISCRIMINATOR.to_vec();

    let mut accounts = vec![
        AccountMeta::new(*authority, true),
        AccountMeta::new(registered_forester_pda, false),
        AccountMeta::new(rent_sponsor, false),
        AccountMeta::new_readonly(compression_authority, false),
        AccountMeta::new_readonly(compressible_config, false),
        AccountMeta::new_readonly(compressed_token_program, false),
    ];

    // Add token accounts as remaining accounts
    for token_account in token_accounts {
        accounts.push(AccountMeta::new(*token_account, false));
    }

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts,
        data,
    }
}

/// Creates a compress and close instruction via the registry program
#[allow(clippy::too_many_arguments)]
pub fn create_compress_and_close_instruction(
    authority: &Pubkey,
    registered_forester_pda: Pubkey,
    compression_authority: Pubkey,
    compressible_config: Pubkey,
    authority_index: u8,
    destination_index: u8,
    indices: Vec<CompressAndCloseIndices>,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let instruction_data = CompressAndCloseData {
        authority_index,
        destination_index,
        indices,
    };

    let mut data = Vec::with_capacity(8 + 200);
    data.extend_from_slice(&COMPRESS_AND_CLOSE_DISCRIMINATOR);
    instruction_data.serialize(&mut data).unwrap();

    let mut accounts = vec![
        AccountMeta::new(*authority, true),
        AccountMeta::new(registered_forester_pda, false),
        AccountMeta::new(compression_authority, false),
        AccountMeta::new_readonly(compressible_config, false),
    ];

    accounts.extend(remaining_accounts);

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts,
        data,
    }
}
