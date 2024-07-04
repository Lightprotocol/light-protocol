use solana_sdk::signature::Signature;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

use light_hasher::Poseidon;
use light_system_program::sdk::event::PublicTransactionEvent;
use light_system_program::{
    sdk::{
        address::derive_address,
        compressed_account::{
            CompressedAccount, CompressedAccountWithMerkleContext, MerkleContext,
        },
        invoke::{create_invoke_instruction, get_sol_pool_pda},
    },
    NewAddressParams,
};

use crate::assert_compressed_tx::{
    assert_compressed_transaction, get_merkle_tree_snapshots, AssertCompressedTransactionInputs,
};
use crate::indexer::Indexer;
use crate::rpc::errors::RpcError;
use crate::rpc::rpc_connection::RpcConnection;
use crate::transaction_params::TransactionParams;

#[allow(clippy::too_many_arguments)]
pub async fn create_addresses_test<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    test_indexer: &mut I,
    address_merkle_tree_pubkeys: &[Pubkey],
    address_merkle_tree_queue_pubkeys: &[Pubkey],
    mut output_merkle_tree_pubkeys: Vec<Pubkey>,
    address_seeds: &[[u8; 32]],
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
    create_out_compressed_accounts_for_input_compressed_accounts: bool,
    transaction_params: Option<TransactionParams>,
) -> Result<(), RpcError> {
    if address_merkle_tree_pubkeys.len() != address_seeds.len() {
        panic!("address_merkle_tree_pubkeys and address_seeds length mismatch for create_addresses_test");
    }
    let mut derived_addresses = Vec::new();
    for (i, address_seed) in address_seeds.iter().enumerate() {
        let derived_address =
            derive_address(&address_merkle_tree_pubkeys[i], address_seed).unwrap();
        println!("derived_address: {:?}", derived_address);
        derived_addresses.push(derived_address);
    }
    let mut address_params = Vec::new();

    for (i, seed) in address_seeds.iter().enumerate() {
        let new_address_params = NewAddressParams {
            address_queue_pubkey: address_merkle_tree_queue_pubkeys[i],
            address_merkle_tree_pubkey: address_merkle_tree_pubkeys[i],
            seed: *seed,
            address_merkle_tree_root_index: 0,
        };
        address_params.push(new_address_params);
    }

    let mut output_compressed_accounts = Vec::new();
    for address in derived_addresses.iter() {
        output_compressed_accounts.push(CompressedAccount {
            lamports: 0,
            owner: rpc.get_payer().pubkey(),
            data: None,
            address: Some(*address),
        });
    }

    if create_out_compressed_accounts_for_input_compressed_accounts {
        for compressed_account in input_compressed_accounts.iter() {
            output_compressed_accounts.push(CompressedAccount {
                lamports: 0,
                owner: rpc.get_payer().pubkey(),
                data: None,
                address: compressed_account.compressed_account.address,
            });
            output_merkle_tree_pubkeys.push(compressed_account.merkle_context.merkle_tree_pubkey);
        }
    }

    let payer = rpc.get_payer().insecure_clone();

    let inputs = CompressedTransactionTestInputs {
        rpc,
        test_indexer,
        fee_payer: &payer,
        authority: &payer,
        input_compressed_accounts,
        output_compressed_accounts: output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys: output_merkle_tree_pubkeys.as_slice(),
        transaction_params,
        relay_fee: None,
        compress_or_decompress_lamports: None,
        is_compress: false,
        new_address_params: &address_params,
        sorted_output_accounts: false,
        created_addresses: Some(derived_addresses.as_slice()),
        recipient: None,
    };
    compressed_transaction_test(inputs).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn compress_sol_test<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    test_indexer: &mut I,
    authority: &Keypair,
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
    create_out_compressed_accounts_for_input_compressed_accounts: bool,
    compress_amount: u64,
    output_merkle_tree_pubkey: &Pubkey,
    transaction_params: Option<TransactionParams>,
) -> Result<(), RpcError> {
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
            output_merkle_tree_pubkeys.push(compressed_account.merkle_context.merkle_tree_pubkey);
        }
    }
    let inputs = CompressedTransactionTestInputs {
        rpc,
        test_indexer,
        fee_payer: authority,
        authority,
        input_compressed_accounts,
        output_compressed_accounts: output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys: &[*output_merkle_tree_pubkey],
        transaction_params,
        relay_fee: None,
        compress_or_decompress_lamports: Some(compress_amount),
        is_compress: true,
        new_address_params: &[],
        sorted_output_accounts: false,
        created_addresses: None,
        recipient: None,
    };
    compressed_transaction_test(inputs).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn decompress_sol_test<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    test_indexer: &mut I,
    authority: &Keypair,
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
    recipient: &Pubkey,
    decompress_amount: u64,
    output_merkle_tree_pubkey: &Pubkey,
    transaction_params: Option<TransactionParams>,
) -> Result<(), RpcError> {
    let input_lamports = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.lamports)
        .sum::<u64>();

    let output_compressed_accounts = vec![CompressedAccount {
        lamports: input_lamports - decompress_amount,
        owner: rpc.get_payer().pubkey(),
        data: None,
        address: None,
    }];
    let payer = rpc.get_payer().insecure_clone();
    let inputs = CompressedTransactionTestInputs {
        rpc,
        test_indexer,
        fee_payer: &payer,
        authority,
        input_compressed_accounts,
        output_compressed_accounts: output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys: &[*output_merkle_tree_pubkey],
        transaction_params,
        relay_fee: None,
        compress_or_decompress_lamports: Some(decompress_amount),
        is_compress: false,
        new_address_params: &[],
        sorted_output_accounts: false,
        created_addresses: None,
        recipient: Some(*recipient),
    };
    compressed_transaction_test(inputs).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn transfer_compressed_sol_test<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    test_indexer: &mut I,
    authority: &Keypair,
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
    recipients: &[Pubkey],
    output_merkle_tree_pubkeys: &[Pubkey],
    transaction_params: Option<TransactionParams>,
) -> Result<Signature, RpcError> {
    if recipients.len() != output_merkle_tree_pubkeys.len() {
        panic!("recipients and output_merkle_tree_pubkeys length mismatch for transfer_compressed_sol_test");
    }

    if input_compressed_accounts.is_empty() {
        panic!("input_compressed_accounts is empty for transfer_compressed_sol_test");
    }
    let input_lamports = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.lamports)
        .sum::<u64>();
    let mut output_compressed_accounts = Vec::new();
    let mut output_merkle_tree_pubkeys = output_merkle_tree_pubkeys.to_vec();
    output_merkle_tree_pubkeys.sort();
    let input_addresses = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.address)
        .collect::<Vec<_>>();
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
            owner: recipients[i],
            data: None,
            address,
        });
    }
    let payer = rpc.get_payer().insecure_clone();
    let inputs = CompressedTransactionTestInputs {
        rpc,
        test_indexer,
        fee_payer: &payer,
        authority,
        input_compressed_accounts,
        output_compressed_accounts: output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys: output_merkle_tree_pubkeys.as_slice(),
        transaction_params,
        relay_fee: None,
        compress_or_decompress_lamports: None,
        is_compress: false,
        new_address_params: &[],
        sorted_output_accounts: false,
        created_addresses: None,
        recipient: None,
    };
    compressed_transaction_test(inputs).await
}

