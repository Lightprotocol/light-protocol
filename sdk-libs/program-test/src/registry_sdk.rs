//! Local registry SDK for program-test.
//!
//! This module provides the minimal registry program SDK functionality needed
//! for forester and compressible modules without requiring the `devenv` feature.
//! It reimplements the necessary constants, PDA derivation, type definitions,
//! and instruction builders locally to avoid anchor program dependencies.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_pubkey::Pubkey;
use solana_sdk::instruction::{AccountMeta, Instruction};

// ============================================================================
// Program IDs
// ============================================================================

/// Registry program ID
pub const REGISTRY_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");

// ============================================================================
// PDA Seeds
// ============================================================================

pub const FORESTER_SEED: &[u8] = b"forester";
pub const FORESTER_EPOCH_SEED: &[u8] = b"forester_epoch";
pub const PROTOCOL_CONFIG_PDA_SEED: &[u8] = b"authority";

// ============================================================================
// Instruction Discriminators (from discriminator test)
// ============================================================================

/// Claim instruction discriminator
pub const CLAIM_DISCRIMINATOR: [u8; 8] = [62, 198, 214, 193, 213, 159, 108, 210];

/// CompressAndClose instruction discriminator
pub const COMPRESS_AND_CLOSE_DISCRIMINATOR: [u8; 8] = [96, 94, 135, 18, 121, 42, 213, 117];

/// RegisterForester instruction discriminator
pub const REGISTER_FORESTER_DISCRIMINATOR: [u8; 8] = [62, 47, 240, 103, 84, 200, 226, 73];

/// RegisterForesterEpoch instruction discriminator
pub const REGISTER_FORESTER_EPOCH_DISCRIMINATOR: [u8; 8] = [43, 120, 253, 194, 109, 192, 101, 188];

/// FinalizeRegistration instruction discriminator
pub const FINALIZE_REGISTRATION_DISCRIMINATOR: [u8; 8] = [230, 188, 172, 96, 204, 247, 98, 227];

/// ReportWork instruction discriminator
#[allow(dead_code)]
pub const REPORT_WORK_DISCRIMINATOR: [u8; 8] = [170, 110, 232, 47, 145, 213, 138, 162];

// ============================================================================
// Account Discriminators (for direct account serialization)
// ============================================================================

/// ProtocolConfigPda account discriminator
pub const PROTOCOL_CONFIG_PDA_DISCRIMINATOR: [u8; 8] = [96, 176, 239, 146, 1, 254, 99, 146];

/// ForesterPda account discriminator
pub const FORESTER_PDA_DISCRIMINATOR: [u8; 8] = [51, 47, 187, 86, 82, 153, 117, 5];

/// ForesterEpochPda account discriminator
pub const FORESTER_EPOCH_PDA_DISCRIMINATOR: [u8; 8] = [29, 117, 211, 141, 99, 143, 250, 114];

/// EpochPda account discriminator
pub const EPOCH_PDA_DISCRIMINATOR: [u8; 8] = [66, 224, 46, 2, 167, 137, 120, 107];

// ============================================================================
// PDA Derivation Functions
// ============================================================================

/// Derives the protocol config PDA address.
pub fn get_protocol_config_pda_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PROTOCOL_CONFIG_PDA_SEED], &REGISTRY_PROGRAM_ID)
}

/// Derives the forester PDA for a given authority.
pub fn get_forester_pda(authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[FORESTER_SEED, authority.as_ref()], &REGISTRY_PROGRAM_ID)
}

/// Derives the forester epoch PDA from forester PDA and epoch.
pub fn get_forester_epoch_pda(forester_pda: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            FORESTER_EPOCH_SEED,
            forester_pda.as_ref(),
            epoch.to_le_bytes().as_slice(),
        ],
        &REGISTRY_PROGRAM_ID,
    )
}

/// Derives the forester epoch PDA from authority and epoch.
pub fn get_forester_epoch_pda_from_authority(authority: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    let forester_pda = get_forester_pda(authority);
    get_forester_epoch_pda(&forester_pda.0, epoch)
}

