#![cfg(feature = "test-sbf")]

use account_compression::errors::AccountCompressionErrorCode;
use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
use light_batched_merkle_tree::{errors::BatchedMerkleTreeError, queue::BatchedQueueAccount};
use light_client::indexer::{AddressWithTree, Indexer};
use light_compressed_account::{
    address::{derive_address, derive_address_legacy},
    compressed_account::{
        CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext, MerkleContext,
    },
    hash_to_bn254_field_size_be,
    instruction_data::{
        compressed_proof::CompressedProof,
        data::{InstructionDataInvoke, NewAddressParams},
    },
    TreeType,
};
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_program_test::{
    accounts::test_accounts::TestAccounts,
    indexer::{TestIndexer, TestIndexerExtensions},
    program_test::{LightProgramTest, TestRpc},
    utils::assert::assert_rpc_error,
    ProgramTestConfig,
};
use light_registry::protocol_config::state::ProtocolConfig;
use light_system_program::{
    errors::SystemProgramError,
    utils::{get_cpi_authority_pda, get_registered_program_pda},
};
use light_test_utils::{
    airdrop_lamports,
    assert_compressed_tx::assert_created_compressed_accounts,
    assert_custom_error_or_program_error,
    system_program::{
        compress_sol_test, create_addresses_test, create_invoke_instruction,
        create_invoke_instruction_data_and_remaining_accounts, decompress_sol_test,
        transfer_compressed_sol_test,
    },
    test_batch_forester::perform_batch_append,
    test_keypairs::for_regenerate_accounts,
    FeeConfig, Rpc, RpcError, TransactionParams,
};
use quote::format_ident;
use serial_test::serial;
use solana_cli_output::CliAccount;
use solana_sdk::{
    instruction::{AccountMeta, Instruction, InstructionError},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::{Transaction, TransactionError},
};
use tokio::fs::write as async_write;
// TODO: use lazy_static to spawn the server once

