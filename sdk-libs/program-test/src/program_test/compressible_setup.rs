//! Test helpers for cold account operations.

use light_client::{
    interface::instructions,
    rpc::{Rpc, RpcError},
};
use solana_sdk::{
    bpf_loader_upgradeable,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

use crate::program_test::TestRpc;

/// Create mock program data account for testing.
pub fn create_mock_program_data(authority: Pubkey) -> Vec<u8> {
    let mut data = vec![0u8; 1024];
    data[0..4].copy_from_slice(&3u32.to_le_bytes()); // Program data discriminator
    data[4..12].copy_from_slice(&0u64.to_le_bytes()); // Slot
    data[12] = 1; // Option<Pubkey> Some(authority)
    data[13..45].copy_from_slice(authority.as_ref()); // Authority pubkey
    data
}

/// Setup mock program data account for testing.
pub fn setup_mock_program_data<T: TestRpc>(
    rpc: &mut T,
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

#[allow(clippy::too_many_arguments)]
pub async fn initialize_compression_config<T: Rpc>(
    rpc: &mut T,
    payer: &Keypair,
    program_id: &Pubkey,
    authority: &Keypair,
    rent_sponsor: Pubkey,
    address_space: Vec<Pubkey>,
    discriminator: &[u8],
    config_bump: Option<u8>,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    if address_space.is_empty() {
        return Err(RpcError::CustomError(
            "At least one address space must be provided".to_string(),
        ));
    }

    let instruction = instructions::initialize_config(
        program_id,
        discriminator,
        &payer.pubkey(),
        &authority.pubkey(),
        rent_sponsor,
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

#[allow(clippy::too_many_arguments)]
pub async fn update_compression_config<T: Rpc>(
    rpc: &mut T,
    payer: &Keypair,
    program_id: &Pubkey,
    authority: &Keypair,
    new_rent_sponsor: Option<Pubkey>,
    new_address_space: Option<Vec<Pubkey>>,
    new_update_authority: Option<Pubkey>,
    discriminator: &[u8],
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let instruction = instructions::update_config(
        program_id,
        discriminator,
        &authority.pubkey(),
        new_rent_sponsor,
        new_address_space,
        new_update_authority,
    );

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, authority])
        .await
}