/// Derives the epoch PDA address for a given epoch.
pub fn get_epoch_pda_address(epoch: u64) -> Pubkey {
    Pubkey::find_program_address(&[&epoch.to_le_bytes()], &REGISTRY_PROGRAM_ID).0
}

// ============================================================================
// Type Definitions
// ============================================================================

/// Configuration for a forester.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ForesterConfig {
    /// Fee in percentage points.
    pub fee: u64,
}

/// Forester PDA account structure.
#[derive(Debug, Default, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ForesterPda {
    pub authority: Pubkey,
    pub config: ForesterConfig,
    pub active_weight: u64,
    /// Pending weight which will get active once the next epoch starts.
    pub pending_weight: u64,
    pub current_epoch: u64,
    /// Link to previous compressed forester epoch account hash.
    pub last_compressed_forester_epoch_pda_hash: [u8; 32],
    pub last_registered_epoch: u64,
}

/// Protocol configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ProtocolConfig {
    /// Solana slot when the protocol starts operating.
    pub genesis_slot: u64,
    /// Minimum weight required for a forester to register to an epoch.
    pub min_weight: u64,
    /// Light protocol slot length
    pub slot_length: u64,
    /// Foresters can register for this phase.
    pub registration_phase_length: u64,
    /// Foresters can perform work in this phase.
    pub active_phase_length: u64,
    /// Foresters can report work to receive performance based rewards in this phase.
    pub report_work_phase_length: u64,
    pub network_fee: u64,
    pub cpi_context_size: u64,
    pub finalize_counter_limit: u64,
    /// Placeholder for future protocol updates.
    pub place_holder: Pubkey,
    pub address_network_fee: u64,
    pub place_holder_b: u64,
    pub place_holder_c: u64,
    pub place_holder_d: u64,
    pub place_holder_e: u64,
    pub place_holder_f: u64,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            genesis_slot: 0,
            min_weight: 1,
            slot_length: 10,
            registration_phase_length: 100,
            active_phase_length: 1000,
            report_work_phase_length: 100,
            network_fee: 5000,
            cpi_context_size: 20 * 1024 + 8, // DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2
            finalize_counter_limit: 100,
            place_holder: Pubkey::default(),
            address_network_fee: 10000,
            place_holder_b: 0,
            place_holder_c: 0,
            place_holder_d: 0,
            place_holder_e: 0,
            place_holder_f: 0,
        }
    }
}

/// Protocol config PDA account structure.
/// Includes Anchor's 8-byte discriminator at the start.
#[derive(Debug, BorshDeserialize)]
pub struct ProtocolConfigPda {
    pub authority: Pubkey,
    pub bump: u8,
    pub config: ProtocolConfig,
}

/// Indices for CompressAndClose operation (matches registry program's definition).
#[derive(Debug, Copy, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressAndCloseIndices {
    pub source_index: u8,
    pub mint_index: u8,
    pub owner_index: u8,
    pub rent_sponsor_index: u8,
    pub delegate_index: u8,
}

// ============================================================================
// Instruction Builders
// ============================================================================

/// Builds the Claim instruction.
///
/// # Accounts (in order)
/// - authority (signer, writable)
/// - registered_forester_pda (writable)
/// - rent_sponsor (writable)
/// - compression_authority (read-only)
/// - compressible_config (read-only)
/// - compressed_token_program (read-only)
/// - token_accounts (writable, remaining)
pub fn build_claim_instruction(
    authority: Pubkey,
    registered_forester_pda: Pubkey,
    rent_sponsor: Pubkey,
    compression_authority: Pubkey,
    compressible_config: Pubkey,
    compressed_token_program: Pubkey,
    token_accounts: &[Pubkey],
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new(registered_forester_pda, false),
        AccountMeta::new(rent_sponsor, false),
        AccountMeta::new_readonly(compression_authority, false),
        AccountMeta::new_readonly(compressible_config, false),
        AccountMeta::new_readonly(compressed_token_program, false),
    ];

    for token_account in token_accounts {
        accounts.push(AccountMeta::new(*token_account, false));
    }

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts,
        data: CLAIM_DISCRIMINATOR.to_vec(),
    }
}