/// invoke_failing_test
/// - inputs, outputs, new addresses, (fail with every possible input)
/// Test(outputs):
/// 1. invalid lamports (ComputeOutputSumFailed)
/// 2.1 all accounts have data but signer is not a program (InvokingProgramNotProvided)
/// 2.2 one of multiple accounts has data but signer is not a program (InvokingProgramNotProvided)
/// 3. invalid output Merkle tree (AccountDiscriminatorMismatch)
/// 4. address (InvalidAddress)
/// Test(address):
/// 1. inconsistent address seed (ProofVerificationFailed)
/// 2. invalid proof (ProofVerificationFailed)
/// 3. invalid root index (ProofVerificationFailed)
/// 4.1 invalid address queue account (InvalidQueueType)
/// 4.2 invalid address queue account (AccountDiscriminatorMismatch)
/// 5. invalid address Merkle tree account (AccountDiscriminatorMismatch)
/// Test(inputs):
/// 1. invalid proof (ProofVerificationFailed)
/// 2. invalid root index (ProofVerificationFailed)
/// 3. invalid leaf index (ProofVerificationFailed)
/// 4.1 invalid account data lamports (ProofVerificationFailed)
/// 4.2 invalid account data address (ProofVerificationFailed)
/// 4.3 invalid account data owner (SignerCheckFailed)
/// - invalid data is not tested because a compressed account that is not program-owned cannot have data
/// 5. invalid Merkle tree account (AccountDiscriminatorMismatch)
/// 6.1 invalid queue account (InvalidQueueType)
/// 6.2 invalid queue account (AccountDiscriminatorMismatch)
#[serial]
#[tokio::test]
async fn invoke_failing_test() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    // no inputs
    let (remaining_accounts, inputs_struct) = create_invoke_instruction_data_and_remaining_accounts(
        &Vec::new(),
        &Vec::new(),
        &Vec::new(),
        &Vec::new(),
        &Vec::new(),
        &Vec::new(),
        None,
        None,
        false,
    );
    create_instruction_and_failing_transaction(
        &mut rpc,
        &payer,
        inputs_struct,
        remaining_accounts,
        SystemProgramError::EmptyInputs.into(),
    )
    .await
    .unwrap();

    // circuit instantiations allow for 1, 2, 3, 4, 8 inclusion proofs
    let options = [0usize, 1usize, 2usize, 3usize, 4usize, 8usize];

    for mut num_addresses in 0..=2 {
        for (j, option) in options.iter().enumerate() {
            // there is no combined circuit instantiation for 8 inputs and addresses
            if j == 5 {
                num_addresses = 0;
            }
            for num_outputs in 1..8 {
                println!(
                    "failing_transaction_inputs num_addresses: {}, num_outputs: {}, option: {}",
                    num_addresses, num_outputs, option
                );
                failing_transaction_inputs(
                    &mut rpc,
                    &payer,
                    *option,
                    1_000_000,
                    num_addresses,
                    num_outputs,
                    false,
                )
                .await
                .unwrap();
            }
        }
    }
    for mut num_addresses in 0..=2 {
        for (j, option) in options.iter().enumerate() {
            // there is no combined circuit instantiation for 8 inputs and addresses
            if j == 5 {
                num_addresses = 0;
            }
            for num_outputs in 0..8 {
                println!(
                    "failing_transaction_inputs2 num_addresses: {}, num_outputs: {}, option: {}",
                    num_addresses, num_outputs, options[j]
                );
                failing_transaction_inputs(
                    &mut rpc,
                    &payer,
                    *option,
                    0,
                    num_addresses,
                    num_outputs,
                    false,
                )
                .await
                .unwrap();
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn failing_transaction_inputs(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    num_inputs: usize,
    amount: u64,
    num_addresses: usize,
    num_outputs: usize,
    output_compressed_accounts_with_address: bool,
) -> Result<(), RpcError> {
    let env = rpc.test_accounts.clone();
    // create compressed accounts that can be used as inputs
    for _ in 0..num_inputs {
        let mut test_indexer = (*rpc.indexer().unwrap()).clone();
        let output_merkle_tree = env.v1_state_trees[0].merkle_tree;
        compress_sol_test(
            rpc,
            &mut test_indexer,
            payer,
            &[],
            false,
            amount,
            &output_merkle_tree,
            None,
        )
        .await
        .unwrap();
        *rpc.indexer_mut()? = test_indexer;
    }
    let (mut new_address_params, derived_addresses) =
        create_address_test_inputs(&env, num_addresses);
    let input_compressed_accounts = rpc
        .get_compressed_accounts_with_merkle_context_by_owner(&payer.pubkey())[0..num_inputs]
        .to_vec();
    let hashes = input_compressed_accounts
        .iter()
        .map(|x| x.hash().unwrap())
        .collect::<Vec<_>>();

    let proof_input_derived_addresses = if num_addresses != 0 {
        Some(derived_addresses.as_slice())
    } else {
        None
    };

    let addresses_with_tree = proof_input_derived_addresses
        .unwrap_or_default()
        .iter()
        .map(|address| AddressWithTree {
            address: *address,
            tree: rpc.test_accounts.v1_address_trees[0].merkle_tree,
        })
        .collect::<Vec<_>>();

    let (root_indices, proof) = if !addresses_with_tree.is_empty() || !hashes.is_empty() {
        // || proof_input_derived_addresses.is_some()
        let proof_rpc_res = rpc
            .get_validity_proof(hashes, addresses_with_tree, None)
            .await
            .unwrap();
        for (i, root_index) in proof_rpc_res.value.addresses.iter().enumerate() {
            new_address_params[i].address_merkle_tree_root_index = root_index.root_index;
        }
        let root_indices = proof_rpc_res
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>();
        (root_indices, proof_rpc_res.value.proof.0)
    } else {
        (Vec::new(), None)
    };
    let (output_compressed_accounts, output_merkle_tree_pubkeys) = if num_outputs > 0 {
        let mut output_compressed_accounts = vec![];
        let mut output_merkle_tree_pubkeys = vec![];
        let sum_lamports = input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.lamports)
            .sum::<u64>();
        let output_amount = sum_lamports / num_outputs as u64;
        let remainder = sum_lamports % num_outputs as u64;
        #[allow(clippy::needless_range_loop)]
        for i in 0..num_outputs {
            let address = if output_compressed_accounts_with_address && i < num_addresses {
                Some(derived_addresses[i])
            } else {
                None
            };
            output_compressed_accounts.push(CompressedAccount {
                lamports: output_amount,
                owner: payer.pubkey().into(),
                data: None,
                address,
            });
            output_merkle_tree_pubkeys.push(env.v1_state_trees[0].merkle_tree);
        }
        output_compressed_accounts[0].lamports += remainder;
        (output_compressed_accounts, output_merkle_tree_pubkeys)
    } else {
        (Vec::new(), Vec::new())
    };
    let (remaining_accounts, inputs_struct) = create_invoke_instruction_data_and_remaining_accounts(
        &new_address_params,
        &input_compressed_accounts
            .iter()
            .map(|x| x.merkle_context)
            .collect::<Vec<_>>(),
        &input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>(),
        &root_indices,
        &output_merkle_tree_pubkeys,
        &output_compressed_accounts,
        proof,
        None,
        false,
    );
    println!("num_addresses {:?}", num_addresses);
    if num_addresses > 0 {
        failing_transaction_address(rpc, payer, &env, &inputs_struct, remaining_accounts.clone())
            .await?;
    }
    println!("num_inputs {:?}", num_inputs);
    if num_inputs > 0 {
        failing_transaction_inputs_inner(
            rpc,
            payer,
            &env,
            &inputs_struct,
            remaining_accounts.clone(),
        )
        .await?;
    }
    if num_outputs > 0 {
        failing_transaction_output(rpc, payer, &env, inputs_struct, remaining_accounts.clone())
            .await?;
    }
    Ok(())
}

pub async fn failing_transaction_inputs_inner<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    env: &TestAccounts,
    inputs_struct: &InstructionDataInvoke,
    remaining_accounts: Vec<AccountMeta>,
) -> Result<(), RpcError> {
    let num_inputs = inputs_struct
        .input_compressed_accounts_with_merkle_context
        .len();
    let num_outputs = inputs_struct.output_compressed_accounts.len();
    // invalid proof
    {
        println!("invalid proof");
        let mut inputs_struct = inputs_struct.clone();
        println!("inputs_struct {:?}", inputs_struct);
        inputs_struct.proof.as_mut().unwrap().a = inputs_struct.proof.as_ref().unwrap().c;
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::ProofVerificationFailed.into(),
        )
        .await
        .unwrap();
    }
    // invalid root index
    {
        println!("invalid root index");
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1].root_index = 0;
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::ProofVerificationFailed.into(),
        )
        .await
        .unwrap();
    }
    // // invalid leaf index
    // {
    //     println!(
    //         "leaf index: {}",
    //         inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
    //             .merkle_context
    //             .leaf_index
    //     );
    //     let mut inputs_struct = inputs_struct.clone();
    //     inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
    //         .merkle_context
    //         .leaf_index += 1;
    //     create_instruction_and_failing_transaction(
    //         rpc,
    //         payer,
    //         inputs_struct,
    //         remaining_accounts.clone(),
    //         SystemProgramError::ProofVerificationFailed.into(),
    //     )
    //     .await
    //     .unwrap();
    // }
    // invalid account data (lamports)
    if !inputs_struct.output_compressed_accounts.is_empty() {
        let mut inputs_struct = inputs_struct.clone();
        let amount = inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
            .compressed_account
            .lamports;
        inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
            .compressed_account
            .lamports = amount + 1;
        let error_code = if !inputs_struct.output_compressed_accounts.is_empty() {
            // adapting compressed output account so that sumcheck passes
            inputs_struct.output_compressed_accounts[0]
                .compressed_account
                .lamports += 1;
            SystemProgramError::ProofVerificationFailed.into()
        } else {
            SystemProgramError::SumCheckFailed.into()
        };

        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            error_code,
        )
        .await
        .unwrap();
    }
    // invalid account data (address)
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
            .compressed_account
            .address = Some(hash_to_bn254_field_size_be([1u8; 32].as_slice()));
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::ProofVerificationFailed.into(),
        )
        .await
        .unwrap();
    }
    // invalid account data (owner)
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
            .compressed_account
            .owner = Keypair::new().pubkey().into();

        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::SignerCheckFailed.into(),
        )
        .await
        .unwrap();
    }
    // invalid account data (data)
    {
        let data = CompressedAccountData {
            discriminator: [1u8; 8],
            data: vec![1u8; 1],
            data_hash: hash_to_bn254_field_size_be([1u8; 32].as_slice()),
        };
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
            .compressed_account
            .data = Some(data);
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::SignerCheckFailed.into(),
        )
        .await
        .unwrap();
    }
    // invalid Merkle tree account
    {
        let inputs_struct = inputs_struct.clone();
        let mut remaining_accounts = remaining_accounts.clone();
        remaining_accounts[inputs_struct.input_compressed_accounts_with_merkle_context
            [num_inputs - 1]
            .merkle_context
            .merkle_tree_pubkey_index as usize] = AccountMeta {
            pubkey: env.v1_address_trees[0].merkle_tree,
            is_signer: false,
            is_writable: false,
        };
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch.into(),
        )
        .await
        .unwrap();
    }
    // invalid queue account
    {
        let inputs_struct = inputs_struct.clone();
        let mut remaining_accounts = remaining_accounts.clone();
        remaining_accounts[inputs_struct.input_compressed_accounts_with_merkle_context
            [num_inputs - 1]
            .merkle_context
            .queue_pubkey_index as usize] = AccountMeta {
            pubkey: env.v1_address_trees[0].queue,
            is_signer: false,
            is_writable: true,
        };
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
        )
        .await
        .unwrap();
    }
    // invalid queue account
    {
        let inputs_struct = inputs_struct.clone();
        let mut remaining_accounts = remaining_accounts.clone();
        remaining_accounts[inputs_struct.input_compressed_accounts_with_merkle_context
            [num_inputs - 1]
            .merkle_context
            .queue_pubkey_index as usize] = AccountMeta {
            pubkey: env.v1_address_trees[0].merkle_tree,
            is_signer: false,
            is_writable: true,
        };
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            AccountCompressionErrorCode::InvalidAccount.into(),
        )
        .await
        .unwrap();
    }
    // output Merkle tree is not unique (we need at least 2 outputs for this test)
    if num_outputs > 1 {
        let mut inputs_struct = inputs_struct.clone();
        let mut remaining_accounts = remaining_accounts.clone();
        let remaining_mt_acc = remaining_accounts
            [inputs_struct.output_compressed_accounts[1].merkle_tree_index as usize]
            .clone();
        remaining_accounts.push(remaining_mt_acc);
        inputs_struct.output_compressed_accounts[1].merkle_tree_index =
            (remaining_accounts.len() - 1) as u8;
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::OutputMerkleTreeNotUnique.into(),
        )
        .await
        .unwrap();
    }
    Ok(())
}

fn create_address_test_inputs(
    env: &TestAccounts,
    num_addresses: usize,
) -> (Vec<NewAddressParams>, Vec<[u8; 32]>) {
    let mut address_seeds = vec![];
    for i in 1..=num_addresses {
        address_seeds.push([i as u8; 32]);
    }

    let mut new_address_params = vec![];
    let mut derived_addresses = Vec::new();
    for address_seed in address_seeds.iter() {
        new_address_params.push(NewAddressParams {
            seed: *address_seed,
            address_queue_pubkey: env.v1_address_trees[0].queue.into(),
            address_merkle_tree_pubkey: env.v1_address_trees[0].merkle_tree.into(),
            address_merkle_tree_root_index: 0,
        });
        let derived_address =
            derive_address_legacy(&env.v1_address_trees[0].merkle_tree.into(), address_seed)
                .unwrap();
        derived_addresses.push(derived_address);
    }
    (new_address_params, derived_addresses)
}

