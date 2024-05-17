use crate::{
    get_hash_set,
    test_indexer::{StateMerkleTreeAccounts, TestIndexer},
    AccountZeroCopy,
};
use account_compression::{
    initialize_nullifier_queue::NullifierQueueAccount, AddressQueueAccount, StateMerkleTreeAccount,
};
use light_system_program::sdk::{
    compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
    event::PublicTransactionEvent,
    invoke::get_compressed_sol_pda,
};
use num_bigint::BigUint;
use num_traits::FromBytes;
use solana_program_test::ProgramTestContext;
use solana_sdk::account::ReadableAccount;
use solana_sdk::pubkey::Pubkey;

pub struct AssertCompressedTransactionInputs<'a, const INDEXED_ARRAY_SIZE: usize> {
    pub context: &'a mut ProgramTestContext,
    pub test_indexer: &'a mut TestIndexer<INDEXED_ARRAY_SIZE>,
    pub output_compressed_accounts: &'a [CompressedAccount],
    pub created_output_compressed_accounts: &'a [CompressedAccountWithMerkleContext],
    pub input_compressed_account_hashes: &'a [[u8; 32]],
    pub output_merkle_tree_snapshots: &'a [MerkleTreeTestSnapShot],
    pub input_merkle_tree_snapshots: &'a [MerkleTreeTestSnapShot],
    pub created_addresses: &'a [[u8; 32]],
    pub address_queue_pubkeys: &'a [Pubkey],
    pub event: &'a PublicTransactionEvent,
    pub sorted_output_accounts: bool,
    pub compression_lamports: Option<u64>,
    pub is_compress: bool,
    pub relay_fee: Option<u64>,
    pub compression_recipient: Option<Pubkey>,
    pub recipient_balance_pre: u64,
    pub compressed_sol_pda_balance_pre: u64,
}

/// General tx assert:
/// 1. ouputs created
/// 2. inputs nullified
/// 3. addressed inserted into address queue
/// 4. Public Transaction event emitted correctly
/// 5. Merkle tree was updated correctly
/// 6. TODO: Fees have been paid (after fee refactor)
/// 7. Check compression amount was transferred
pub async fn assert_compressed_transaction<const INDEXED_ARRAY_SIZE: usize>(
    input: AssertCompressedTransactionInputs<'_, INDEXED_ARRAY_SIZE>,
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
        input.sorted_output_accounts,
    );
    // CHECK 2
    assert_nullifiers_exist_in_hash_sets(
        input.context,
        input.input_merkle_tree_snapshots,
        input.input_compressed_account_hashes,
    )
    .await;

    // CHECK 3
    assert_addresses_exist_in_hash_sets(
        input.context,
        input.address_queue_pubkeys,
        input.created_addresses,
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
        input.compression_lamports,
        input.is_compress,
        input.relay_fee,
    );
    // CHECK 5
    assert_merkle_tree_after_tx(
        input.context,
        input.output_merkle_tree_snapshots,
        input.test_indexer,
    )
    .await;

    // CHECK 7
    if let Some(compression_lamports) = input.compression_lamports {
        assert_compression(
            input.context,
            compression_lamports,
            input.compressed_sol_pda_balance_pre,
            input.recipient_balance_pre,
            &input.compression_recipient.unwrap_or_default(),
            input.is_compress,
        )
        .await;
    }
}

pub async fn assert_nullifiers_exist_in_hash_sets(
    context: &mut ProgramTestContext,
    snapshots: &[MerkleTreeTestSnapShot],
    input_compressed_account_hashes: &[[u8; 32]],
) {
    for (i, hash) in input_compressed_account_hashes.iter().enumerate() {
        let nullifier_queue = unsafe {
            get_hash_set::<u16, NullifierQueueAccount>(
                context,
                snapshots[i].accounts.nullifier_queue,
            )
            .await
        };
        assert!(nullifier_queue
            .contains(&BigUint::from_be_bytes(hash.as_slice()), None)
            .unwrap());
    }
}