/// Builds the CompressAndClose instruction.
///
/// # Accounts (in order)
/// - authority (signer, writable)
/// - registered_forester_pda (writable)
/// - compression_authority (writable)
/// - compressible_config (read-only)
/// - remaining_accounts
#[allow(clippy::too_many_arguments)]
pub fn build_compress_and_close_instruction(
    authority: Pubkey,
    registered_forester_pda: Pubkey,
    compression_authority: Pubkey,
    compressible_config: Pubkey,
    authority_index: u8,
    destination_index: u8,
    indices: Vec<CompressAndCloseIndices>,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new(registered_forester_pda, false),
        AccountMeta::new(compression_authority, false),
        AccountMeta::new_readonly(compressible_config, false),
    ];
    accounts.extend(remaining_accounts);

    // Serialize instruction data: discriminator + authority_index + destination_index + indices vec
    let mut data = COMPRESS_AND_CLOSE_DISCRIMINATOR.to_vec();
    data.push(authority_index);
    data.push(destination_index);
    // Borsh serialize the indices vector
    indices.serialize(&mut data).unwrap();

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts,
        data,
    }
}

/// Builds the RegisterForester instruction.
///
/// # Accounts (in order):
/// 1. fee_payer (signer, writable)
/// 2. authority (signer)
/// 3. protocol_config_pda (read-only)
/// 4. forester_pda (writable, init)
/// 5. system_program (read-only)
pub fn create_register_forester_instruction(
    fee_payer: &Pubkey,
    governance_authority: &Pubkey,
    forester_authority: &Pubkey,
    config: ForesterConfig,
) -> Instruction {
    let (forester_pda, bump) = get_forester_pda(forester_authority);
    let (protocol_config_pda, _) = get_protocol_config_pda_address();

    let accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new_readonly(protocol_config_pda, false),
        AccountMeta::new(forester_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
    ];

    // Instruction data: discriminator + bump + authority (pubkey) + config + weight (Option<u64>)
    let mut data = REGISTER_FORESTER_DISCRIMINATOR.to_vec();
    data.push(bump);
    data.extend_from_slice(forester_authority.as_ref());
    config.serialize(&mut data).unwrap();
    // weight: Some(1) encoded as Option<u64>
    data.push(1u8); // Some variant
    data.extend_from_slice(&1u64.to_le_bytes()); // weight = 1

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts,
        data,
    }
}

/// Builds the RegisterForesterEpoch instruction.
///
/// # Accounts (in order):
/// 1. fee_payer (signer, writable)
/// 2. forester_epoch_pda (writable, init)
/// 3. forester_pda (read-only)
/// 4. authority (signer)
/// 5. epoch_pda (writable, init_if_needed)
/// 6. protocol_config (read-only)
/// 7. system_program (read-only)
pub fn create_register_forester_epoch_pda_instruction(
    authority: &Pubkey,
    derivation: &Pubkey,
    epoch: u64,
) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_from_authority(derivation, epoch);
    let (forester_pda, _) = get_forester_pda(derivation);
    let epoch_pda = get_epoch_pda_address(epoch);
    let protocol_config_pda = get_protocol_config_pda_address().0;

    let accounts = vec![
        AccountMeta::new(*authority, true),                    // fee_payer
        AccountMeta::new(forester_epoch_pda, false),           // forester_epoch_pda
        AccountMeta::new_readonly(forester_pda, false),        // forester_pda
        AccountMeta::new_readonly(*authority, true),           // authority
        AccountMeta::new(epoch_pda, false),                    // epoch_pda
        AccountMeta::new_readonly(protocol_config_pda, false), // protocol_config
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // system_program
    ];

    // Instruction data: discriminator + epoch (u64)
    let mut data = REGISTER_FORESTER_EPOCH_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&epoch.to_le_bytes());

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts,
        data,
    }
}