pub async fn failing_transaction_address<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    env: &TestAccounts,
    inputs_struct: &InstructionDataInvoke,
    remaining_accounts: Vec<AccountMeta>,
) -> Result<(), RpcError> {
    // inconsistent seed
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.new_address_params[0].seed = [100u8; 32];
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::ProofVerificationFailed.into(),
        )
        .await
        .unwrap();
    }
    // invalid proof
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.proof.as_mut().unwrap().a = inputs_struct.proof.as_ref().unwrap().c;
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::ProofVerificationFailed.into(),
        )
        .await
        .unwrap();
    }
    // invalid root index
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.new_address_params[0].address_merkle_tree_root_index = 0;
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::ProofVerificationFailed.into(),
        )
        .await
        .unwrap();
    }

    // invalid address queue account
    {
        let inputs_struct = inputs_struct.clone();
        let mut remaining_accounts = remaining_accounts.clone();
        remaining_accounts
            [inputs_struct.new_address_params[0].address_queue_account_index as usize] =
            AccountMeta {
                pubkey: env.v1_state_trees[0].nullifier_queue,
                is_signer: false,
                is_writable: false,
            };
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            MerkleTreeMetadataError::InvalidQueueType.into(),
        )
        .await
        .unwrap();
    }
    // invalid address queue account
    {
        let inputs_struct = inputs_struct.clone();
        let mut remaining_accounts = remaining_accounts.clone();
        remaining_accounts
            [inputs_struct.new_address_params[0].address_queue_account_index as usize] =
            AccountMeta {
                pubkey: env.v1_state_trees[0].merkle_tree,
                is_signer: false,
                is_writable: true,
            };
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch.into(),
        )
        .await
        .unwrap();
    }
    // invalid address Merkle tree account
    {
        let inputs_struct = inputs_struct.clone();
        let mut remaining_accounts = remaining_accounts.clone();
        remaining_accounts
            [inputs_struct.new_address_params[0].address_merkle_tree_account_index as usize] =
            AccountMeta {
                pubkey: env.v1_state_trees[0].merkle_tree,
                is_signer: false,
                is_writable: false,
            };
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            SystemProgramError::AddressMerkleTreeAccountDiscriminatorMismatch.into(),
        )
        .await
        .unwrap();
    }
    Ok(())
}
/// Output compressed accounts no inputs:
/// 1. invalid lamports (for no input compressed accounts lamports can only be 0)
/// 2. data but signer is not a program
/// 3. invalid output Merkle tree
/// 4. address that doesn't exist
pub async fn failing_transaction_output<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    env: &TestAccounts,
    inputs_struct: InstructionDataInvoke,
    remaining_accounts: Vec<AccountMeta>,
) -> Result<(), RpcError> {
    let num_output_compressed_accounts = inputs_struct.output_compressed_accounts.len();
    // invalid lamports
    {
        let mut inputs_struct = inputs_struct.clone();
        let error_code = if inputs_struct
            .input_compressed_accounts_with_merkle_context
            .iter()
            .map(|x| x.compressed_account.lamports)
            .sum::<u64>()
            == 0
        {
            SystemProgramError::ComputeOutputSumFailed.into()
        } else {
            SystemProgramError::SumCheckFailed.into()
        };
        inputs_struct.output_compressed_accounts[num_output_compressed_accounts - 1]
            .compressed_account
            .lamports = 1;
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct.clone(),
            remaining_accounts.clone(),
            error_code,
        )
        .await
        .unwrap();
    }
    // Data but signer is not a program
    {
        let mut inputs_struct = inputs_struct.clone();

        for (i, account) in inputs_struct
            .output_compressed_accounts
            .iter_mut()
            .enumerate()
        {
            let data = CompressedAccountData {
                discriminator: [i as u8; 8],
                data: vec![i as u8; i],
                data_hash: [i as u8; 32],
            };
            account.compressed_account.data = Some(data);
        }
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct.clone(),
            remaining_accounts.clone(),
            SystemProgramError::InvokingProgramNotProvided.into(),
        )
        .await
        .unwrap();
    }
    // Invalid output Merkle tree
    {
        let mut remaining_accounts = remaining_accounts.clone();
        remaining_accounts[inputs_struct.output_compressed_accounts
            [num_output_compressed_accounts - 1]
            .merkle_tree_index as usize] = AccountMeta {
            pubkey: env.v1_address_trees[0].merkle_tree,
            is_signer: false,
            is_writable: false,
        };
        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct.clone(),
            remaining_accounts.clone(),
            SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch.into(),
        )
        .await
        .unwrap();
    }

    // Address that doesn't exist
    {
        let mut inputs_struct = inputs_struct.clone();

        for account in inputs_struct.output_compressed_accounts.iter_mut() {
            let address = Some(hash_to_bn254_field_size_be(
                Keypair::new().pubkey().to_bytes().as_slice(),
            ));
            account.compressed_account.address = address;
        }

        create_instruction_and_failing_transaction(
            rpc,
            payer,
            inputs_struct.clone(),
            remaining_accounts.clone(),
            SystemProgramError::InvalidAddress.into(),
        )
        .await
        .unwrap();
    }
    Ok(())
}

pub async fn perform_tx_with_output_compressed_accounts(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    payer_pubkey: Pubkey,
    output_compressed_accounts: Vec<CompressedAccount>,
    output_merkle_tree_pubkeys: Vec<Pubkey>,
    expected_error_code: u32,
) -> Result<(), RpcError> {
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        output_merkle_tree_pubkeys.as_slice(),
        &Vec::new(),
        &Vec::new(),
        None,
        None,
        false,
        None,
        true,
    );
    let result = rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[payer])
        .await;
    assert_rpc_error(result, 0, expected_error_code)
}

pub async fn create_instruction_and_failing_transaction<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    inputs_struct: InstructionDataInvoke,
    remaining_accounts: Vec<AccountMeta>,
    expected_error_code: u32,
) -> Result<(), RpcError> {
    let mut inputs = Vec::new();

    InstructionDataInvoke::serialize(&inputs_struct, &mut inputs).unwrap();

    let instruction_data = light_system_program::instruction::Invoke { inputs };

    let sol_pool_pda = None;

    let accounts = light_system_program::accounts::InvokeInstruction {
        fee_payer: payer.pubkey(),
        authority: payer.pubkey(),
        registered_program_pda: get_registered_program_pda(&light_system_program::ID),
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        account_compression_program: account_compression::ID,
        account_compression_authority: get_cpi_authority_pda(&light_system_program::ID),
        sol_pool_pda,
        decompression_recipient: None,
        system_program: solana_sdk::system_program::ID,
    };
    let instruction = Instruction {
        program_id: light_system_program::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;
    if assert_rpc_error(result.clone(), 0, expected_error_code).is_err() {
        // In case program panics instead of returning an error code.
        // This can happen if proof verification fails and debug print runs oom.
        assert_rpc_error(result, 0, 21)
    } else {
        Ok(())
    }
}

/// Tests Execute compressed transaction:
/// 1. should succeed: without compressed account(0 lamports), no in compressed account
/// 2. should fail: in compressed account and invalid zkp
/// 3. should fail: in compressed account and invalid signer
/// 4. should succeed: in compressed account inserted in (1.) and valid zkp
#[serial]
#[tokio::test]
async fn invoke_test() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let env = rpc.test_accounts.clone();

    let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.v1_state_trees[0].merkle_tree;
    let nullifier_queue_pubkey = env.v1_state_trees[0].nullifier_queue;
    println!("merkle_tree_pubkey {:?}", merkle_tree_pubkey.to_bytes());
    println!(
        "nullifier_queue_pubkey {:?}",
        nullifier_queue_pubkey.to_bytes()
    );
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey.into(),
        data: None,
        address: None,
    }];
    let output_merkle_tree_pubkeys = vec![merkle_tree_pubkey];
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        output_merkle_tree_pubkeys.as_slice(),
        &Vec::new(),
        &Vec::new(),
        None,
        None,
        false,
        None,
        true,
    );

    let event = TestRpc::create_and_send_transaction_with_public_event(
        &mut rpc,
        &[instruction],
        &payer_pubkey,
        &[&payer],
        Some(TransactionParams {
            v1_input_compressed_accounts: 0u8,
            v2_input_compressed_accounts: false,
            num_output_compressed_accounts: 1,
            num_new_addresses: 0,
            compress: 0,
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();

    let slot: u64 = rpc.get_slot().await.unwrap();
    let (created_compressed_accounts, _) =
        test_indexer.add_event_and_compressed_accounts(slot, &event.0);

    assert_created_compressed_accounts(
        output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys.as_slice(),
        created_compressed_accounts.as_slice(),
    );

    let input_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey.into(),
        data: None,
        address: None,
    }];
    // check invalid proof
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey: merkle_tree_pubkey.into(),
            leaf_index: 0,
            queue_pubkey: nullifier_queue_pubkey.into(),
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        &[merkle_tree_pubkey],
        &[Some(0u16)],
        &Vec::new(),
        None,
        None,
        false,
        None,
        true,
    );

    let res = rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
        .await;
    assert!(res.is_err());

    // check invalid signer for in compressed_account
    let invalid_signer_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Keypair::new().pubkey().into(),
        data: None,
        address: None,
    }];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &invalid_signer_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey: merkle_tree_pubkey.into(),
            leaf_index: 0,
            queue_pubkey: nullifier_queue_pubkey.into(),
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        &[merkle_tree_pubkey],
        &[Some(0u16)],
        &Vec::new(),
        None,
        None,
        false,
        None,
        true,
    );

    let res = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(res.is_err());

    // create Merkle proof
    // get zkp from server
    // create instruction as usual with correct zkp
    let compressed_account_with_context =
        rpc.indexer.as_ref().unwrap().compressed_accounts[0].clone();
    let proof_rpc_res = rpc
        .get_validity_proof(
            vec![compressed_account_with_context.hash().unwrap()],
            vec![],
            None,
        )
        .await
        .unwrap();
    let proof = proof_rpc_res.value.proof.0.unwrap();
    let input_compressed_accounts = vec![compressed_account_with_context.compressed_account];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey: merkle_tree_pubkey.into(),
            leaf_index: 0,
            queue_pubkey: nullifier_queue_pubkey.into(),
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        &Vec::new(),
        Some(proof),
        None,
        false,
        None,
        true,
    );
    println!("Transaction with zkp -------------------------");

    TestRpc::create_and_send_transaction_with_public_event(
        &mut rpc,
        &[instruction],
        &payer_pubkey,
        &[&payer],
        Some(TransactionParams {
            v1_input_compressed_accounts: 1,
            v2_input_compressed_accounts: false,
            num_output_compressed_accounts: 1,
            num_new_addresses: 0,
            compress: 0,
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();

    println!("Double spend -------------------------");
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Keypair::new().pubkey().into(),
        data: None,
        address: None,
    }];
    // double spend
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey: merkle_tree_pubkey.into(),
            leaf_index: 0,
            queue_pubkey: nullifier_queue_pubkey.into(),
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        &Vec::new(),
        Some(proof),
        None,
        false,
        None,
        true,
    );
    let res = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(res.is_err());
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Keypair::new().pubkey().into(),
        data: None,
        address: None,
    }];
    // invalid compressed_account
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey: merkle_tree_pubkey.into(),
            leaf_index: 1,
            queue_pubkey: nullifier_queue_pubkey.into(),
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        &Vec::new(),
        Some(proof),
        None,
        false,
        None,
        true,
    );
    let res = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(res.is_err());
}