pub async fn assert_addresses_exist_in_hash_sets(
    context: &mut ProgramTestContext,
    address_queue_pubkeys: &[Pubkey],
    created_addresses: &[[u8; 32]],
) {
    for (address, pubkey) in created_addresses.iter().zip(address_queue_pubkeys) {
        let address_queue =
            unsafe { get_hash_set::<u16, AddressQueueAccount>(context, *pubkey).await };
        assert!(address_queue
            .contains(&BigUint::from_be_bytes(address), None)
            .unwrap());
    }
}

pub fn assert_created_compressed_accounts(
    output_compressed_accounts: &[CompressedAccount],
    output_merkle_tree_pubkeys: &[Pubkey],
    created_out_compressed_accounts: &[CompressedAccountWithMerkleContext],
    sorted: bool,
) {
    if !sorted {
        for (i, output_account) in created_out_compressed_accounts.iter().enumerate() {
            assert_eq!(
                output_account.compressed_account.lamports, output_compressed_accounts[i].lamports,
                "lamports mismatch"
            );
            assert_eq!(
                output_account.compressed_account.owner, output_compressed_accounts[i].owner,
                "owner mismatch"
            );
            assert_eq!(
                output_account.compressed_account.data, output_compressed_accounts[i].data,
                "data mismatch"
            );
            assert_eq!(
                output_account.compressed_account.address, output_compressed_accounts[i].address,
                "address mismatch"
            );
            assert_eq!(
                output_account.merkle_context.merkle_tree_pubkey, output_merkle_tree_pubkeys[i],
                "merkle tree pubkey mismatch"
            );
        }
    } else {
        for output_account in created_out_compressed_accounts.iter() {
            assert!(output_compressed_accounts
                .iter()
                .any(|x| x.lamports == output_account.compressed_account.lamports),);
            assert!(output_compressed_accounts
                .iter()
                .any(|x| x.owner == output_account.compressed_account.owner),);
            assert!(output_compressed_accounts
                .iter()
                .any(|x| x.data == output_account.compressed_account.data),);
            assert!(output_compressed_accounts
                .iter()
                .any(|x| x.address == output_account.compressed_account.address),);
            assert!(output_merkle_tree_pubkeys
                .iter()
                .any(|x| *x == output_account.merkle_context.merkle_tree_pubkey),);
        }
    }
}

pub fn assert_public_transaction_event(
    event: &PublicTransactionEvent,
    input_compressed_account_hashes: Option<&Vec<[u8; 32]>>,
    output_merkle_tree_accounts: &[StateMerkleTreeAccounts],
    output_leaf_indices: &Vec<u32>,
    compression_lamports: Option<u64>,
    is_compress: bool,
    relay_fee: Option<u64>,
) {
    assert_eq!(
        event.input_compressed_account_hashes,
        *input_compressed_account_hashes.unwrap_or(&Vec::<[u8; 32]>::new()),
        "assert_public_transaction_event: input compressed account hashes mismatch"
    );
    for index in event.output_state_merkle_tree_account_indices.iter() {
        assert_eq!(
            event.pubkey_array[*index as usize],
            output_merkle_tree_accounts[*index as usize].merkle_tree,
            "assert_public_transaction_event: output state merkle tree account index mismatch"
        );
    }
    assert_eq!(
        event.output_leaf_indices, *output_leaf_indices,
        "assert_public_transaction_event: output leaf indices mismatch"
    );

    assert_eq!(
        event.compression_lamports, compression_lamports,
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
}

// TODO: add assert that changelog, seq number is updated correctly
/// Asserts that the merkle tree account has been updated correctly,
/// by comparing the merkle tree account with the test indexer merkle tree.
/// Asserts:
/// 1. The root has been updated
/// 2. The next index has been updated
pub async fn assert_merkle_tree_after_tx<const INDEXED_ARRAY_SIZE: usize>(
    context: &mut ProgramTestContext,
    snapshots: &[MerkleTreeTestSnapShot],
    test_indexer: &mut TestIndexer<INDEXED_ARRAY_SIZE>,
) {
    let mut deduped_snapshots = snapshots.to_vec();
    deduped_snapshots.sort();
    deduped_snapshots.dedup();
    for (i, snapshot) in deduped_snapshots.iter().enumerate() {
        let merkle_tree_account =
            AccountZeroCopy::<StateMerkleTreeAccount>::new(context, snapshot.accounts.merkle_tree)
                .await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        if merkle_tree.root() == snapshot.root {
            println!("deduped_snapshots: {:?}", deduped_snapshots);
            println!("i: {:?}", i);
            panic!("merkle tree root update failed");
        }
        assert_eq!(
            merkle_tree.next_index(),
            snapshot.next_index + snapshot.num_added_accounts
        );
        let test_indexer_merkle_tree = test_indexer
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == snapshot.accounts.merkle_tree)
            .expect("merkle tree not found in test indexer");

        if merkle_tree.root() != test_indexer_merkle_tree.merkle_tree.root() {
            // The following lines are just debug prints
            println!("Merkle tree pubkey {:?}", snapshot.accounts.merkle_tree);
            for (i, leaf) in test_indexer_merkle_tree.merkle_tree.layers[0]
                .iter()
                .enumerate()
            {
                println!("test_indexer_merkle_tree index {} leaf: {:?}", i, leaf);
            }
            let merkle_tree_roots = merkle_tree_account.deserialized().load_roots().unwrap();
            for i in 0..16 {
                println!("root {} {:?}", i, merkle_tree_roots.get(i));
            }
            for i in 0..5 {
                test_indexer_merkle_tree
                    .merkle_tree
                    .update(&[0u8; 32], 15 - i)
                    .unwrap();
                println!(
                    "roll back root {} {:?}",
                    15 - i,
                    test_indexer_merkle_tree.merkle_tree.root()
                );
            }

            panic!("merkle tree root update failed");
        }
    }
}