/// Builds the FinalizeRegistration instruction.
///
/// # Accounts (in order):
/// 1. forester_epoch_pda (writable)
/// 2. authority (signer)
/// 3. epoch_pda (read-only)
pub fn create_finalize_registration_instruction(
    authority: &Pubkey,
    derivation: &Pubkey,
    epoch: u64,
) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_from_authority(derivation, epoch);
    let epoch_pda = get_epoch_pda_address(epoch);

    let accounts = vec![
        AccountMeta::new(forester_epoch_pda, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(epoch_pda, false),
    ];

    Instruction {
        program_id: REGISTRY_PROGRAM_ID,
        accounts,
        data: FINALIZE_REGISTRATION_DISCRIMINATOR.to_vec(),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Deserializes a ProtocolConfigPda from account data.
/// Skips the 8-byte Anchor discriminator automatically.
pub fn deserialize_protocol_config_pda(data: &[u8]) -> Result<ProtocolConfigPda, std::io::Error> {
    // Skip 8-byte Anchor discriminator
    if data.len() < 8 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Account data too short for discriminator",
        ));
    }
    ProtocolConfigPda::deserialize(&mut &data[8..])
}

/// Deserializes a ForesterPda from account data.
/// Skips the 8-byte Anchor discriminator automatically.
pub fn deserialize_forester_pda(data: &[u8]) -> Result<ForesterPda, std::io::Error> {
    // Skip 8-byte Anchor discriminator
    if data.len() < 8 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Account data too short for discriminator",
        ));
    }
    ForesterPda::deserialize(&mut &data[8..])
}

// ============================================================================
// ForesterEpochPda (for direct account serialization)
// ============================================================================

/// ForesterEpochPda account structure for serialization.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ForesterEpochPda {
    pub authority: Pubkey,
    pub config: ForesterConfig,
    pub epoch: u64,
    pub weight: u64,
    pub work_counter: u64,
    pub has_reported_work: bool,
    pub forester_index: u64,
    pub epoch_active_phase_start_slot: u64,
    pub total_epoch_weight: Option<u64>,
    pub protocol_config: ProtocolConfig,
    pub finalize_counter: u64,
}

/// EpochPda account structure for serialization.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct EpochPda {
    pub epoch: u64,
    pub protocol_config: ProtocolConfig,
    pub total_work: u64,
    pub registered_weight: u64,
}

// ============================================================================
// Direct Account Serialization (for LiteSVM set_account)
// ============================================================================

/// Creates a ProtocolConfig with a very long active phase (effectively infinite).
/// This allows any slot to be in epoch 0's active phase.
pub fn protocol_config_for_tests() -> ProtocolConfig {
    ProtocolConfig {
        genesis_slot: 0,
        min_weight: 1,
        slot_length: 10,
        registration_phase_length: 0, // No registration phase - always active
        active_phase_length: u64::MAX / 2, // Very long active phase
        report_work_phase_length: 0,
        network_fee: 5000,
        cpi_context_size: 20 * 1024 + 8,
        finalize_counter_limit: u64::MAX,
        place_holder: Pubkey::default(),
        address_network_fee: 10000,
        place_holder_b: 0,
        place_holder_c: 0,
        place_holder_d: 0,
        place_holder_e: 0,
        place_holder_f: 0,
    }
}

/// Serializes a ProtocolConfigPda to account data with Anchor discriminator.
pub fn serialize_protocol_config_pda(
    authority: Pubkey,
    bump: u8,
    config: ProtocolConfig,
) -> Vec<u8> {
    let mut data = PROTOCOL_CONFIG_PDA_DISCRIMINATOR.to_vec();
    authority.serialize(&mut data).unwrap();
    data.push(bump);
    config.serialize(&mut data).unwrap();
    data
}

/// Serializes a ForesterPda to account data with Anchor discriminator.
pub fn serialize_forester_pda(forester: &ForesterPda) -> Vec<u8> {
    let mut data = FORESTER_PDA_DISCRIMINATOR.to_vec();
    forester.authority.serialize(&mut data).unwrap();
    forester.config.serialize(&mut data).unwrap();
    forester.active_weight.serialize(&mut data).unwrap();
    forester.pending_weight.serialize(&mut data).unwrap();
    forester.current_epoch.serialize(&mut data).unwrap();
    forester
        .last_compressed_forester_epoch_pda_hash
        .serialize(&mut data)
        .unwrap();
    forester.last_registered_epoch.serialize(&mut data).unwrap();
    data
}

