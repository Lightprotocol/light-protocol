use std::collections::HashMap;

use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
use light_client::{
    fee::TransactionParams,
    indexer::{AddressWithTree, Indexer},
    rpc::{errors::RpcError, Rpc},
};
use light_compressed_account::{
    address::derive_address_legacy,
    compressed_account::{
        CompressedAccount, CompressedAccountWithMerkleContext, MerkleContext,
        PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
    },
    instruction_data::{
        compressed_proof::CompressedProof,
        data::{
            InstructionDataInvoke, NewAddressParams, NewAddressParamsPacked,
            OutputCompressedAccountWithPackedContext,
        },
    },
};
use light_program_test::{indexer::TestIndexerExtensions, program_test::test_rpc::TestRpc};
use light_system_program::{
    constants::SOL_POOL_PDA_SEED,
    utils::{get_cpi_authority_pda, get_registered_program_pda},
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};

use crate::assert_compressed_tx::{
    assert_compressed_transaction, get_merkle_tree_snapshots, AssertCompressedTransactionInputs,
};

#[allow(clippy::too_many_arguments)]
pub async fn create_addresses_test<
    R: Rpc + TestRpc + Indexer,
    I: Indexer + TestIndexerExtensions,
>(
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
            derive_address_legacy(&address_merkle_tree_pubkeys[i].into(), address_seed).unwrap();
        derived_addresses.push(derived_address);
    }
    let mut address_params = Vec::new();

    for (i, seed) in address_seeds.iter().enumerate() {
        let new_address_params = NewAddressParams {
            address_queue_pubkey: address_merkle_tree_queue_pubkeys[i].into(),
            address_merkle_tree_pubkey: address_merkle_tree_pubkeys[i].into(),
            seed: *seed,
            address_merkle_tree_root_index: 0,
        };
        address_params.push(new_address_params);
    }

    let mut output_compressed_accounts = Vec::new();
    for address in derived_addresses.iter() {
        output_compressed_accounts.push(CompressedAccount {
            lamports: 0,
            owner: rpc.get_payer().pubkey().into(),
            data: None,
            address: Some(*address),
        });
    }

    if create_out_compressed_accounts_for_input_compressed_accounts {
        for compressed_account in input_compressed_accounts.iter() {
            output_compressed_accounts.push(CompressedAccount {
                lamports: 0,
                owner: rpc.get_payer().pubkey().into(),
                data: None,
                address: compressed_account.compressed_account.address,
            });
            output_merkle_tree_pubkeys
                .push(compressed_account.merkle_context.merkle_tree_pubkey.into());
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
pub async fn compress_sol_test<R: Rpc + TestRpc + Indexer, I: Indexer + TestIndexerExtensions>(
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
        owner: authority.pubkey().into(),
        data: None,
        address: None,
    });
    let mut output_merkle_tree_pubkeys = vec![*output_merkle_tree_pubkey];
    if create_out_compressed_accounts_for_input_compressed_accounts {
        for compressed_account in input_compressed_accounts.iter() {
            output_compressed_accounts.push(CompressedAccount {
                lamports: 0,
                owner: authority.pubkey().into(),
                data: None,
                address: compressed_account.compressed_account.address,
            });
            output_merkle_tree_pubkeys
                .push(compressed_account.merkle_context.merkle_tree_pubkey.into());
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
pub async fn decompress_sol_test<R: Rpc + TestRpc + Indexer, I: Indexer + TestIndexerExtensions>(
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
        owner: rpc.get_payer().pubkey().into(),
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
pub async fn transfer_compressed_sol_test<
    R: Rpc + TestRpc + Indexer,
    I: Indexer + TestIndexerExtensions,
>(
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
            owner: recipients[i].into(),
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

#[derive(Debug)]
pub struct CompressedTransactionTestInputs<'a, R: Rpc, I: Indexer> {
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
pub async fn compressed_transaction_test<
    R: Rpc + TestRpc + Indexer,
    I: Indexer + TestIndexerExtensions,
>(
    inputs: CompressedTransactionTestInputs<'_, R, I>,
) -> Result<Signature, RpcError> {
    let mut compressed_account_hashes = Vec::new();
    let compressed_account_input_hashes = if !inputs.input_compressed_accounts.is_empty() {
        for compressed_account in inputs.input_compressed_accounts.iter() {
            compressed_account_hashes.push(compressed_account.hash().unwrap());
        }
        Some(compressed_account_hashes.to_vec())
    } else {
        None
    };
    let state_input_merkle_trees = inputs
        .input_compressed_accounts
        .iter()
        .map(|x| x.merkle_context.merkle_tree_pubkey.into())
        .collect::<Vec<Pubkey>>();
    let state_input_merkle_trees = if state_input_merkle_trees.is_empty() {
        None
    } else {
        Some(state_input_merkle_trees)
    };
    let mut root_indices = Vec::new();
    let mut input_merkle_tree_snapshots = Vec::new();
    let mut address_params = Vec::new();
    let mut proof = None;
    if !inputs.input_compressed_accounts.is_empty() || !inputs.new_address_params.is_empty() {
        let address_with_trees = inputs
            .new_address_params
            .iter()
            .enumerate()
            .map(|(i, x)| AddressWithTree {
                address: inputs.created_addresses.as_ref().unwrap()[i],
                tree: x.address_merkle_tree_pubkey.into(),
            })
            .collect::<Vec<_>>();
        let proof_rpc_res = inputs
            .test_indexer
            .get_validity_proof(
                compressed_account_input_hashes.unwrap_or_else(Vec::new),
                address_with_trees,
                None,
            )
            .await
            .unwrap();
        root_indices = proof_rpc_res
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>();

        if let Some(proof_rpc_res) = proof_rpc_res.value.proof.0 {
            proof = Some(proof_rpc_res);
        }

        let input_merkle_tree_accounts = inputs
            .test_indexer
            .get_state_merkle_tree_accounts(state_input_merkle_trees.unwrap_or(vec![]).as_slice());
        input_merkle_tree_snapshots =
            get_merkle_tree_snapshots::<R>(inputs.rpc, input_merkle_tree_accounts.as_slice()).await;

        if !inputs.new_address_params.is_empty() {
            for (i, input_address_params) in inputs.new_address_params.iter().enumerate() {
                address_params.push(input_address_params.clone());
                address_params[i].address_merkle_tree_root_index =
                    proof_rpc_res.value.addresses[i].root_index;
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
    let (event, signature, slot) = TestRpc::create_and_send_transaction_with_public_event(
        inputs.rpc,
        &[instruction],
        &inputs.fee_payer.pubkey(),
        &[inputs.fee_payer, inputs.authority],
        inputs.transaction_params,
    )
    .await?
    .unwrap();

    let (created_output_compressed_accounts, _) = inputs
        .test_indexer
        .add_event_and_compressed_accounts(slot, &event.clone());
    let input = AssertCompressedTransactionInputs {
        rpc: inputs.rpc,
        test_indexer: inputs.test_indexer,
        output_compressed_accounts: inputs.output_compressed_accounts,
        created_output_compressed_accounts: created_output_compressed_accounts.as_slice(),
        event: &event,
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
        input_compressed_account_hashes: compressed_account_hashes.as_slice(),
        address_queue_pubkeys: &inputs
            .new_address_params
            .iter()
            .map(|x| x.address_queue_pubkey.into())
            .collect::<Vec<Pubkey>>(),
    };
    assert_compressed_transaction(input).await;
    Ok(signature)
}

pub fn get_sol_pool_pda() -> Pubkey {
    Pubkey::find_program_address(&[SOL_POOL_PDA_SEED], &light_system_program::ID).0
}

// TODO: move to light-test-utils
#[allow(clippy::too_many_arguments)]
pub fn create_invoke_instruction(
    fee_payer: &Pubkey,
    payer: &Pubkey,
    input_compressed_accounts: &[CompressedAccount],
    output_compressed_accounts: &[CompressedAccount],
    merkle_context: &[MerkleContext],
    output_compressed_account_merkle_tree_pubkeys: &[Pubkey],
    input_root_indices: &[Option<u16>],
    new_address_params: &[NewAddressParams],
    proof: Option<CompressedProof>,
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
    decompression_recipient: Option<Pubkey>,
    sort: bool,
) -> Instruction {
    let (remaining_accounts, mut inputs_struct) =
        create_invoke_instruction_data_and_remaining_accounts(
            new_address_params,
            merkle_context,
            input_compressed_accounts,
            input_root_indices,
            output_compressed_account_merkle_tree_pubkeys,
            output_compressed_accounts,
            proof,
            compress_or_decompress_lamports,
            is_compress,
        );
    if sort {
        inputs_struct
            .output_compressed_accounts
            .sort_by(|a, b| a.merkle_tree_index.cmp(&b.merkle_tree_index));
    }
    let mut inputs = Vec::new();

    InstructionDataInvoke::serialize(&inputs_struct, &mut inputs).unwrap();

    let instruction_data = light_system_program::instruction::Invoke { inputs };

    let sol_pool_pda = compress_or_decompress_lamports.map(|_| get_sol_pool_pda());

    let accounts = light_system_program::accounts::InvokeInstruction {
        fee_payer: *fee_payer,
        authority: *payer,
        registered_program_pda: get_registered_program_pda(&light_system_program::ID),
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        account_compression_program: account_compression::ID,
        account_compression_authority: get_cpi_authority_pda(&light_system_program::ID),
        sol_pool_pda,
        decompression_recipient,
        system_program: solana_sdk::system_program::ID,
    };
    Instruction {
        program_id: light_system_program::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_invoke_instruction_data_and_remaining_accounts(
    new_address_params: &[NewAddressParams],
    merkle_context: &[MerkleContext],
    input_compressed_accounts: &[CompressedAccount],
    input_root_indices: &[Option<u16>],
    output_compressed_account_merkle_tree_pubkeys: &[Pubkey],
    output_compressed_accounts: &[CompressedAccount],
    proof: Option<CompressedProof>,
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
) -> (Vec<AccountMeta>, InstructionDataInvoke) {
    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
    let mut _input_compressed_accounts: Vec<PackedCompressedAccountWithMerkleContext> =
        Vec::<PackedCompressedAccountWithMerkleContext>::new();
    let mut index = 0;
    let mut new_address_params_packed = new_address_params
        .iter()
        .map(|x| NewAddressParamsPacked {
            seed: x.seed,
            address_merkle_tree_root_index: x.address_merkle_tree_root_index,
            address_merkle_tree_account_index: 0, // will be assigned later
            address_queue_account_index: 0,       // will be assigned later
        })
        .collect::<Vec<NewAddressParamsPacked>>();
    for (i, context) in merkle_context.iter().enumerate() {
        match remaining_accounts.get(&context.merkle_tree_pubkey.into()) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(context.merkle_tree_pubkey.into(), index);
                index += 1;
            }
        };
        let root_index = if input_root_indices.len() > i {
            input_root_indices[i]
        } else {
            None
        };
        let prove_by_index = root_index.is_none();
        _input_compressed_accounts.push(PackedCompressedAccountWithMerkleContext {
            compressed_account: input_compressed_accounts[i].clone(),
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: *remaining_accounts
                    .get(&context.merkle_tree_pubkey.into())
                    .unwrap() as u8,
                queue_pubkey_index: 0,
                leaf_index: context.leaf_index,
                prove_by_index,
            },
            read_only: false,
            root_index: root_index.unwrap_or_default(),
        });
    }

    for (i, context) in merkle_context.iter().enumerate() {
        match remaining_accounts.get(&context.queue_pubkey.into()) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(context.queue_pubkey.into(), index);
                index += 1;
            }
        };
        _input_compressed_accounts[i]
            .merkle_context
            .queue_pubkey_index = *remaining_accounts
            .get(&context.queue_pubkey.into())
            .unwrap() as u8;
    }

    let mut output_compressed_accounts_with_context: Vec<OutputCompressedAccountWithPackedContext> =
        Vec::<OutputCompressedAccountWithPackedContext>::new();

    for (i, mt) in output_compressed_account_merkle_tree_pubkeys
        .iter()
        .enumerate()
    {
        match remaining_accounts.get(mt) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(*mt, index);
                index += 1;
            }
        };

        output_compressed_accounts_with_context.push(OutputCompressedAccountWithPackedContext {
            compressed_account: output_compressed_accounts[i].clone(),
            merkle_tree_index: *remaining_accounts.get(mt).unwrap() as u8,
        });
    }

    for (i, params) in new_address_params.iter().enumerate() {
        match remaining_accounts.get(&params.address_merkle_tree_pubkey.into()) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.address_merkle_tree_pubkey.into(), index);
                index += 1;
            }
        };
        new_address_params_packed[i].address_merkle_tree_account_index = *remaining_accounts
            .get(&params.address_merkle_tree_pubkey.into())
            .unwrap()
            as u8;
    }

    for (i, params) in new_address_params.iter().enumerate() {
        match remaining_accounts.get(&params.address_queue_pubkey.into()) {
            Some(_) => {}
            None => {
                remaining_accounts.insert(params.address_queue_pubkey.into(), index);
                index += 1;
            }
        };
        new_address_params_packed[i].address_queue_account_index = *remaining_accounts
            .get(&params.address_queue_pubkey.into())
            .unwrap() as u8;
    }
    // let mut remaining_accounts = remaining_accounts
    //     .iter()
    //     .map(|(k, i)| (AccountMeta::new(*k, false), *i))
    //     .collect::<Vec<(AccountMeta, usize)>>();
    let mut remaining_accounts = remaining_accounts
        .iter()
        .map(|(k, i)| (AccountMeta::new(*k, false), *i))
        .collect::<Vec<(AccountMeta, usize)>>();
    // hash maps are not sorted so we need to sort manually and collect into a vector again
    remaining_accounts.sort_by_key(|(_, idx)| *idx);
    let remaining_accounts = remaining_accounts
        .iter()
        .map(|(k, _)| k.clone())
        .collect::<Vec<AccountMeta>>();

    let inputs_struct = InstructionDataInvoke {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: _input_compressed_accounts,
        output_compressed_accounts: output_compressed_accounts_with_context,
        proof,
        new_address_params: new_address_params_packed,
        compress_or_decompress_lamports,
        is_compress,
    };
    (remaining_accounts, inputs_struct)
}