pub struct CompressedTransactionTestInputs<'a, R: RpcConnection, I: Indexer<R>> {
    rpc: &'a mut R,
    test_indexer: &'a mut I,
    fee_payer: &'a Keypair,
    authority: &'a Keypair,
    input_compressed_accounts: &'a [CompressedAccountWithMerkleContext],
    output_compressed_accounts: &'a [CompressedAccount],
    output_merkle_tree_pubkeys: &'a [Pubkey],
    transaction_params: Option<TransactionParams>,
    relay_fee: Option<u64>,
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
    new_address_params: &'a [NewAddressParams],
    sorted_output_accounts: bool,
    created_addresses: Option<&'a [[u8; 32]]>,
    recipient: Option<Pubkey>,
}

#[allow(clippy::too_many_arguments)]
pub async fn compressed_transaction_test<R: RpcConnection, I: Indexer<R>>(
    inputs: CompressedTransactionTestInputs<'_, R, I>,
) -> Result<Signature, RpcError> {
    let mut compressed_account_hashes = Vec::new();

    let compressed_account_input_hashes = if !inputs.input_compressed_accounts.is_empty() {
        for compressed_account in inputs.input_compressed_accounts.iter() {
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
    } else {
        None
    };
    let state_input_merkle_trees = inputs
        .input_compressed_accounts
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
    let mut input_merkle_tree_snapshots = Vec::new();
    let mut address_params = Vec::new();
    if !inputs.input_compressed_accounts.is_empty() || !inputs.new_address_params.is_empty() {
        let address_merkle_tree_pubkeys = if inputs.new_address_params.is_empty() {
            None
        } else {
            Some(
                inputs
                    .new_address_params
                    .iter()
                    .map(|x| x.address_merkle_tree_pubkey)
                    .collect::<Vec<_>>(),
            )
        };
        let proof_rpc_res = inputs
            .test_indexer
            .create_proof_for_compressed_accounts(
                compressed_account_input_hashes,
                state_input_merkle_trees,
                inputs.created_addresses,
                address_merkle_tree_pubkeys,
                inputs.rpc,
            )
            .await;
        root_indices = proof_rpc_res.root_indices;
        proof = Some(proof_rpc_res.proof);
        let input_merkle_tree_accounts = inputs
            .test_indexer
            .get_state_merkle_tree_accounts(state_input_merkle_trees.unwrap_or(&[]));
        input_merkle_tree_snapshots =
            get_merkle_tree_snapshots::<R>(inputs.rpc, input_merkle_tree_accounts.as_slice()).await;

        if !inputs.new_address_params.is_empty() {
            for (i, input_address_params) in inputs.new_address_params.iter().enumerate() {
                address_params.push(input_address_params.clone());
                address_params[i].address_merkle_tree_root_index =
                    proof_rpc_res.address_root_indices[i];
            }
        }
    }

    let output_merkle_tree_accounts = inputs
        .test_indexer
        .get_state_merkle_tree_accounts(inputs.output_merkle_tree_pubkeys);
    let output_merkle_tree_snapshots =
        get_merkle_tree_snapshots::<R>(inputs.rpc, output_merkle_tree_accounts.as_slice()).await;
    let instruction = create_invoke_instruction(
        &inputs.fee_payer.pubkey(),
        &inputs.authority.pubkey().clone(),
        inputs
            .input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<CompressedAccount>>()
            .as_slice(),
        inputs.output_compressed_accounts,
        inputs
            .input_compressed_accounts
            .iter()
            .map(|x| x.merkle_context)
            .collect::<Vec<MerkleContext>>()
            .as_slice(),
        inputs.output_merkle_tree_pubkeys,
        &root_indices,
        &address_params,
        proof,
        inputs.compress_or_decompress_lamports,
        inputs.is_compress,
        inputs.recipient,
        true,
    );
    let mut recipient_balance_pre = 0;
    let mut compressed_sol_pda_balance_pre = 0;
    if inputs.compress_or_decompress_lamports.is_some() {
        compressed_sol_pda_balance_pre =
            match inputs.rpc.get_account(get_sol_pool_pda()).await.unwrap() {
                Some(account) => account.lamports,
                None => 0,
            };
    }
    if inputs.recipient.is_some() {
        // TODO: assert sender balance after fee refactor
        recipient_balance_pre = match inputs
            .rpc
            .get_account(inputs.recipient.unwrap())
            .await
            .unwrap()
        {
            Some(account) => account.lamports,
            None => 0,
        };
    }
    let event = inputs
        .rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &inputs.fee_payer.pubkey(),
            &[inputs.fee_payer, inputs.authority],
            inputs.transaction_params,
        )
        .await?
        .unwrap();

    let (created_output_compressed_accounts, _) = inputs
        .test_indexer
        .add_event_and_compressed_accounts(&event.0);
    let input = AssertCompressedTransactionInputs {
        rpc: inputs.rpc,
        test_indexer: inputs.test_indexer,
        output_compressed_accounts: inputs.output_compressed_accounts,
        created_output_compressed_accounts: created_output_compressed_accounts.as_slice(),
        event: &event.0,
        input_merkle_tree_snapshots: input_merkle_tree_snapshots.as_slice(),
        output_merkle_tree_snapshots: output_merkle_tree_snapshots.as_slice(),
        recipient_balance_pre,
        compress_or_decompress_lamports: inputs.compress_or_decompress_lamports,
        is_compress: inputs.is_compress,
        compressed_sol_pda_balance_pre,
        compression_recipient: inputs.recipient,
        created_addresses: inputs.created_addresses.unwrap_or(&[]),
        sorted_output_accounts: inputs.sorted_output_accounts,
        relay_fee: inputs.relay_fee,
        input_compressed_account_hashes: &compressed_account_hashes,
        address_queue_pubkeys: &inputs
            .new_address_params
            .iter()
            .map(|x| x.address_queue_pubkey)
            .collect::<Vec<Pubkey>>(),
    };
    assert_compressed_transaction(input).await;
    Ok(event.1)
}