/// Serializes a ForesterEpochPda to account data with Anchor discriminator.
pub fn serialize_forester_epoch_pda(epoch_pda: &ForesterEpochPda) -> Vec<u8> {
    let mut data = FORESTER_EPOCH_PDA_DISCRIMINATOR.to_vec();
    epoch_pda.serialize(&mut data).unwrap();
    data
}

/// Serializes an EpochPda to account data with Anchor discriminator.
pub fn serialize_epoch_pda(epoch_pda: &EpochPda) -> Vec<u8> {
    let mut data = EPOCH_PDA_DISCRIMINATOR.to_vec();
    epoch_pda.serialize(&mut data).unwrap();
    data
}

/// Sets up protocol config, forester, and forester epoch accounts for testing.
/// Uses a very long active phase so any slot is valid for epoch 0.
///
/// This allows compress/close operations to work without the full devenv setup.
pub fn setup_test_protocol_accounts(
    context: &mut litesvm::LiteSVM,
    forester_authority: &Pubkey,
) -> Result<(), String> {
    let protocol_config = protocol_config_for_tests();

    // 1. Set up ProtocolConfigPda
    let (protocol_config_pda, protocol_bump) = get_protocol_config_pda_address();
    let protocol_data = serialize_protocol_config_pda(
        *forester_authority, // Use forester as governance authority for simplicity
        protocol_bump,
        protocol_config,
    );
    let protocol_account = solana_account::Account {
        lamports: 1_000_000_000,
        data: protocol_data,
        owner: REGISTRY_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    context
        .set_account(protocol_config_pda, protocol_account)
        .map_err(|e| format!("Failed to set protocol config account: {}", e))?;

    // 2. Set up ForesterPda
    let (forester_pda, _forester_bump) = get_forester_pda(forester_authority);
    let forester = ForesterPda {
        authority: *forester_authority,
        config: ForesterConfig::default(),
        active_weight: 1,
        pending_weight: 0,
        current_epoch: 0,
        last_compressed_forester_epoch_pda_hash: [0u8; 32],
        last_registered_epoch: 0,
    };
    let forester_data = serialize_forester_pda(&forester);
    let forester_account = solana_account::Account {
        lamports: 1_000_000_000,
        data: forester_data,
        owner: REGISTRY_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    context
        .set_account(forester_pda, forester_account)
        .map_err(|e| format!("Failed to set forester account: {}", e))?;

    // 3. Set up ForesterEpochPda for epoch 0
    let (forester_epoch_pda, _epoch_bump) =
        get_forester_epoch_pda_from_authority(forester_authority, 0);
    let forester_epoch = ForesterEpochPda {
        authority: *forester_authority,
        config: ForesterConfig::default(),
        epoch: 0,
        weight: 1,
        work_counter: 0,
        has_reported_work: false,
        forester_index: 0,
        epoch_active_phase_start_slot: 0,
        total_epoch_weight: Some(1), // Must be Some for active phase
        protocol_config,
        finalize_counter: 1, // Already finalized
    };
    let forester_epoch_data = serialize_forester_epoch_pda(&forester_epoch);
    let forester_epoch_account = solana_account::Account {
        lamports: 1_000_000_000,
        data: forester_epoch_data,
        owner: REGISTRY_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    context
        .set_account(forester_epoch_pda, forester_epoch_account)
        .map_err(|e| format!("Failed to set forester epoch account: {}", e))?;

    // 4. Set up EpochPda for epoch 0
    let epoch_pda_address = get_epoch_pda_address(0);
    let epoch_pda = EpochPda {
        epoch: 0,
        protocol_config,
        total_work: 0,
        registered_weight: 1, // Must match forester weight
    };
    let epoch_pda_data = serialize_epoch_pda(&epoch_pda);
    let epoch_pda_account = solana_account::Account {
        lamports: 1_000_000_000,
        data: epoch_pda_data,
        owner: REGISTRY_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    context
        .set_account(epoch_pda_address, epoch_pda_account)
        .map_err(|e| format!("Failed to set epoch pda account: {}", e))?;

    Ok(())
}