/// Tests Execute compressed transaction with address:
/// 1. should fail: create out compressed account with address without input compressed account with address or created address
/// 2. should fail: v1 address tree with v2 address derivation
/// 3. should fail: v2 address tree create address with invoke instruction (invoking program id required for derivation)
/// 2. should succeed: create out compressed account with new created address
/// 3. should fail: create two addresses with the same seeds
/// 4. should succeed: create two addresses with different seeds
/// 5. should succeed: create multiple addresses with different seeds and spend input compressed accounts
///    testing: (input accounts, new addresses) (1, 1), (1, 2), (2, 1), (2, 2)
#[serial]
#[tokio::test]
async fn test_with_address() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_with_batched_trees(false))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let env = rpc.test_accounts.clone();

    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = rpc.test_accounts.v1_state_trees[0].merkle_tree;

    let address_seed = [1u8; 32];
    let derived_address = derive_address_legacy(
        &rpc.test_accounts.v1_address_trees[0].merkle_tree.into(),
        &address_seed,
    )
    .unwrap();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey.into(),
        data: None,
        address: Some(derived_address), // this should not be sent, only derived on-chain
    }];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        None,
        None,
        false,
        None,
        true,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap().0,
    );

    let res = rpc.process_transaction(transaction).await;
    assert_custom_error_or_program_error(res, SystemProgramError::InvalidAddress.into()).unwrap();
    // v1 address tree with new derivation should fail
    {
        let derived_address = derive_address(
            &address_seed,
            &rpc.test_accounts.v2_address_trees[0].to_bytes(),
            &payer_pubkey.to_bytes(),
        );
        let output_compressed_accounts = vec![CompressedAccount {
            lamports: 0,
            owner: payer_pubkey.into(),
            data: None,
            address: Some(derived_address), // this should not be sent, only derived on-chain
        }];

        let address_params = vec![NewAddressParams {
            seed: address_seed,
            address_queue_pubkey: rpc.test_accounts.v1_address_trees[0].queue.into(),
            address_merkle_tree_pubkey: rpc.test_accounts.v1_address_trees[0].merkle_tree.into(),
            address_merkle_tree_root_index: 0,
        }];
        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &Vec::new(),
            &output_compressed_accounts,
            &Vec::new(),
            &[rpc.test_accounts.v2_state_trees[0].output_queue],
            &Vec::new(),
            address_params.as_slice(),
            None,
            None,
            false,
            None,
            true,
        );

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer_pubkey),
            &[&payer],
            rpc.get_latest_blockhash().await.unwrap().0,
        );

        let res = rpc.process_transaction(transaction).await;
        assert_custom_error_or_program_error(res, SystemProgramError::InvalidAddress.into())
            .unwrap();
    }
    // batch address tree with new derivation should fail with invoke because invoking program is not provided.
    {
        let derived_address = derive_address(
            &address_seed,
            &rpc.test_accounts.v2_address_trees[0].to_bytes(),
            &payer_pubkey.to_bytes(),
        );
        let output_compressed_accounts = vec![CompressedAccount {
            lamports: 0,
            owner: payer_pubkey.into(),
            data: None,
            address: Some(derived_address), // this should not be sent, only derived on-chain
        }];
        let address_params = vec![NewAddressParams {
            seed: address_seed,
            address_queue_pubkey: rpc.test_accounts.v2_address_trees[0].into(),
            address_merkle_tree_pubkey: rpc.test_accounts.v2_address_trees[0].into(),
            address_merkle_tree_root_index: 0,
        }];

        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &Vec::new(),
            &output_compressed_accounts,
            &Vec::new(),
            &[env.v2_state_trees[0].output_queue],
            &Vec::new(),
            address_params.as_slice(),
            None,
            None,
            false,
            None,
            true,
        );

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer_pubkey),
            &[&payer],
            rpc.get_latest_blockhash().await.unwrap().0,
        );

        let res = rpc.process_transaction(transaction).await;
        assert_custom_error_or_program_error(res, SystemProgramError::DeriveAddressError.into())
            .unwrap();
    }
    let env = rpc.test_accounts.clone();
    println!("creating address -------------------------");
    let mut indexer = rpc.clone_indexer().unwrap();
    create_addresses_test(
        &mut rpc,
        &mut indexer,
        &[env.v1_address_trees[0].merkle_tree],
        &[env.v1_address_trees[0].queue],
        vec![env.v1_state_trees[0].merkle_tree],
        &[address_seed],
        &Vec::new(),
        false,
        None,
    )
    .await
    .unwrap();
    (*rpc.indexer_mut().unwrap()) = indexer;
    // transfer with address
    println!("transfer with address-------------------------");

    let compressed_account_with_context = rpc.indexer().unwrap().compressed_accounts[0].clone();
    let recipient_pubkey = Keypair::new().pubkey();
    let mut indexer = rpc.clone_indexer().unwrap();
    transfer_compressed_sol_test(
        &mut rpc,
        &mut indexer,
        &payer,
        std::slice::from_ref(&compressed_account_with_context),
        &[recipient_pubkey],
        &[compressed_account_with_context
            .merkle_context
            .merkle_tree_pubkey
            .into()],
        None,
    )
    .await
    .unwrap();

    assert_eq!(indexer.compressed_accounts.len(), 1);
    assert_eq!(
        indexer.compressed_accounts[0]
            .compressed_account
            .address
            .unwrap(),
        derived_address
    );
    assert_eq!(
        indexer.compressed_accounts[0]
            .compressed_account
            .owner
            .to_bytes(),
        recipient_pubkey.to_bytes()
    );
    (*rpc.indexer_mut().unwrap()) = indexer;

    let address_seed_2 = [2u8; 32];
    let mut indexer = rpc.clone_indexer().unwrap();
    let event = create_addresses_test(
        &mut rpc,
        &mut indexer,
        &[
            env.v1_address_trees[0].merkle_tree,
            env.v1_address_trees[0].merkle_tree,
        ],
        &[env.v1_address_trees[0].queue, env.v1_address_trees[0].queue],
        vec![
            env.v1_state_trees[0].merkle_tree,
            env.v1_state_trees[0].merkle_tree,
        ],
        &[address_seed_2, address_seed_2],
        &Vec::new(),
        false,
        None,
    )
    .await;
    // Should fail to insert the same address twice in the same tx
    assert!(matches!(
        event,
        Err(RpcError::TransactionError(
            // ElementAlreadyExists
            TransactionError::InstructionError(0, InstructionError::Custom(9002))
        ))
    ));

    println!("test 2in -------------------------");

    let address_seed_3 = [3u8; 32];
    let mut indexer = rpc.clone_indexer().unwrap();
    create_addresses_test(
        &mut rpc,
        &mut indexer,
        &[
            env.v1_address_trees[0].merkle_tree,
            env.v1_address_trees[0].merkle_tree,
        ],
        &[env.v1_address_trees[0].queue, env.v1_address_trees[0].queue],
        vec![
            env.v1_state_trees[0].merkle_tree,
            env.v1_state_trees[0].merkle_tree,
        ],
        &[address_seed_2, address_seed_3],
        &Vec::new(),
        false,
        None,
    )
    .await
    .unwrap();
    (*rpc.indexer_mut().unwrap()) = indexer;

    // Test combination
    // (num_input_compressed_accounts, num_new_addresses)
    let test_inputs = vec![
        (1, 1),
        (1, 2),
        (2, 1),
        (2, 2),
        (3, 1),
        (3, 2),
        (4, 1),
        (4, 2),
    ];
    for (n_input_compressed_accounts, n_new_addresses) in test_inputs {
        let compressed_input_accounts = rpc
            .get_compressed_accounts_with_merkle_context_by_owner(&payer_pubkey)
            [0..n_input_compressed_accounts]
            .to_vec();

        let mut address_vec = Vec::new();
        // creates multiple seeds by taking the number of input accounts and zeroing out the jth byte
        for j in 0..n_new_addresses {
            let mut address_seed = [n_input_compressed_accounts as u8; 32];
            address_seed[j + (n_new_addresses * 2)] = 0_u8;
            address_vec.push(address_seed);
        }

        let mut indexer = rpc.clone_indexer().unwrap();
        create_addresses_test(
            &mut rpc,
            &mut indexer,
            &vec![env.v1_address_trees[0].merkle_tree; n_new_addresses],
            &vec![env.v1_address_trees[0].queue; n_new_addresses],
            vec![env.v1_state_trees[0].merkle_tree; n_new_addresses],
            &address_vec,
            &compressed_input_accounts,
            true,
            None,
        )
        .await
        .unwrap();
        (*rpc.indexer_mut().unwrap()) = indexer;
    }
}

