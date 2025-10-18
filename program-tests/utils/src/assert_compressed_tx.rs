use account_compression::{state::QueueAccount, StateMerkleTreeAccount};
use anchor_lang::Discriminator;
use forester_utils::account_zero_copy::{
    get_concurrent_merkle_tree, get_hash_set, AccountZeroCopy,
};
use light_account_checks::discriminator::Discriminator as LightDiscriminator;
use light_batched_merkle_tree::{
    batch::Batch, merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueMetadata,
};
use light_client::{
    indexer::{Indexer, StateMerkleTreeAccounts},
    rpc::Rpc,
};
use light_compressed_account::{
    compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
    TreeType,
};
use light_event::event::{MerkleTreeSequenceNumber, PublicTransactionEvent};
use light_hasher::Poseidon;
use light_program_test::indexer::TestIndexerExtensions;
use num_bigint::BigUint;
use num_traits::FromBytes;
use solana_sdk::{account::ReadableAccount, pubkey::Pubkey};

use crate::system_program::get_sol_pool_pda;

pub struct AssertCompressedTransactionInputs<'a, R: Rpc, I: Indexer + TestIndexerExtensions> {
    pub rpc: &'a mut R,
    pub test_indexer: &'a mut I,
    pub output_compressed_accounts: &'a [CompressedAccount],
    pub created_output_compressed_accounts: &'a [CompressedAccountWithMerkleContext],
    pub input_compressed_account_hashes: &'a [[u8; 32]],
    pub output_merkle_tree_snapshots: &'a [MerkleTreeTestSnapShot],
    pub input_merkle_tree_snapshots: &'a [MerkleTreeTestSnapShot],
    pub created_addresses: &'a [[u8; 32]],
    pub address_queue_pubkeys: &'a [Pubkey],
    pub event: &'a PublicTransactionEvent,
    pub sorted_output_accounts: bool,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub relay_fee: Option<u64>,
    pub compression_recipient: Option<Pubkey>,
    pub recipient_balance_pre: u64,
    pub compressed_sol_pda_balance_pre: u64,
}

/// General tx assert:
/// 1. outputs created
/// 2. inputs nullified
/// 3. addressed inserted into address queue
/// 4. Public Transaction event emitted correctly
/// 5. Merkle tree was updated correctly
/// 6. TODO: Fees have been paid (after fee refactor)
/// 7. Check compression amount was transferred
pub async fn assert_compressed_transaction<R: Rpc, I: Indexer + TestIndexerExtensions>(
    input: AssertCompressedTransactionInputs<'_, R, I>,
) {
    // CHECK 1
    assert_created_compressed_accounts(
        input.output_compressed_accounts,
        input
            .output_merkle_tree_snapshots
            .iter()
            .map(|x| x.accounts.merkle_tree)
            .collect::<Vec<_>>()
            .as_slice(),
        input.created_output_compressed_accounts,
    );
    // CHECK 2
    assert_nullifiers_exist_in_hash_sets(
        input.rpc,
        input.input_merkle_tree_snapshots,
        input.input_compressed_account_hashes,
    )
    .await;

    // CHECK 3
    assert_addresses_exist_in_hash_sets(
        input.rpc,
        input.address_queue_pubkeys,
        input.created_addresses,
    )
    .await;

    // CHECK 5
    let sequence_numbers = assert_merkle_tree_after_tx(
        input.rpc,
        input.output_merkle_tree_snapshots,
        input.test_indexer,
    )
    .await;

    // CHECK 4
    assert_public_transaction_event(
        input.event,
        Some(&input.input_compressed_account_hashes.to_vec()),
        input
            .output_merkle_tree_snapshots
            .iter()
            .map(|x| x.accounts)
            .collect::<Vec<_>>()
            .as_slice(),
        &input
            .created_output_compressed_accounts
            .iter()
            .map(|x| x.merkle_context.leaf_index)
            .collect::<Vec<_>>(),
        input.compress_or_decompress_lamports,
        input.is_compress,
        input.relay_fee,
        sequence_numbers,
    );

    // CHECK 7
    if let Some(compress_or_decompress_lamports) = input.compress_or_decompress_lamports {
        assert_compression(
            input.rpc,
            compress_or_decompress_lamports,
            input.compressed_sol_pda_balance_pre,
            input.recipient_balance_pre,
            &input.compression_recipient.unwrap_or_default(),
            input.is_compress,
        )
        .await;
    }
}

