use forester_utils::forester_epoch::{Epoch, TreeAccounts};
use light_client::rpc::{Rpc, RpcError};
use light_compressed_account::TreeType;
use light_program_test::{
    accounts::test_keypairs::TestKeypairs, program_test::TestRpc,
    utils::register_test_forester::register_test_forester,
};
use light_registry::{
    protocol_config::state::ProtocolConfig, sdk::create_finalize_registration_instruction,
    ForesterConfig,
};
use solana_sdk::signature::Signer;

/// Sets up a forester, registers it, and advances to the active epoch phase.
/// This function encapsulates all forester-related setup that was previously
/// done conditionally in light-program-test.
///
/// # Arguments
/// * `context` - The test RPC context
/// * `protocol_config` - Protocol configuration
///
/// # Returns
/// * `Result<Epoch, RpcError>` - The registered and activated epoch
pub async fn setup_forester_and_advance_to_epoch<R: Rpc + TestRpc>(
    context: &mut R,
    protocol_config: &ProtocolConfig,
) -> Result<Epoch, RpcError> {
    let test_keypairs = TestKeypairs::program_test_default();
    // Register the test forester
    register_test_forester(
        context,
        &test_keypairs.governance_authority,
        &test_keypairs.forester.pubkey(),
        ForesterConfig::default(),
    )
    .await?;

    // Register the epoch
    let mut registered_epoch = Epoch::register(
        context,
        protocol_config,
        &test_keypairs.forester,
        &test_keypairs.forester.pubkey(),
    )
    .await?
    .ok_or_else(|| RpcError::CustomError("Failed to register epoch".to_string()))?;

    // Advance to active phase
    context.warp_to_slot(registered_epoch.phases.active.start)?;

    // Create tree accounts for the epoch using test keypairs
    let tree_accounts = vec![
        TreeAccounts::new(
            test_keypairs.state_merkle_tree.pubkey(),
            test_keypairs.nullifier_queue.pubkey(),
            TreeType::StateV1,
            false,
        ),
        TreeAccounts::new(
            test_keypairs.address_merkle_tree.pubkey(),
            test_keypairs.address_merkle_tree_queue.pubkey(),
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
        &test_keypairs.forester.pubkey(),
        &test_keypairs.forester.pubkey(),
        0,
    );
    context
        .create_and_send_transaction(
            &[ix],
            &test_keypairs.forester.pubkey(),
            &[&test_keypairs.forester],
        )
        .await?;

    Ok(registered_epoch)
}