#[serial]
#[tokio::test]
async fn test_with_compression() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let payer_pubkey = payer.pubkey();
    let env = rpc.test_accounts.clone();
    let merkle_tree_pubkey = env.v1_state_trees[0].merkle_tree;
    let nullifier_queue_pubkey = env.v1_state_trees[0].nullifier_queue;

    let compress_amount = 1_000_000;
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: compress_amount + 1,
        owner: payer_pubkey.into(),
        data: None,
        address: None,
    }];
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        None,
        Some(compress_amount),
        false,
        None,
        true,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap().0,
    );

    let result = rpc.process_transaction(transaction).await;
    // should fail because of insufficient input funds
    assert_custom_error_or_program_error(result, SystemProgramError::ComputeOutputSumFailed.into())
        .unwrap();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: compress_amount,
        owner: payer_pubkey.into(),
        data: None,
        address: None,
    }];
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        &[merkle_tree_pubkey],
        &Vec::new(),
        &Vec::new(),
        None,
        None,
        true,
        None,
        true,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap().0,
    );

    let result = rpc.process_transaction(transaction).await;
    // should fail because of insufficient decompress amount funds
    assert_custom_error_or_program_error(result, SystemProgramError::ComputeOutputSumFailed.into())
        .unwrap();

    let mut indexer = rpc.clone_indexer().unwrap();
    compress_sol_test(
        &mut rpc,
        &mut indexer,
        &payer,
        &Vec::new(),
        false,
        compress_amount,
        &env.v1_state_trees[0].merkle_tree,
        None,
    )
    .await
    .unwrap();
    rpc.indexer = Some(indexer);

    let compressed_account_with_context = rpc
        .indexer
        .as_ref()
        .unwrap()
        .compressed_accounts
        .last()
        .unwrap()
        .clone();
    let proof_rpc_res = rpc
        .get_validity_proof(
            vec![compressed_account_with_context.hash().unwrap()],
            vec![],
            None,
        )
        .await
        .unwrap();
    let proof = proof_rpc_res.value.proof.0.unwrap();
    let input_compressed_accounts =
        vec![compressed_account_with_context.clone().compressed_account];
    let recipient_pubkey = Keypair::new().pubkey();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: recipient_pubkey.into(),
        data: None,
        address: None,
    }];
    let recipient = Keypair::new().pubkey();
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey: merkle_tree_pubkey.into(),
            leaf_index: 0,
            queue_pubkey: nullifier_queue_pubkey.into(),
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        &Vec::new(),
        Some(proof),
        Some(compress_amount),
        true,
        Some(recipient),
        true,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap().0,
    );
    println!("Transaction with zkp -------------------------");

    let result = rpc.process_transaction(transaction).await;
    // should fail because of insufficient output funds
    assert_custom_error_or_program_error(result, SystemProgramError::SumCheckFailed.into())
        .unwrap();

    let compressed_account_with_context =
        rpc.get_compressed_accounts_with_merkle_context_by_owner(&payer_pubkey)[0].clone();

    let mut test_indexer = (*rpc.indexer().unwrap()).clone();
    decompress_sol_test(
        &mut rpc,
        &mut test_indexer,
        &payer,
        &vec![compressed_account_with_context],
        &recipient_pubkey,
        compress_amount,
        &env.v1_state_trees[0].merkle_tree,
        None,
    )
    .await
    .unwrap();
    *(rpc.indexer_mut().unwrap()) = test_indexer;
}