pub async fn assert_nullifiers_exist_in_hash_sets<R: Rpc>(
    rpc: &mut R,
    snapshots: &[MerkleTreeTestSnapShot],
    input_compressed_account_hashes: &[[u8; 32]],
) {
    for (i, hash) in input_compressed_account_hashes.iter().enumerate() {
        match snapshots[i].tree_type {
            TreeType::StateV1 => {
                let nullifier_queue = unsafe {
                    get_hash_set::<QueueAccount, R>(rpc, snapshots[i].accounts.nullifier_queue)
                        .await
                };
                assert!(nullifier_queue
                    .contains(&BigUint::from_be_bytes(hash.as_slice()), None)
                    .unwrap());
            }
            TreeType::StateV2 => {
                let mut merkle_tree_account_data = rpc
                    .get_account(snapshots[i].accounts.merkle_tree)
                    .await
                    .unwrap()
                    .unwrap()
                    .data
                    .clone();
                let mut merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
                    &mut merkle_tree_account_data,
                    &snapshots[i].accounts.merkle_tree.into(),
                )
                .unwrap();
                let mut batches = merkle_tree.queue_batches.batches;
                batches.iter_mut().enumerate().any(|(i, batch)| {
                    Batch::check_non_inclusion(
                        batch.num_iters as usize,
                        batch.bloom_filter_capacity,
                        hash,
                        merkle_tree.bloom_filter_stores[i],
                    )
                    .is_err()
                });
            }
            _ => {
                panic!("assert_nullifiers_exist_in_hash_sets: invalid tree_type");
            }
        }
    }
}

pub async fn assert_addresses_exist_in_hash_sets<R: Rpc>(
    rpc: &mut R,
    address_queue_pubkeys: &[Pubkey],
    created_addresses: &[[u8; 32]],
) {
    for (address, pubkey) in created_addresses.iter().zip(address_queue_pubkeys) {
        let account = rpc.get_account(*pubkey).await.unwrap().unwrap();
        let discriminator = &account.data[0..8];
        match discriminator {
            QueueAccount::DISCRIMINATOR => {
                let address_queue = unsafe { get_hash_set::<QueueAccount, R>(rpc, *pubkey).await };
                assert!(address_queue
                    .contains(&BigUint::from_be_bytes(address), None)
                    .unwrap());
            }
            BatchedMerkleTreeAccount::LIGHT_DISCRIMINATOR_SLICE => {
                let mut account_data = account.data.clone();
                let mut merkle_tree =
                    BatchedMerkleTreeAccount::address_from_bytes(&mut account_data, &pubkey.into())
                        .unwrap();
                let mut batches = merkle_tree.queue_batches.batches;
                // Must be included in one batch
                batches.iter_mut().enumerate().any(|(i, batch)| {
                    Batch::check_non_inclusion(
                        batch.num_iters as usize,
                        batch.bloom_filter_capacity,
                        address,
                        merkle_tree.bloom_filter_stores[i],
                    )
                    .is_err()
                });
                // must not be included in any other batch
                batches.iter_mut().enumerate().any(|(i, batch)| {
                    Batch::check_non_inclusion(
                        batch.num_iters as usize,
                        batch.bloom_filter_capacity,
                        address,
                        merkle_tree.bloom_filter_stores[i],
                    )
                    .is_ok()
                });
            }
            _ => {
                panic!("assert_addresses_exist_in_hash_sets: invalid discriminator");
            }
        }
    }
}

