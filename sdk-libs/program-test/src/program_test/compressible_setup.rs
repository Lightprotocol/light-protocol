//! Test helpers for compressible account operations
//!
//! This module provides common functionality for testing compressible accounts,
//! including mock program data setup and configuration management.

use light_compressible_client::CompressibleInstruction;
use solana_sdk::{
    bpf_loader_upgradeable,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

use crate::{
    program_test::{LightProgramTest, TestRpc},
    Rpc, RpcError,
};

/// Create mock program data account for testing
///
/// This creates a minimal program data account structure that mimics
/// what the BPF loader would create for deployed programs.
pub fn create_mock_program_data(authority: Pubkey) -> Vec<u8> {
    let mut data = vec![0u8; 1024];
    data[0..4].copy_from_slice(&3u32.to_le_bytes()); // Program data discriminator
    data[4..12].copy_from_slice(&0u64.to_le_bytes()); // Slot
    data[12] = 1; // Option<Pubkey> Some(authority)
    data[13..45].copy_from_slice(authority.as_ref()); // Authority pubkey
    data
}

/// Setup mock program data account for testing
///
/// For testing without ledger, LiteSVM does not create program data accounts,
/// so we need to create them manually. This is required for programs that
/// check their upgrade authority.
///
/// # Arguments
/// * `rpc` - The test RPC client
/// * `payer` - The payer keypair (used as authority)
/// * `program_id` - The program ID to create data account for
///
/// # Returns
/// The pubkey of the created program data account
pub fn setup_mock_program_data(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
) -> Pubkey {
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(program_data_pda, mock_account);
    program_data_pda
}

/// Initialize compression config for a program
///
/// This is a high-level helper that handles the complete flow of initializing
/// a compression configuration for a program, including proper signer management.
///
/// # Arguments
/// * `rpc` - The test RPC client
/// * `payer` - The transaction fee payer
/// * `program_id` - The program to initialize config for
/// * `authority` - The config authority (can be same as payer)
/// * `compression_delay` - Number of slots to wait before compression
/// * `rent_recipient` - Where to send rent from compressed accounts
/// * `address_space` - List of address trees for this program
///
/// # Returns
/// Transaction signature on success
#[allow(clippy::too_many_arguments)]
pub async fn initialize_compression_config(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    authority: &Keypair,
    compression_delay: u32,
    rent_recipient: Pubkey,
    address_space: Vec<Pubkey>,
    discriminator: &[u8],
    config_bump: Option<u8>,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    if address_space.is_empty() {
        return Err(RpcError::CustomError(
            "At least one address space must be provided".to_string(),
        ));
    }

    // Use the mid-level instruction builder
    let instruction = CompressibleInstruction::initialize_compression_config(
        program_id,
        discriminator,
        &payer.pubkey(),
        &authority.pubkey(),
        compression_delay,
        rent_recipient,
        address_space,
        config_bump,
    );

    let signers = if payer.pubkey() == authority.pubkey() {
        vec![payer]
    } else {
        vec![payer, authority]
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await
}

/// Update compression config for a program
///
/// This is a high-level helper for updating an existing compression configuration.
/// All parameters except the required ones are optional - pass None to keep existing values.
///
/// # Arguments
/// * `rpc` - The test RPC client
/// * `payer` - The transaction fee payer
/// * `program_id` - The program to update config for
/// * `authority` - The current config authority
/// * `new_compression_delay` - New compression delay (optional)
/// * `new_rent_recipient` - New rent recipient (optional)
/// * `new_address_space` - New address space list (optional)
/// * `new_update_authority` - New authority (optional)
///
/// # Returns
/// Transaction signature on success
#[allow(clippy::too_many_arguments)]
pub async fn update_compression_config(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    authority: &Keypair,
    new_compression_delay: Option<u32>,
    new_rent_recipient: Option<Pubkey>,
    new_address_space: Option<Vec<Pubkey>>,
    new_update_authority: Option<Pubkey>,
    discriminator: &[u8],
) -> Result<solana_sdk::signature::Signature, RpcError> {
    // Use the mid-level instruction builder
    let instruction = CompressibleInstruction::update_compression_config(
        program_id,
        discriminator,
        &authority.pubkey(),
        new_compression_delay,
        new_rent_recipient,
        new_address_space,
        new_update_authority,
    );

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, authority])
        .await
}