#[ignore = "this is a helper function to regenerate accounts"]
// #[serial]
#[tokio::test]
async fn regenerate_accounts() {
    let output_dir = "../../cli/accounts/";

    let protocol_config = ProtocolConfig {
        genesis_slot: 0,
        slot_length: 10,
        registration_phase_length: 100,
        active_phase_length: 200,
        report_work_phase_length: 100,
        ..ProtocolConfig::default()
    };
    let mut config = ProgramTestConfig::default_with_batched_trees(false);
    config.protocol_config = protocol_config;
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let env = rpc.test_accounts.clone();
    let keypairs = for_regenerate_accounts();

    airdrop_lamports(
        &mut rpc,
        &keypairs.governance_authority.pubkey(),
        100_000_000_000,
    )
    .await
    .unwrap();

    airdrop_lamports(&mut rpc, &keypairs.forester.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // // // Setup forester and get epoch information
    // let forester_epoch = setup_forester_and_advance_to_epoch(&mut rpc, &protocol_config)
    //     .await
    //     .unwrap();
    // let forester_epoch_pda = get_forester_epoch_pda(&env.protocol.forester.pubkey(), 0).0;
    // let epoch_pda = get_epoch_pda_address(0);
    // List of public keys to fetch and export - dynamically built from test accounts
    let mut pubkeys = vec![
        (
            "governance_authority_pda",
            env.protocol.governance_authority_pda,
        ),
        ("group_pda", env.protocol.group_pda),
        (
            "registered_program_pda",
            env.protocol.registered_program_pda,
        ),
        (
            "registered_registry_program_pda",
            env.protocol.registered_registry_program_pda,
        ),
        (
            "registered_forester_pda",
            env.protocol.registered_forester_pda,
        ),
        // ("forester_epoch_pda", forester_epoch_pda),
        // ("epoch_pda", epoch_pda),
    ];

    // Add all v1 state trees
    for tree in &env.v1_state_trees {
        pubkeys.push(("merkle_tree_pubkey", tree.merkle_tree));
        pubkeys.push(("nullifier_queue_pubkey", tree.nullifier_queue));
        pubkeys.push(("cpi_context", tree.cpi_context));
    }

    // V1 address trees are deprecated - do not regenerate
    // They are loaded from existing JSON files in devenv mode
    // for tree in &env.v1_address_trees {
    //     pubkeys.push(("address_merkle_tree", tree.merkle_tree));
    //     pubkeys.push(("address_merkle_tree_queue", tree.queue));
    // }

    // Add all v2 state trees
    for tree in &env.v2_state_trees {
        pubkeys.push(("batch_state_merkle_tree", tree.merkle_tree));
        pubkeys.push(("batched_output_queue", tree.output_queue));
        pubkeys.push(("cpi_context", tree.cpi_context));
    }

    // Add all v2 address trees
    for tree_pubkey in &env.v2_address_trees {
        pubkeys.push(("batch_address_merkle_tree", *tree_pubkey));
    }

    let mut rust_file = String::new();
    let code = quote::quote! {
        use solana_sdk::account::Account;
        use solana_sdk::pubkey::Pubkey;
        use std::str::FromStr;
    };
    rust_file.push_str(&code.to_string());
    for (name, pubkey) in pubkeys {
        println!("pubkey {:?}", pubkey);
        println!("name {:?}", name);
        // Fetch account data. Adjust this part to match how you retrieve and structure your account data.
        let account = rpc.get_account(pubkey).await.unwrap();
        println!(
            "{} DISCRIMINATOR {:?}",
            name,
            account.as_ref().unwrap().data[0..8].to_vec()
        );
        let unwrapped_account = account.unwrap();
        let account = CliAccount::new(&pubkey, &unwrapped_account, true);

        // Serialize the account data to JSON. Adjust according to your data structure.
        let json_data = serde_json::to_vec(&account).unwrap();
        let pubkey = if name == "batch_address_merkle_tree" {
            anchor_lang::pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx")
        } else {
            pubkey
        };

        // Construct the output file path
        let file_name = format!("{}_{}.json", name, pubkey);
        let file_path = format!("{}{}", output_dir, file_name);
        println!("Writing account data to {}", file_path);

        // Write the JSON data to a file in the specified directory
        async_write(file_path.clone(), json_data).await.unwrap();

        if name == "registered_program_pda" || name == "registered_registry_program_pda" {
            let lamports = unwrapped_account.lamports;
            let owner = unwrapped_account.owner.to_string();
            let executable = unwrapped_account.executable;
            let rent_epoch = unwrapped_account.rent_epoch;
            let data = unwrapped_account.data.iter().map(|b| quote::quote! {#b});
            let function_name = format_ident!("get_{}", name);
            let code = quote::quote! {

                pub fn #function_name()-> Account {
                    Account {
                        lamports: #lamports,
                        data: vec![#(#data),*],
                        owner: Pubkey::from_str(#owner).unwrap(),
                        executable: #executable,
                        rent_epoch: #rent_epoch,
                    }
              }

            };
            rust_file.push_str(&code.to_string());
        }
    }
    use std::io::Write;

    let output_path = "../../sdk-libs/program-test/src/test_accounts.rs";
    let mut file = std::fs::File::create(output_path).unwrap();
    file.write_all(
        b"// This file is generated by getAccountState.sh. Do not edit it manually.\n\n",
    )
    .unwrap();
    file.write_all(&rustfmt(rust_file).unwrap()).unwrap();
}

use std::{
    env, io,
    io::Write,
    process::{Command, Stdio},
    thread::spawn,
};

/// Applies `rustfmt` on the given string containing Rust code. The purpose of
/// this function is to be able to format autogenerated code (e.g. with `quote`
/// macro).
pub fn rustfmt(code: String) -> Result<Vec<u8>, io::Error> {
    let mut cmd = match env::var_os("RUSTFMT") {
        Some(r) => Command::new(r),
        None => Command::new("rustfmt"),
    };

    let mut cmd = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = cmd.stdin.take().unwrap();
    let mut stdout = cmd.stdout.take().unwrap();

    let stdin_handle = spawn(move || {
        stdin.write_all(code.as_bytes()).unwrap();
    });

    let mut formatted_code = vec![];
    io::copy(&mut stdout, &mut formatted_code)?;

    let _ = cmd.wait();
    stdin_handle.join().unwrap();

    Ok(formatted_code)
}

/// Tests batched compressed transaction execution:
/// 1. Should succeed: without compressed account (0 lamports), no input compressed account.
/// 2. Should fail: input compressed account with invalid ZKP.
/// 3. Should fail: input compressed account with invalid signer.
/// 4. Should succeed: prove inclusion by index.
/// 5. Should fail: double spend by index
/// 6. Should fail: invalid leaf index
/// 7. Should success: Spend compressed accounts by zkp and index, with v1 and v2 trees
/// 8. Should fail: double-spending by index after spending by ZKP.
/// 9. Should fail: double-spending by ZKP after spending by index.
/// 10. Should fail: double-spending by index after spending by index.
/// 11. Should fail: double-spending by ZKP after spending by ZKP.
/// 12. Should fail: spend account by index which is not in value vec
/// 13. Should fail: spend account v1 by zkp marked as spent by index
#[serial]
#[tokio::test]
async fn batch_invoke_test() {
    let config = ProgramTestConfig::default_test_forester(false);

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    // let protocol_config = rpc.config.protocol_config;
    // setup_forester_and_advance_to_epoch(&mut rpc, &protocol_config)
    //     .await
    //     .unwrap();

    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.v2_state_trees[0].merkle_tree;
    let output_queue_pubkey = env.v2_state_trees[0].output_queue;
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer.pubkey().into(),
        data: None,
        address: None,
    }];
    // 1. Should succeed: without compressed account (0 lamports), no input compressed account.
    create_output_accounts(&mut rpc, &payer, output_queue_pubkey, 1, true)
        .await
        .unwrap();
    let compressed_account_with_context =
        rpc.indexer.as_ref().unwrap().compressed_accounts[0].clone();
    println!(
        "compressed_account_with_context {:?}",
        compressed_account_with_context
    );
    let input_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey.into(),
        data: None,
        address: None,
    }];
    // 2. Should fail: input compressed account with invalid ZKP.
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey: merkle_tree_pubkey.into(),
            leaf_index: 0,
            queue_pubkey: output_queue_pubkey.into(),
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        &[output_queue_pubkey],
        &[Some(0u16)],
        &Vec::new(),
        Some(CompressedProof::default()),
        None,
        false,
        None,
        true,
    );

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
        .await;
    // assert_rpc_error(result, 0, SystemProgramError::ProofVerificationFailed.into()).unwrap();
    assert_rpc_error(
        result,
        0,
        SystemProgramError::ProofVerificationFailed.into(),
    )
    .unwrap();

    // 3. Should fail: input compressed account with invalid signer.
    let invalid_signer_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Keypair::new().pubkey().into(),
        data: None,
        address: None,
    }];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &invalid_signer_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey: merkle_tree_pubkey.into(),
            leaf_index: 0,
            queue_pubkey: output_queue_pubkey.into(),
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        &[merkle_tree_pubkey],
        &[Some(0u16)],
        &Vec::new(),
        None,
        None,
        false,
        None,
        true,
    );

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert_rpc_error(result, 0, SystemProgramError::SignerCheckFailed.into()).unwrap();
    println!("pre 4 ------------------");
    // 4. Should succeed: prove inclusion by index.
    {
        let compressed_account_with_context =
            rpc.indexer.as_ref().unwrap().compressed_accounts[0].clone();
        println!(
            "compressed_account_with_context {:?}",
            compressed_account_with_context
        );
        println!("hash {:?}", compressed_account_with_context.hash());
        let proof_rpc_result = rpc
            .get_validity_proof(
                vec![compressed_account_with_context.hash().unwrap()],
                vec![],
                None,
            )
            .await
            .unwrap();
        // No proof since value is in output queue
        assert!(proof_rpc_result.value.proof.0.is_none());
        // No root index since value is in output queue
        assert!(proof_rpc_result.value.accounts[0]
            .root_index
            .proof_by_index());

        let input_compressed_accounts = vec![compressed_account_with_context.compressed_account];

        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &input_compressed_accounts,
            &output_compressed_accounts,
            &[MerkleContext {
                merkle_tree_pubkey: merkle_tree_pubkey.into(),
                leaf_index: compressed_account_with_context.merkle_context.leaf_index,
                queue_pubkey: output_queue_pubkey.into(),
                prove_by_index: true,
                tree_type: TreeType::StateV2,
            }],
            &[output_queue_pubkey],
            &[],
            &Vec::new(),
            None,
            None,
            false,
            None,
            true,
        );
        println!("Transaction with input proof by index -------------------------");

        TestRpc::create_and_send_transaction_with_public_event(
            &mut rpc,
            &[instruction],
            &payer_pubkey,
            &[&payer],
            Some(TransactionParams {
                v1_input_compressed_accounts: 1,
                v2_input_compressed_accounts: true,
                num_output_compressed_accounts: 1,
                num_new_addresses: 0,
                compress: 0,
                fee_config: FeeConfig::test_batched(),
            }),
        )
        .await
        .unwrap()
        .unwrap();
    }

    // 5. Should fail: double spend by index
    {
        let output_compressed_accounts = vec![CompressedAccount {
            lamports: 0,
            owner: Keypair::new().pubkey().into(),
            data: None,
            address: None,
        }];
        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &input_compressed_accounts,
            &output_compressed_accounts,
            &[MerkleContext {
                merkle_tree_pubkey: merkle_tree_pubkey.into(),
                leaf_index: 0,
                queue_pubkey: output_queue_pubkey.into(),
                prove_by_index: true,
                tree_type: TreeType::StateV2,
            }],
            &[output_queue_pubkey],
            &[],
            &Vec::new(),
            None,
            None,
            false,
            None,
            true,
        );
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    // 6. Should fail: invalid leaf index
    {
        let input_compressed_account = rpc
            .get_compressed_accounts_with_merkle_context_by_owner(&payer_pubkey)
            .iter()
            .filter(|x| x.merkle_context.queue_pubkey.to_bytes() == output_queue_pubkey.to_bytes())
            .next_back()
            .unwrap()
            .clone();
        let output_compressed_accounts = vec![CompressedAccount {
            lamports: 0,
            owner: Keypair::new().pubkey().into(),
            data: None,
            address: None,
        }];
        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &[input_compressed_account.compressed_account],
            &output_compressed_accounts,
            &[MerkleContext {
                merkle_tree_pubkey: merkle_tree_pubkey.into(),
                leaf_index: input_compressed_account.merkle_context.leaf_index - 1,
                queue_pubkey: output_queue_pubkey.into(),
                prove_by_index: true,
                tree_type: TreeType::StateV2,
            }],
            &[output_queue_pubkey],
            &[],
            &Vec::new(),
            None,
            None,
            false,
            None,
            true,
        );
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    // create compressed account in v1 Merkle tree
    {
        let merkle_tree_pubkey = env.v1_state_trees[0].merkle_tree;
        create_output_accounts(&mut rpc, &payer, merkle_tree_pubkey, 1, false)
            .await
            .unwrap();
    }
    println!("pre 7 ------------------");
    // 7. Should success: Spend compressed accounts by zkp and index, with v1 and v2 trees
    {
        let compressed_account_with_context_1 = rpc
            .indexer
            .as_ref()
            .unwrap()
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner.to_bytes() == payer_pubkey.to_bytes()
                    && x.merkle_context.queue_pubkey.to_bytes() == output_queue_pubkey.to_bytes()
            })
            .cloned()
            .collect::<Vec<_>>()
            .last()
            .unwrap()
            .clone();

        let compressed_account_with_context_2 = rpc
            .indexer
            .as_ref()
            .unwrap()
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner.to_bytes() == payer_pubkey.to_bytes()
                    && x.merkle_context.queue_pubkey.to_bytes()
                        == env.v1_state_trees[0].nullifier_queue.to_bytes()
            })
            .collect::<Vec<_>>()[0]
            .clone();
        let proof_rpc_result = rpc
            .get_validity_proof(
                vec![
                    compressed_account_with_context_1.hash().unwrap(),
                    compressed_account_with_context_2.hash().unwrap(),
                ],
                vec![],
                None,
            )
            .await
            .unwrap();

        let proof = proof_rpc_result.value.proof.0.unwrap();

        let input_compressed_accounts = vec![
            compressed_account_with_context_1.compressed_account,
            compressed_account_with_context_2.compressed_account,
        ];

        let merkle_context = vec![
            compressed_account_with_context_1.merkle_context,
            compressed_account_with_context_2.merkle_context,
        ];
        let output_compressed_accounts = vec![
            CompressedAccount {
                lamports: 0,
                owner: payer_pubkey.into(),
                data: None,
                address: None,
            },
            CompressedAccount {
                lamports: 0,
                owner: payer_pubkey.into(),
                data: None,
                address: None,
            },
        ];
        let merkle_context_1 = compressed_account_with_context_1.merkle_context;
        let merkle_context_2 = compressed_account_with_context_2.merkle_context;
        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            input_compressed_accounts.as_slice(),
            &output_compressed_accounts,
            merkle_context.as_slice(),
            &[
                merkle_context_1.queue_pubkey.into(), // output queue
                merkle_context_2.merkle_tree_pubkey.into(),
            ],
            &proof_rpc_result
                .value
                .accounts
                .iter()
                .map(|x| x.root_index.root_index())
                .collect::<Vec<_>>(),
            &Vec::new(),
            Some(proof),
            None,
            false,
            None,
            true,
        );
        println!("Combined Transaction with index and zkp -------------------------");

        Rpc::create_and_send_transaction(&mut rpc, &[instruction], &payer_pubkey, &[&payer])
            .await
            .unwrap();
    }
    create_compressed_accounts_in_batch_merkle_tree(&mut rpc, &payer, output_queue_pubkey)
        .await
        .unwrap();
    println!("pre 8 ------------------");
    // 8. spend account by zkp -> double spend by index
    {
        // Selecting compressed account:
        // - from the end of the array (accounts at the end are in the Merkle tree (onyl 10 are inserted))
        // - Compressed account in the batched Merkle tree
        let compressed_account_with_context_1 = rpc
            .indexer
            .as_ref()
            .unwrap()
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner.to_bytes() == payer_pubkey.to_bytes()
                    && x.merkle_context.queue_pubkey.to_bytes() == output_queue_pubkey.to_bytes()
            })
            .next_back()
            .unwrap()
            .clone();
        let result = double_spend_compressed_account(
            &mut rpc,
            &payer,
            TestMode::ByZkpThenIndex,
            compressed_account_with_context_1.clone(),
        )
        .await;
        assert_rpc_error(
            result,
            1,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    println!("pre 9 ------------------");
    // 9. spend account by index -> double spend by zkp
    {
        // Selecting compressed account:
        // - from the end of the array (accounts at the end are in the Merkle tree (only 10 are inserted))
        // - Compressed account in the batched Merkle tree
        let compressed_account_with_context_1 = rpc
            .indexer
            .as_ref()
            .unwrap()
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner.to_bytes() == payer_pubkey.to_bytes()
                    && x.merkle_context.queue_pubkey.to_bytes() == output_queue_pubkey.to_bytes()
            })
            .next_back()
            .unwrap()
            .clone();
        let result = double_spend_compressed_account(
            &mut rpc,
            &payer,
            TestMode::ByIndexThenZkp,
            compressed_account_with_context_1.clone(),
        )
        .await;
        assert_rpc_error(
            result,
            1,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    println!("pre 10 ------------------");
    // 10. spend account by index -> double spend by index
    {
        // Selecting compressed account:
        // - from the end of the array (accounts at the end are in the Merkle tree (onyl 10 are inserted))
        // - Compressed account in the batched Merkle tree
        let compressed_account_with_context_1 = rpc
            .indexer
            .as_ref()
            .unwrap()
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner.to_bytes() == payer_pubkey.to_bytes()
                    && x.merkle_context.queue_pubkey.to_bytes() == output_queue_pubkey.to_bytes()
            })
            .next_back()
            .unwrap()
            .clone();
        let result = double_spend_compressed_account(
            &mut rpc,
            &payer,
            TestMode::ByIndexThenIndex,
            compressed_account_with_context_1.clone(),
        )
        .await;
        assert_rpc_error(
            result,
            1,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    println!("pre 11 ------------------");
    // 11. spend account by zkp -> double spend by zkp
    {
        // Selecting compressed account:
        // - from the end of the array (accounts at the end are in the Merkle tree (onyl 10 are inserted))
        // - Compressed account in the batched Merkle tree
        let compressed_account_with_context_1 = rpc
            .indexer
            .as_ref()
            .unwrap()
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner.to_bytes() == payer_pubkey.to_bytes()
                    && x.merkle_context.queue_pubkey.to_bytes() == output_queue_pubkey.to_bytes()
            })
            .next_back()
            .unwrap()
            .clone();
        let result = double_spend_compressed_account(
            &mut rpc,
            &payer,
            TestMode::ByZkpThenZkp,
            compressed_account_with_context_1.clone(),
        )
        .await;
        assert_rpc_error(
            result,
            1,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    println!("pre 12 ------------------");
    // 12. spend account by zkp  but mark as spent by index
    {
        create_output_accounts(&mut rpc, &payer, output_queue_pubkey, 1, true)
            .await
            .unwrap();
        let accounts = rpc
            .get_compressed_accounts_by_owner(&payer_pubkey, None, None)
            .await
            .unwrap();
        let accounts = accounts.value.items;
        let accounts = accounts
            .iter()
            .filter(|x| x.tree_info.queue == output_queue_pubkey)
            .collect::<Vec<_>>();
        let compressed_account_with_context_1 = accounts[1].clone();
        // overwrite both output queue batches -> all prior values only exist in the Merkle tree not in the output queue
        for _ in 0..2 {
            create_compressed_accounts_in_batch_merkle_tree(&mut rpc, &payer, output_queue_pubkey)
                .await
                .unwrap();
        }

        // Convert to CompressedAccountWithMerkleContext
        let account_with_context: CompressedAccountWithMerkleContext =
            compressed_account_with_context_1.clone().into();
        let light_merkle_context = compressed_account_with_context_1
            .tree_info
            .to_light_merkle_context(compressed_account_with_context_1.leaf_index, true);

        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &[account_with_context.compressed_account],
            &output_compressed_accounts,
            &[light_merkle_context],
            &[compressed_account_with_context_1.tree_info.queue],
            &[None],
            &Vec::new(),
            None,
            None,
            false,
            None,
            true,
        );

        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    println!("pre 13 ------------------");
    // 13. failing - spend account v1 by zkp but mark as spent by index
    // v1 accounts cannot be spent by index
    {
        // Selecting compressed account in v1 Merkle tree
        let compressed_account_with_context_1 = rpc
            .indexer
            .as_ref()
            .unwrap()
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner.to_bytes() == payer_pubkey.to_bytes()
                    && x.merkle_context.queue_pubkey.to_bytes() != output_queue_pubkey.to_bytes()
            })
            .next_back()
            .unwrap()
            .clone();

        let mut merkle_context = compressed_account_with_context_1.merkle_context;
        merkle_context.prove_by_index = true;
        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &input_compressed_accounts,
            &output_compressed_accounts,
            &[merkle_context],
            &[merkle_context.merkle_tree_pubkey.into()],
            &[None],
            &Vec::new(),
            None,
            None,
            false,
            None,
            true,
        );

        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
            .await;
        // Should fail because it tries to deserialize an output queue account from a nullifier queue account
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::V1AccountMarkedAsProofByIndex.into(),
        )
        .unwrap();
    }
}