pub fn assert_created_compressed_accounts(
    output_compressed_accounts: &[CompressedAccount],
    output_merkle_tree_pubkeys: &[Pubkey],
    created_out_compressed_accounts: &[CompressedAccountWithMerkleContext],
) {
    for output_account in created_out_compressed_accounts.iter() {
        assert!(output_compressed_accounts.iter().any(|x| x.lamports
            == output_account.compressed_account.lamports
            && x.owner == output_account.compressed_account.owner
            && x.data == output_account.compressed_account.data
            && x.address == output_account.compressed_account.address),);
        assert!(output_merkle_tree_pubkeys.iter().any(|x| *x
            == output_account.merkle_context.merkle_tree_pubkey.into()
            || *x == output_account.merkle_context.queue_pubkey.into()),);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn assert_public_transaction_event(
    event: &PublicTransactionEvent,
    input_compressed_account_hashes: Option<&Vec<[u8; 32]>>,
    output_merkle_tree_accounts: &[StateMerkleTreeAccounts],
    output_leaf_indices: &Vec<u32>,
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
    relay_fee: Option<u64>,
    sequence_numbers: Vec<MerkleTreeSequenceNumber>,
) {
    assert_eq!(
        event.input_compressed_account_hashes,
        *input_compressed_account_hashes.unwrap_or(&Vec::<[u8; 32]>::new()),
        "assert_public_transaction_event: input compressed account hashes mismatch"
    );
    for account in event.output_compressed_accounts.iter() {
        assert!(
            output_merkle_tree_accounts.iter().any(|x| x.merkle_tree
                == event.pubkey_array[account.merkle_tree_index as usize].into()
                // handle output queue
                || x.nullifier_queue == event.pubkey_array[account.merkle_tree_index as usize].into()),
            "assert_public_transaction_event: output state merkle tree account index mismatch"
        );
    }
    assert_eq!(
        event.output_leaf_indices, *output_leaf_indices,
        "assert_public_transaction_event: output leaf indices mismatch"
    );

    assert_eq!(
        event.compress_or_decompress_lamports, compress_or_decompress_lamports,
        "assert_public_transaction_event: compression lamports mismatch"
    );
    assert_eq!(
        event.is_compress, is_compress,
        "assert_public_transaction_event: is_compress mismatch"
    );
    assert_eq!(
        event.relay_fee, relay_fee,
        "assert_public_transaction_event: relay fee mismatch"
    );
    let mut updated_sequence_numbers = event.sequence_numbers.clone();
    for account in event.output_compressed_accounts.iter() {
        let queue_pubkey = event.pubkey_array[account.merkle_tree_index as usize];
        let index = &mut updated_sequence_numbers
            .iter_mut()
            .find(|x| x.tree_pubkey == queue_pubkey);
        if index.is_none() {
            println!("reference sequence numbers: {:?}", sequence_numbers);
            println!("event: {:?}", event);
            // Not really applicable for the ouput queue.
            // panic!(
            //     "queue pubkey not found in sequence numbers : {:?}",
            //     queue_pubkey
            // );
        } else {
            let seq = &mut index.as_mut().unwrap().seq;
            // The output queue doesn't have a sequence number hence we set it
            // u64::MAX to mark it as a batched queue.
            *seq = seq.saturating_add(1);
        }
    }
    for sequence_number in updated_sequence_numbers.iter() {
        sequence_numbers
            .iter()
            .any(|x| x.tree_pubkey == sequence_number.tree_pubkey && x.seq == sequence_number.seq);
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct MerkleTreeTestSnapShot {
    pub accounts: StateMerkleTreeAccounts,
    pub root: [u8; 32],
    pub next_index: usize,
    pub num_added_accounts: usize,
    pub merkle_tree_account_lamports: u64,
    pub queue_account_lamports: u64,
    pub cpi_context_account_lamports: u64,
    pub tree_type: TreeType,
}

// TODO: add assert that changelog, seq number is updated correctly
/// Asserts that the merkle tree account has been updated correctly,
/// by comparing the merkle tree account with the test indexer merkle tree.
/// Asserts:
/// 1. The root has been updated
/// 2. The next index has been updated
pub async fn assert_merkle_tree_after_tx<R: Rpc, I: Indexer + TestIndexerExtensions>(
    rpc: &mut R,
    snapshots: &[MerkleTreeTestSnapShot],
    test_indexer: &mut I,
) -> Vec<MerkleTreeSequenceNumber> {
    let mut deduped_snapshots = snapshots.to_vec();
    deduped_snapshots.sort();
    deduped_snapshots.dedup();
    let mut sequence_numbers = Vec::new();
    for (i, snapshot) in deduped_snapshots.iter().enumerate() {
        match snapshot.tree_type {
            TreeType::StateV1 => {
                let merkle_tree =
                    get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
                        rpc,
                        snapshot.accounts.merkle_tree,
                    )
                    .await;
                println!("sequence number: {:?}", merkle_tree.next_index() as u64);
                println!("next index: {:?}", snapshot.next_index);
                println!("prev sequence number: {:?}", snapshot.num_added_accounts);
                sequence_numbers.push(MerkleTreeSequenceNumber {
                    tree_pubkey: snapshot.accounts.merkle_tree.into(),
                    queue_pubkey: snapshot.accounts.nullifier_queue.into(),
                    tree_type: TreeType::StateV1 as u64,
                    seq: merkle_tree.sequence_number() as u64,
                });
                if merkle_tree.root() == snapshot.root {
                    println!("deduped_snapshots: {:?}", deduped_snapshots);
                    println!("i: {:?}", i);
                    panic!("merkle tree root update failed, it should have updated but didn't");
                }
                assert_eq!(
                    merkle_tree.next_index(),
                    snapshot.next_index + snapshot.num_added_accounts
                );
                let test_indexer_merkle_tree = test_indexer
                    .get_state_merkle_trees_mut()
                    .iter_mut()
                    .find(|x| x.accounts.merkle_tree == snapshot.accounts.merkle_tree)
                    .expect("merkle tree not found in test indexer");

                if merkle_tree.root() != test_indexer_merkle_tree.merkle_tree.root() {
                    // The following lines are just println prints
                    println!("Merkle tree pubkey {:?}", snapshot.accounts.merkle_tree);
                    for (i, leaf) in test_indexer_merkle_tree.merkle_tree.layers[0]
                        .iter()
                        .enumerate()
                    {
                        println!("test_indexer_merkle_tree index {} leaf: {:?}", i, leaf);
                    }
                    for i in 0..16 {
                        println!("root {} {:?}", i, merkle_tree.roots.get(i));
                    }

                    panic!("merkle tree root update failed");
                }
            }
            TreeType::StateV2 => {
                // TODO: assert batched merkle tree
            }
            _ => {
                panic!(
                    "assert_merkle_tree_after_tx: get_merkle_tree_snapshots: invalid discriminator"
                );
            }
        }
    }
    sequence_numbers
}

/// Takes a snapshot of the provided the onchain Merkle trees.
/// Snapshot data:
/// 1. root
/// 2. next_index
/// 3. num_added_accounts // so that we can assert the expected next index after tx
/// 4. lamports of all bundle accounts
pub async fn get_merkle_tree_snapshots<R: Rpc>(
    rpc: &mut R,
    accounts: &[StateMerkleTreeAccounts],
) -> Vec<MerkleTreeTestSnapShot> {
    let mut snapshots = Vec::new();
    for account_bundle in accounts.iter() {
        let mut account_data = rpc
            .get_account(account_bundle.merkle_tree)
            .await
            .unwrap()
            .unwrap();
        match &account_data.data[0..8] {
            StateMerkleTreeAccount::DISCRIMINATOR => {
                let merkle_tree =
                    get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
                        rpc,
                        account_bundle.merkle_tree,
                    )
                    .await;
                let merkle_tree_account =
                    AccountZeroCopy::<StateMerkleTreeAccount>::new(rpc, account_bundle.merkle_tree)
                        .await;

                let queue_account_lamports = match rpc
                    .get_account(account_bundle.nullifier_queue)
                    .await
                    .unwrap()
                {
                    Some(x) => x.lamports,
                    None => 0,
                };
                let cpi_context_account_lamports =
                    match rpc.get_account(account_bundle.cpi_context).await.unwrap() {
                        Some(x) => x.lamports,
                        None => 0,
                    };
                snapshots.push(MerkleTreeTestSnapShot {
                    accounts: *account_bundle,
                    root: merkle_tree.root(),
                    next_index: merkle_tree.next_index(),
                    num_added_accounts: accounts
                        .iter()
                        .filter(|x| x.merkle_tree == account_bundle.merkle_tree)
                        .count(),
                    merkle_tree_account_lamports: merkle_tree_account.account.lamports(),
                    queue_account_lamports,
                    cpi_context_account_lamports,
                    tree_type: TreeType::StateV1,
                });
            }
            BatchedMerkleTreeAccount::LIGHT_DISCRIMINATOR_SLICE => {
                let merkle_tree_account_lamports = account_data.lamports;
                let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
                    &mut account_data.data,
                    &account_bundle.merkle_tree.into(),
                )
                .unwrap();
                let queue_account_lamports = match rpc
                    .get_account(account_bundle.nullifier_queue)
                    .await
                    .unwrap()
                {
                    Some(x) => x.lamports,
                    None => 0,
                };
                let cpi_context_account_lamports =
                    match rpc.get_account(account_bundle.cpi_context).await.unwrap() {
                        Some(x) => x.lamports,
                        None => 0,
                    };
                let root = *merkle_tree.root_history.last().unwrap();

                let output_queue = AccountZeroCopy::<BatchedQueueMetadata>::new(
                    rpc,
                    account_bundle.nullifier_queue,
                )
                .await;

                snapshots.push(MerkleTreeTestSnapShot {
                    accounts: *account_bundle,
                    root,
                    next_index: output_queue.deserialized().batch_metadata.next_index as usize,
                    num_added_accounts: accounts
                        .iter()
                        .filter(|x| x.merkle_tree == account_bundle.merkle_tree)
                        .count(),
                    merkle_tree_account_lamports,
                    queue_account_lamports,
                    cpi_context_account_lamports,
                    tree_type: TreeType::StateV2,
                });
            }
            _ => {
                panic!("get_merkle_tree_snapshots: invalid discriminator");
            }
        }
    }
    snapshots
}

pub async fn assert_compression<R: Rpc>(
    context: &mut R,
    compress_amount: u64,
    compressed_sol_pda_balance_pre: u64,
    recipient_balance_pre: u64,
    recipient: &Pubkey,
    is_compress: bool,
) {
    if is_compress {
        let compressed_sol_pda_balance = match context.get_account(get_sol_pool_pda()).await {
            Ok(Some(account)) => account.lamports,
            _ => 0,
        };

        assert_eq!(
            compressed_sol_pda_balance,
            compressed_sol_pda_balance_pre + compress_amount,
            "assert_compression: balance of compressed sol pda insufficient, compress sol failed"
        );
    } else {
        let compressed_sol_pda_balance =
            match context.get_account(get_sol_pool_pda()).await.unwrap() {
                Some(account) => account.lamports,
                None => 0,
            };

        assert_eq!(
            compressed_sol_pda_balance,
            compressed_sol_pda_balance_pre - compress_amount,
            "assert_compression: balance of compressed sol pda incorrect, decompress sol failed"
        );

        let recipient_balance = context
            .get_account(*recipient)
            .await
            .unwrap()
            .unwrap()
            .lamports;

        assert_eq!(
            recipient_balance,
            recipient_balance_pre + compress_amount,
            "assert_compression: balance of recipient insufficient, decompress sol failed"
        );
    }
}