#[cfg(test)]
mod test {
    use anchor_lang::AnchorDeserialize;
    use solana_sdk::{signature::Keypair, signer::Signer};

    use super::*;

    #[test]
    fn test_create_execute_compressed_transaction() {
        let payer = Keypair::new().pubkey();
        let recipient = Keypair::new().pubkey();
        let input_compressed_accounts = vec![
            CompressedAccount {
                lamports: 100,
                owner: payer.into(),
                address: None,
                data: None,
            },
            CompressedAccount {
                lamports: 100,
                owner: payer.into(),
                address: None,
                data: None,
            },
        ];
        let output_compressed_accounts = vec![
            CompressedAccount {
                lamports: 50,
                owner: payer.into(),
                address: None,
                data: None,
            },
            CompressedAccount {
                lamports: 150,
                owner: recipient.into(),
                address: None,
                data: None,
            },
        ];
        let merkle_tree_indices = [0, 2];
        let merkle_tree_pubkey = Keypair::new().pubkey();
        let merkle_tree_pubkey_1 = Keypair::new().pubkey();

        let nullifier_array_pubkey = Keypair::new().pubkey();
        let input_merkle_context = vec![
            MerkleContext {
                merkle_tree_pubkey: merkle_tree_pubkey.into(),
                queue_pubkey: nullifier_array_pubkey.into(),
                leaf_index: 0,
                prove_by_index: false,
                tree_type: light_compressed_account::TreeType::StateV1,
            },
            MerkleContext {
                merkle_tree_pubkey: merkle_tree_pubkey.into(),
                queue_pubkey: nullifier_array_pubkey.into(),
                leaf_index: 1,
                prove_by_index: false,
                tree_type: light_compressed_account::TreeType::StateV1,
            },
        ];

        let output_compressed_account_merkle_tree_pubkeys =
            vec![merkle_tree_pubkey, merkle_tree_pubkey_1];
        let input_root_indices = vec![Some(0), Some(1)];
        let proof = CompressedProof {
            a: [0u8; 32],
            b: [1u8; 64],
            c: [0u8; 32],
        };
        let instruction = create_invoke_instruction(
            &payer,
            &payer,
            &input_compressed_accounts.clone(),
            &output_compressed_accounts.clone(),
            &input_merkle_context,
            &output_compressed_account_merkle_tree_pubkeys,
            &input_root_indices.clone(),
            Vec::<NewAddressParams>::new().as_slice(),
            Some(proof),
            Some(100),
            true,
            None,
            true,
        );
        assert_eq!(instruction.program_id, light_system_program::ID);

        let deserialized_instruction_data: InstructionDataInvoke =
            InstructionDataInvoke::deserialize(&mut instruction.data[12..].as_ref()).unwrap();
        deserialized_instruction_data
            .input_compressed_accounts_with_merkle_context
            .iter()
            .enumerate()
            .for_each(|(i, compressed_account_with_context)| {
                assert_eq!(
                    input_compressed_accounts[i],
                    compressed_account_with_context.compressed_account
                );
            });
        deserialized_instruction_data
            .output_compressed_accounts
            .iter()
            .enumerate()
            .for_each(|(i, compressed_account)| {
                assert_eq!(
                    OutputCompressedAccountWithPackedContext {
                        compressed_account: output_compressed_accounts[i].clone(),
                        merkle_tree_index: merkle_tree_indices[i] as u8
                    },
                    *compressed_account
                );
            });
        assert_eq!(
            deserialized_instruction_data
                .input_compressed_accounts_with_merkle_context
                .len(),
            2
        );
        assert_eq!(
            deserialized_instruction_data
                .output_compressed_accounts
                .len(),
            2
        );
        assert_eq!(deserialized_instruction_data.proof.unwrap().a, proof.a);
        assert_eq!(deserialized_instruction_data.proof.unwrap().b, proof.b);
        assert_eq!(deserialized_instruction_data.proof.unwrap().c, proof.c);
        assert_eq!(
            deserialized_instruction_data
                .compress_or_decompress_lamports
                .unwrap(),
            100
        );
        assert!(deserialized_instruction_data.is_compress);
        let ref_account_meta = AccountMeta::new(payer, true);
        assert_eq!(instruction.accounts[0], ref_account_meta);
        assert_eq!(
            deserialized_instruction_data.input_compressed_accounts_with_merkle_context[0]
                .merkle_context
                .queue_pubkey_index,
            1
        );
        assert_eq!(
            deserialized_instruction_data.input_compressed_accounts_with_merkle_context[1]
                .merkle_context
                .queue_pubkey_index,
            1
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data
                .input_compressed_accounts_with_merkle_context[0]
                .merkle_context
                .merkle_tree_pubkey_index as usize],
            AccountMeta::new(merkle_tree_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data
                .input_compressed_accounts_with_merkle_context[1]
                .merkle_context
                .merkle_tree_pubkey_index as usize],
            AccountMeta::new(merkle_tree_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data
                .input_compressed_accounts_with_merkle_context[0]
                .merkle_context
                .queue_pubkey_index as usize],
            AccountMeta::new(nullifier_array_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data
                .input_compressed_accounts_with_merkle_context[1]
                .merkle_context
                .queue_pubkey_index as usize],
            AccountMeta::new(nullifier_array_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data.output_compressed_accounts[0]
                .merkle_tree_index as usize],
            AccountMeta::new(merkle_tree_pubkey, false)
        );
        assert_eq!(
            instruction.accounts[9 + deserialized_instruction_data.output_compressed_accounts[1]
                .merkle_tree_index as usize],
            AccountMeta::new(merkle_tree_pubkey_1, false)
        );
    }
}