#[derive(Debug, PartialEq)]
pub enum TestMode {
    ByZkpThenIndex,
    ByIndexThenZkp,
    ByIndexThenIndex,
    ByZkpThenZkp,
}

pub async fn double_spend_compressed_account<R: Rpc + Indexer + TestRpc>(
    rpc: &mut R,
    payer: &Keypair,
    mode: TestMode,
    compressed_account_with_context_1: CompressedAccountWithMerkleContext,
) -> Result<(), RpcError> {
    let proof_rpc_result = rpc
        .get_validity_proof(
            vec![compressed_account_with_context_1.hash().unwrap()],
            vec![],
            None,
        )
        .await
        .unwrap();
    let input_compressed_accounts = vec![compressed_account_with_context_1.compressed_account];
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer.pubkey().into(),
        data: None,
        address: None,
    }];
    let merkle_context_1 = compressed_account_with_context_1.merkle_context;
    let mut instructions = vec![create_invoke_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[merkle_context_1],
        &[merkle_context_1.queue_pubkey.into()],
        &proof_rpc_result
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        &Vec::new(),
        proof_rpc_result.value.proof.0,
        None,
        false,
        None,
        true,
    )];

    {
        let mut merkle_context = merkle_context_1;
        merkle_context.prove_by_index = true;
        let instruction = create_invoke_instruction(
            &payer.pubkey(),
            &payer.pubkey(),
            &input_compressed_accounts,
            &output_compressed_accounts,
            &[merkle_context],
            &[merkle_context.queue_pubkey.into()],
            &[None],
            &Vec::new(),
            None,
            None,
            false,
            None,
            true,
        );
        if mode == TestMode::ByZkpThenIndex {
            instructions.insert(1, instruction);
        } else if mode == TestMode::ByIndexThenZkp {
            instructions.insert(0, instruction);
        } else if mode == TestMode::ByIndexThenIndex {
            instructions.remove(0);
            instructions.push(instruction.clone());
            instructions.push(instruction);
        }
    }
    if mode == TestMode::ByZkpThenZkp {
        let instruction = instructions[0].clone();
        instructions.push(instruction);
    }
    TestRpc::create_and_send_transaction_with_public_event(
        rpc,
        &instructions,
        &payer.pubkey(),
        &[payer],
        None,
    )
    .await?
    .unwrap();
    Ok(())
}

