use forester_utils::forester_epoch::{Epoch, TreeAccounts};
use light_client::rpc::{Rpc, RpcError};
use light_program_test::program_test::TestRpc;
use light_compressed_account::TreeType;
use light_registry::{
    protocol_config::state::ProtocolConfig,
    sdk::create_finalize_registration_instruction,
};
use light_program_test::utils::register_test_forester::register_test_forester;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use light_registry::ForesterConfig;

/// Sets up a forester, registers it, and advances to the active epoch phase.
/// This function encapsulates all forester-related setup that was previously
/// done conditionally in light-program-test.
///
/// # Arguments
/// * `context` - The test RPC context
/// * `protocol_config` - Protocol configuration
/// * `forester_keypair` - Keypair for the forester
/// * `state_merkle_tree` - State merkle tree pubkey
/// * `nullifier_queue` - Nullifier queue pubkey  
/// * `address_merkle_tree` - Address merkle tree pubkey
/// * `address_queue` - Address queue pubkey
///
/// # Returns
/// * `Result<Epoch, RpcError>` - The registered and activated epoch
pub async fn setup_forester_and_advance_to_epoch<R: Rpc + TestRpc>(
    context: &mut R,
    protocol_config: &ProtocolConfig,
    governance_authority: &Keypair,
    forester_keypair: &Keypair,
    state_merkle_tree: Pubkey,
    nullifier_queue: Pubkey,
    address_merkle_tree: Pubkey,
    address_queue: Pubkey,
) -> Result<Epoch, RpcError> {
    // Register the test forester
    register_test_forester(
        context,
        governance_authority,
        &forester_keypair.pubkey(),
        ForesterConfig::default(),
    )
    .await?;

    // Register the epoch
    let mut registered_epoch = Epoch::register(
        context,
        protocol_config,
        forester_keypair,
        &forester_keypair.pubkey(),
    )
    .await?
    .unwrap();

    // Advance to active phase
    context.warp_to_slot(registered_epoch.phases.active.start)?;

    // Create tree accounts for the epoch
    let tree_accounts = vec![
        TreeAccounts::new(
            state_merkle_tree,
            nullifier_queue,
            TreeType::StateV1,
            false,
        ),
        TreeAccounts::new(
            address_merkle_tree,
            address_queue,
            TreeType::AddressV1,
            false,
        ),
    ];

    // Add trees to the epoch with schedule
    registered_epoch
        .fetch_account_and_add_trees_with_schedule(context, &tree_accounts)
        .await?;

    // Finalize registration
    let ix = create_finalize_registration_instruction(
        &forester_keypair.pubkey(),
        &forester_keypair.pubkey(),
        0,
    );
    context
        .create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[forester_keypair])
        .await?;

    Ok(registered_epoch)
}