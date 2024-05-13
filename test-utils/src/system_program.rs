use light_compressed_pda::{
    sdk::{
        address::derive_address,
        compressed_account::{
            CompressedAccount, CompressedAccountWithMerkleContext, MerkleContext,
        },
        event::PublicTransactionEvent,
        invoke::{create_invoke_instruction, get_compressed_sol_pda},
    },
    NewAddressParams,
};
use light_hasher::Poseidon;
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

use crate::{create_and_send_transaction_with_event, test_indexer::TestIndexer, TransactionParams};

#[allow(clippy::too_many_arguments)]
pub async fn create_addresses(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    address_merkle_tree_pubkeys: &[Pubkey],
    address_merkle_tree_queue_pubkeys: &[Pubkey],
    output_merkle_tree_pubkeys: &[Pubkey],
    address_seeds: &[[u8; 32]],
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
    create_out_compressed_accounts_for_input_compressed_accounts: bool,
    transaction_params: Option<TransactionParams>,
) -> Result<(), BanksClientError> {
    let mut derived_addresses = Vec::new();
    for (i, address_seed) in address_seeds.iter().enumerate() {
        let derived_address =
            derive_address(&address_merkle_tree_pubkeys[i], address_seed).unwrap();
        derived_addresses.push(derived_address);
    }
    let mut compressed_account_hashes = Vec::new();

    let compressed_account_input_hashes = if input_compressed_accounts.is_empty() {
        None
    } else {
        for compressed_account in input_compressed_accounts.iter() {
            compressed_account_hashes.push(
                compressed_account
                    .compressed_account
                    .hash::<Poseidon>(
                        &compressed_account.merkle_context.merkle_tree_pubkey,
                        &compressed_account.merkle_context.leaf_index,
                    )
                    .unwrap(),
            );
        }
        Some(compressed_account_hashes.as_slice())
    };
    let state_input_merkle_trees = input_compressed_accounts
        .iter()
        .map(|x| x.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<Pubkey>>();
    let state_input_merkle_trees = if state_input_merkle_trees.is_empty() {
        None
    } else {
        Some(state_input_merkle_trees.as_slice())
    };
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            compressed_account_input_hashes,
            state_input_merkle_trees,
            Some(derived_addresses.as_slice()),
            Some(address_merkle_tree_pubkeys),
            context,
        )
        .await;
    let mut address_params = Vec::new();

    for (i, seed) in address_seeds.iter().enumerate() {
        let new_address_params = NewAddressParams {
            address_queue_pubkey: address_merkle_tree_queue_pubkeys[i],
            address_merkle_tree_pubkey: address_merkle_tree_pubkeys[i],
            seed: *seed,
            address_merkle_tree_root_index: proof_rpc_res.address_root_indices[i],
        };
        address_params.push(new_address_params);
    }

    let mut output_compressed_accounts = Vec::new();
    for (i, address_param) in address_params.iter().enumerate() {
        output_compressed_accounts.push(CompressedAccount {
            lamports: 0,
            owner: context.payer.pubkey(),
            data: None,
            address: Some(
                derive_address(&address_merkle_tree_pubkeys[i], &address_param.seed).unwrap(),
            ),
        });
    }

    if create_out_compressed_accounts_for_input_compressed_accounts {
        for compressed_account in input_compressed_accounts.iter() {
            output_compressed_accounts.push(CompressedAccount {
                lamports: 0,
                owner: context.payer.pubkey(),
                data: None,
                address: compressed_account.compressed_account.address,
            });
        }
    }

    let instruction = create_invoke_instruction(
        &context.payer.pubkey(),
        &context.payer.pubkey().clone(),
        input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<CompressedAccount>>()
            .as_slice(),
        &output_compressed_accounts,
        input_compressed_accounts
            .iter()
            .map(|x| x.merkle_context.clone())
            .collect::<Vec<MerkleContext>>()
            .as_slice(),
        &output_merkle_tree_pubkeys,
        &proof_rpc_res.root_indices,
        &address_params,
        Some(proof_rpc_res.proof.clone()),
        None,
        false,
        None,
    );

    let event = create_and_send_transaction_with_event::<PublicTransactionEvent>(
        context,
        &[instruction],
        &context.payer.pubkey(),
        &[&context.payer.insecure_clone()],
        transaction_params,
    )
    .await;

    let (created_out_compressed_accounts, _) =
        test_indexer.add_event_and_compressed_accounts(event?.unwrap());
    assert_created_compressed_accounts(
        &output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys,
        created_out_compressed_accounts.as_slice(),
        false,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn compress_sol_test(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    authority: &Keypair,
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
    create_out_compressed_accounts_for_input_compressed_accounts: bool,
    compress_amount: u64,
    output_merkle_tree_pubkey: &Pubkey,
    transaction_params: Option<TransactionParams>,
) -> Result<(), BanksClientError> {
    let mut compressed_account_hashes = Vec::new();

    let compressed_account_input_hashes = if input_compressed_accounts.is_empty() {
        None
    } else {
        for compressed_account in input_compressed_accounts.iter() {
            compressed_account_hashes.push(
                compressed_account
                    .compressed_account
                    .hash::<Poseidon>(
                        &compressed_account.merkle_context.merkle_tree_pubkey,
                        &compressed_account.merkle_context.leaf_index,
                    )
                    .unwrap(),
            );
        }
        Some(compressed_account_hashes.as_slice())
    };
    let state_input_merkle_trees = input_compressed_accounts
        .iter()
        .map(|x| x.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<Pubkey>>();
    let state_input_merkle_trees = if state_input_merkle_trees.is_empty() {
        None
    } else {
        Some(state_input_merkle_trees.as_slice())
    };
    let mut root_indices = Vec::new();
    let mut proof = None;
    if !input_compressed_accounts.is_empty() {
        let proof_rpc_res = test_indexer
            .create_proof_for_compressed_accounts(
                compressed_account_input_hashes,
                state_input_merkle_trees,
                None,
                None,
                context,
            )
            .await;
        root_indices = proof_rpc_res.root_indices;
        proof = Some(proof_rpc_res.proof);
    }

    let input_lamports = if input_compressed_accounts.is_empty() {
        0
    } else {
        input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.lamports)
            .sum::<u64>()
    };
    let mut output_compressed_accounts = Vec::new();
    output_compressed_accounts.push(CompressedAccount {
        lamports: input_lamports + compress_amount,
        owner: authority.pubkey(),
        data: None,
        address: None,
    });
    let mut output_merkle_tree_pubkeys = vec![*output_merkle_tree_pubkey];
    if create_out_compressed_accounts_for_input_compressed_accounts {
        for compressed_account in input_compressed_accounts.iter() {
            output_compressed_accounts.push(CompressedAccount {
                lamports: 0,
                owner: authority.pubkey(),
                data: None,
                address: compressed_account.compressed_account.address,
            });
            output_merkle_tree_pubkeys
                .push(compressed_account.merkle_context.merkle_tree_pubkey.clone());
        }
    }
    println!("input_compressed_accounts: {:?}", input_compressed_accounts);

    let instruction = create_invoke_instruction(
        &authority.pubkey(),
        &authority.pubkey().clone(),
        input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<CompressedAccount>>()
            .as_slice(),
        &output_compressed_accounts,
        input_compressed_accounts
            .iter()
            .map(|x| x.merkle_context.clone())
            .collect::<Vec<MerkleContext>>()
            .as_slice(),
        output_merkle_tree_pubkeys.as_slice(),
        &root_indices,
        &Vec::new(),
        proof,
        Some(compress_amount),
        true,
        Some(get_compressed_sol_pda()),
    );
    // TODO: assert sender balance after fee refactor
    // let sender_pre_balance = context
    //     .banks_client
    //     .get_account(authority.pubkey())
    //     .await
    //     .unwrap()
    //     .unwrap()
    //     .lamports;
    let compressed_sol_pda_balance_pre = match context
        .banks_client
        .get_account(get_compressed_sol_pda())
        .await
        .unwrap()
    {
        Some(account) => account.lamports,
        None => 0,
    };

    let event = create_and_send_transaction_with_event::<PublicTransactionEvent>(
        context,
        &[instruction],
        &authority.pubkey(),
        &[&authority],
        transaction_params,
    )
    .await;

    let (created_compressed_accounts, _) =
        test_indexer.add_event_and_compressed_accounts(event?.unwrap());
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
        "balance of compressed sol pda insufficient, compress sol failed"
    );
    assert_created_compressed_accounts(
        &output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys.as_slice(),
        created_compressed_accounts.as_slice(),
        false,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn decompress_sol_test(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    authority: &Keypair,
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
    recipient: &Pubkey,
    decompress_amount: u64,
    output_merkle_tree_pubkey: &Pubkey,
    transaction_params: Option<TransactionParams>,
) -> Result<(), BanksClientError> {
    let mut compressed_account_hashes = Vec::new();

    let compressed_account_input_hashes = if input_compressed_accounts.is_empty() {
        panic!("input_compressed_accounts is empty for decompress_sol_test");
    } else {
        for compressed_account in input_compressed_accounts.iter() {
            compressed_account_hashes.push(
                compressed_account
                    .compressed_account
                    .hash::<Poseidon>(
                        &compressed_account.merkle_context.merkle_tree_pubkey,
                        &compressed_account.merkle_context.leaf_index,
                    )
                    .unwrap(),
            );
        }
        Some(compressed_account_hashes.as_slice())
    };
    let state_input_merkle_trees = input_compressed_accounts
        .iter()
        .map(|x| x.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<Pubkey>>();
    let state_input_merkle_trees = if state_input_merkle_trees.is_empty() {
        None
    } else {
        Some(state_input_merkle_trees.as_slice())
    };
    let mut root_indices = Vec::new();
    let mut proof = None;
    if !input_compressed_accounts.is_empty() {
        let proof_rpc_res = test_indexer
            .create_proof_for_compressed_accounts(
                compressed_account_input_hashes,
                state_input_merkle_trees,
                None,
                None,
                context,
            )
            .await;
        root_indices = proof_rpc_res.root_indices;
        proof = Some(proof_rpc_res.proof);
    }

    let input_lamports = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.lamports)
        .sum::<u64>();

    let mut output_compressed_accounts = Vec::new();
    output_compressed_accounts.push(CompressedAccount {
        lamports: input_lamports - decompress_amount,
        owner: context.payer.pubkey(),
        data: None,
        address: None,
    });
    let output_merkle_tree_pubkeys = vec![*output_merkle_tree_pubkey];

    let instruction = create_invoke_instruction(
        &context.payer.pubkey(),
        &authority.pubkey().clone(),
        input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<CompressedAccount>>()
            .as_slice(),
        &output_compressed_accounts,
        input_compressed_accounts
            .iter()
            .map(|x| x.merkle_context.clone())
            .collect::<Vec<MerkleContext>>()
            .as_slice(),
        output_merkle_tree_pubkeys.as_slice(),
        &root_indices,
        &Vec::new(),
        proof,
        Some(decompress_amount),
        false,
        Some(*recipient),
    );
    // TODO: assert sender balance after fee refactor
    let recipient_balance_pre = context
        .banks_client
        .get_account(*recipient)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let compressed_sol_pda_balance_pre = match context
        .banks_client
        .get_account(get_compressed_sol_pda())
        .await
        .unwrap()
    {
        Some(account) => account.lamports,
        None => 0,
    };

    let event = create_and_send_transaction_with_event::<PublicTransactionEvent>(
        context,
        &[instruction],
        &context.payer.pubkey(),
        &[&context.payer.insecure_clone(), authority],
        transaction_params,
    )
    .await;

    let (created_compressed_accounts, _) =
        test_indexer.add_event_and_compressed_accounts(event?.unwrap());
    let compressed_sol_pda_balance = context
        .banks_client
        .get_account(get_compressed_sol_pda())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert_eq!(
        compressed_sol_pda_balance,
        compressed_sol_pda_balance_pre - decompress_amount,
        "balance of compressed sol pda incorrect, decompress sol failed"
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
        recipient_balance_pre + decompress_amount,
        "balance of recipient insufficient, decompress sol failed"
    );

    assert_created_compressed_accounts(
        &output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys.as_slice(),
        created_compressed_accounts.as_slice(),
        false,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn transfer_compressed_sol_test(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    authority: &Keypair,
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
    output_merkle_tree_pubkeys: &[Pubkey],
    transaction_params: Option<TransactionParams>,
) -> Result<(), BanksClientError> {
    let mut compressed_account_hashes = Vec::new();

    let compressed_account_input_hashes = if input_compressed_accounts.is_empty() {
        panic!("input_compressed_accounts is empty for transfer_compressed_sol_test");
    } else {
        for compressed_account in input_compressed_accounts.iter() {
            compressed_account_hashes.push(
                compressed_account
                    .compressed_account
                    .hash::<Poseidon>(
                        &compressed_account.merkle_context.merkle_tree_pubkey,
                        &compressed_account.merkle_context.leaf_index,
                    )
                    .unwrap(),
            );
        }
        Some(compressed_account_hashes.as_slice())
    };
    let state_input_merkle_trees = input_compressed_accounts
        .iter()
        .map(|x| x.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<Pubkey>>();
    let state_input_merkle_trees = if state_input_merkle_trees.is_empty() {
        None
    } else {
        Some(state_input_merkle_trees.as_slice())
    };
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            compressed_account_input_hashes,
            state_input_merkle_trees,
            None,
            None,
            context,
        )
        .await;
    let input_addresses = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.address)
        .collect::<Vec<_>>();
    let input_lamports = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.lamports)
        .sum::<u64>();
    let mut output_compressed_accounts = Vec::new();
    let mut output_merkle_tree_pubkeys = output_merkle_tree_pubkeys.to_vec();
    output_merkle_tree_pubkeys.sort();
    for (i, _) in output_merkle_tree_pubkeys.iter().enumerate() {
        let address = if i < input_addresses.len() {
            input_addresses[i]
        } else {
            None
        };
        let mut lamports = input_lamports / output_merkle_tree_pubkeys.len() as u64;
        if i == 0 {
            lamports += input_lamports % output_merkle_tree_pubkeys.len() as u64;
        }

        output_compressed_accounts.push(CompressedAccount {
            lamports,
            owner: authority.pubkey(),
            data: None,
            address,
        });
    }
    println!("input_compressed_accounts: {:?}", input_compressed_accounts);
    println!(
        "output_compressed_accounts: {:?}",
        output_compressed_accounts
    );
    let instruction = create_invoke_instruction(
        &context.payer.pubkey(),
        &authority.pubkey().clone(),
        input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<CompressedAccount>>()
            .as_slice(),
        &output_compressed_accounts,
        input_compressed_accounts
            .iter()
            .map(|x| x.merkle_context.clone())
            .collect::<Vec<MerkleContext>>()
            .as_slice(),
        &output_merkle_tree_pubkeys,
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        None,
        false,
        None,
    );

    let event = create_and_send_transaction_with_event::<PublicTransactionEvent>(
        context,
        &[instruction],
        &context.payer.pubkey(),
        &[&context.payer.insecure_clone(), authority],
        transaction_params,
    )
    .await;

    let (created_out_compressed_accounts, _) =
        test_indexer.add_event_and_compressed_accounts(event?.unwrap());

    assert_created_compressed_accounts(
        &output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys.as_slice(),
        created_out_compressed_accounts.as_slice(),
        true,
    );
    Ok(())
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
        for (_, output_account) in created_out_compressed_accounts.iter().enumerate() {
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