/// fill batch and perform batch append
pub async fn create_compressed_accounts_in_batch_merkle_tree(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    output_queue_pubkey: Pubkey,
) -> Result<(), RpcError> {
    let mut output_queue_account = rpc.get_account(output_queue_pubkey).await.unwrap().unwrap();
    let output_queue =
        BatchedQueueAccount::output_from_bytes(&mut output_queue_account.data).unwrap();
    let fullness = output_queue.get_num_inserted_in_current_batch();
    let remaining_leaves = output_queue.get_metadata().batch_metadata.batch_size - fullness;

    for _ in 0..remaining_leaves {
        create_output_accounts(rpc, payer, output_queue_pubkey, 1, true).await?;
    }
    for i in 0..output_queue
        .get_metadata()
        .batch_metadata
        .get_num_zkp_batches()
    {
        println!("Performing batch append {}", i);

        let forester = rpc.test_accounts.protocol.forester.insecure_clone();
        let (index, mut bundle) = TestIndexerExtensions::get_state_merkle_trees_mut(rpc)
            .iter()
            .enumerate()
            .find(|(_, x)| x.accounts.nullifier_queue == output_queue_pubkey)
            .map(|(x, bundle)| (x, (*bundle).clone()))
            .unwrap();
        perform_batch_append(rpc, &mut bundle, &forester, 0, false, None).await?;
        rpc.indexer.as_mut().unwrap().state_merkle_trees[index] = bundle;
    }
    Ok(())
}
pub async fn create_output_accounts(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    output_queue_pubkey: Pubkey,
    num_accounts: usize,
    is_batched: bool,
) -> Result<Signature, RpcError> {
    let output_compressed_accounts = vec![
        CompressedAccount {
            lamports: 0,
            owner: payer.pubkey().into(),
            data: None,
            address: None,
        };
        num_accounts
    ];
    println!("payer.pubkey() {:?}", payer.pubkey());
    let output_merkle_tree_pubkeys = vec![output_queue_pubkey; num_accounts];
    let instruction = create_invoke_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &Vec::new(),
        &output_compressed_accounts,
        &Vec::new(),
        output_merkle_tree_pubkeys.as_slice(),
        &Vec::new(),
        &Vec::new(),
        None,
        None,
        false,
        None,
        true,
    );
    let fee_config = if is_batched {
        FeeConfig::test_batched()
    } else {
        FeeConfig::default()
    };

    let (event, signature, _) = TestRpc::create_and_send_transaction_with_public_event(
        rpc,
        &[instruction],
        &payer.pubkey(),
        &[payer],
        Some(TransactionParams {
            v1_input_compressed_accounts: 0u8,
            v2_input_compressed_accounts: is_batched,
            num_output_compressed_accounts: num_accounts as u8,
            num_new_addresses: 0,
            compress: 0,
            fee_config,
        }),
    )
    .await
    .unwrap()
    .unwrap();
    // Assertion.
    {
        let mut test_indexer = rpc.clone_indexer()?;
        let slot: u64 = rpc.get_slot().await.unwrap();
        let (created_compressed_accounts, _) =
            test_indexer.add_event_and_compressed_accounts(slot, &event);
        assert_created_compressed_accounts(
            output_compressed_accounts.as_slice(),
            output_merkle_tree_pubkeys.as_slice(),
            created_compressed_accounts.as_slice(),
        );
    }
    Ok(signature)
}