/// Takes a snapshot of the provided the onchain Merkle trees.
/// Snapshot data:
/// 1. root
/// 2. next_index
/// 3. num_added_accounts // so that we can assert the expected next index after tx
/// 4. lamports of all bundle accounts
pub async fn get_merkle_tree_snapshots<const INDEXED_ARRAY_SIZE: usize>(
    context: &mut ProgramTestContext,
    accounts: &[StateMerkleTreeAccounts],
) -> Vec<MerkleTreeTestSnapShot> {
    let mut snapshots = Vec::new();
    for account_bundle in accounts.iter() {
        let merkle_tree_account =
            AccountZeroCopy::<StateMerkleTreeAccount>::new(context, account_bundle.merkle_tree)
                .await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        let queue_account_lamports = match context
            .banks_client
            .get_account(account_bundle.nullifier_queue)
            .await
            .unwrap()
        {
            Some(x) => x.lamports,
            None => 0,
        };
        let cpi_context_account_lamports = match context
            .banks_client
            .get_account(account_bundle.cpi_context)
            .await
            .unwrap()
        {
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
        });
    }
    snapshots
}

pub async fn assert_compression(
    context: &mut ProgramTestContext,
    compress_amount: u64,
    compressed_sol_pda_balance_pre: u64,
    recipient_balance_pre: u64,
    recipient: &Pubkey,
    is_compress: bool,
) {
    if is_compress {
        let compressed_sol_pda_balance = context
            .banks_client
            .get_account(get_compressed_sol_pda())
            .await
            .unwrap()
            .unwrap()
            .lamports;

        assert_eq!(
            compressed_sol_pda_balance,
            compressed_sol_pda_balance_pre + compress_amount,
            "assert_compression: balance of compressed sol pda insufficient, compress sol failed"
        );
    } else {
        let compressed_sol_pda_balance = match context
            .banks_client
            .get_account(get_compressed_sol_pda())
            .await
            .unwrap()
        {
            Some(account) => account.lamports,
            None => 0,
        };

        assert_eq!(
            compressed_sol_pda_balance,
            compressed_sol_pda_balance_pre - compress_amount,
            "assert_compression: balance of compressed sol pda incorrect, decompress sol failed"
        );

        let recipient_balance = context
            .banks_client
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
