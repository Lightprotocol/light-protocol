use std::cmp;

use account_compression::{AddressMerkleTreeConfig, AddressQueueConfig, RegisteredProgram};
use light_merkle_tree_metadata::queue::QueueType;
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction,
};
pub mod address;
pub mod address_tree_rollover;
pub mod assert_compressed_tx;
pub mod assert_epoch;
pub mod assert_merkle_tree;
pub mod assert_queue;
pub mod assert_rollover;
pub mod assert_token_tx;
pub mod conversions;
pub mod create_address_test_program_sdk;
pub mod e2e_test_env;
#[allow(unused)]
pub mod spl;
pub mod state_tree_rollover;
pub mod system_program;
#[allow(unused)]
pub mod test_forester;

pub use create_address_test_program::ID as CREATE_ADDRESS_TEST_PROGRAM_ID;
pub use forester_utils::{
    airdrop_lamports, create_account_instruction,
    forester_epoch::{Epoch, TreeAccounts, TreeType},
    get_concurrent_merkle_tree, get_hash_set, get_indexed_merkle_tree,
    registry::{
        create_rollover_address_merkle_tree_instructions,
        create_rollover_state_merkle_tree_instructions, register_test_forester,
        update_test_forester,
    },
    AccountZeroCopy,
};
pub use light_client::{
    rpc::{
        assert_rpc_error, solana_rpc::SolanaRpcUrl, RpcConnection, RpcError, SolanaRpcConnection,
    },
    transaction_params::{FeeConfig, TransactionParams},
};
use light_hasher::Poseidon;
use light_program_test::{
    indexer::utils::assert_address_merkle_tree_initialized,
    test_env::create_address_merkle_tree_and_queue_account,
};
use light_registry::account_compression_cpi::sdk::get_registered_program_pda;

use crate::assert_queue::assert_address_queue_initialized;

#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub async fn create_address_merkle_tree_and_queue_account_with_assert<R: RpcConnection>(
    payer: &Keypair,
    registry: bool,
    context: &mut R,
    address_merkle_tree_keypair: &Keypair,
    address_queue_keypair: &Keypair,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
    index: u64,
) -> Result<Signature, RpcError> {
    let result = create_address_merkle_tree_and_queue_account(
        payer,
        registry,
        context,
        address_merkle_tree_keypair,
        address_queue_keypair,
        program_owner,
        forester,
        merkle_tree_config,
        queue_config,
        index,
    )
    .await;

    // To initialize the indexed tree we do 4 operations:
    // 1. insert 0 append 0 and update 0
    // 2. insert 1 append BN254_FIELD_SIZE -1 and update 0
    // we appended two values this the expected next index is 2;
    // The right most leaf is the hash of the indexed array element with value FIELD_SIZE - 1
    // index 1, next_index: 0
    let expected_change_log_length = cmp::min(4, merkle_tree_config.changelog_size as usize);
    let expected_roots_length = cmp::min(4, merkle_tree_config.roots_size as usize);
    let expected_next_index = 2;
    let expected_indexed_change_log_length =
        cmp::min(4, merkle_tree_config.address_changelog_size as usize);

    let mut reference_tree =
        light_indexed_merkle_tree::reference::IndexedMerkleTree::<Poseidon, usize>::new(
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_HEIGHT as usize,
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_CANOPY_DEPTH as usize,
        )
        .unwrap();
    reference_tree.init().unwrap();

    let expected_right_most_leaf = reference_tree
        .merkle_tree
        .get_leaf(reference_tree.merkle_tree.rightmost_index - 1)
        .unwrap();

    let _expected_right_most_leaf = [
        30, 164, 22, 238, 180, 2, 24, 181, 64, 193, 207, 184, 219, 233, 31, 109, 84, 232, 162, 158,
        220, 48, 163, 158, 50, 107, 64, 87, 167, 217, 99, 245,
    ];
    assert_eq!(expected_right_most_leaf, _expected_right_most_leaf);
    let owner = if registry {
        let registered_program = get_registered_program_pda(&light_registry::ID);
        let registered_program_account = context
            .get_anchor_account::<RegisteredProgram>(&registered_program)
            .await
            .unwrap()
            .unwrap();
        registered_program_account.group_authority_pda
    } else {
        payer.pubkey()
    };

    assert_address_merkle_tree_initialized(
        context,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
        merkle_tree_config,
        index,
        program_owner,
        forester,
        expected_change_log_length,
        expected_roots_length,
        expected_next_index,
        &expected_right_most_leaf,
        &owner,
        expected_indexed_change_log_length,
    )
    .await;

    assert_address_queue_initialized(
        context,
        &address_queue_keypair.pubkey(),
        queue_config,
        &address_merkle_tree_keypair.pubkey(),
        merkle_tree_config,
        QueueType::AddressQueue,
        index,
        program_owner,
        forester,
        &owner,
    )
    .await;

    result
}

/// Asserts that the given `BanksTransactionResultWithMetadata` is an error with a custom error code
/// or a program error.
/// Unfortunately BanksTransactionResultWithMetadata does not reliably expose the custom error code, so
/// we allow program error as well.
// TODO: unify with assert_rpc_error
#[allow(clippy::result_large_err)]
pub fn assert_custom_error_or_program_error(
    result: Result<solana_sdk::signature::Signature, RpcError>,
    error_code: u32,
) -> Result<(), RpcError> {
    let accepted_errors = [
        (0, InstructionError::ProgramFailedToComplete),
        (0, InstructionError::Custom(error_code)),
    ];

    let is_accepted = accepted_errors.iter().any(|(index, error)| {
        matches!(result, Err(RpcError::TransactionError(transaction::TransactionError::InstructionError(i, ref e))) if i == (*index as u8) && e == error)
    });

    if !is_accepted {
        println!("result {:?}", result);
        println!("error_code {:?}", error_code);
        return Err(RpcError::AssertRpcError(format!(
            "Expected error code {} or program error, got {:?}",
            error_code, result
        )));
    }

    Ok(())
}
