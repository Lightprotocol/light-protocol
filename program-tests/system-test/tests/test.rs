#![cfg(feature = "test-sbf")]

use account_compression::errors::AccountCompressionErrorCode;
use anchor_lang::{error::ErrorCode, AnchorSerialize, InstructionData, ToAccountMetas};
use light_batched_merkle_tree::{
    errors::BatchedMerkleTreeError,
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData, queue::BatchedQueueAccount,
};
use light_client::indexer::Indexer;
use light_hasher::Poseidon;
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    test_batch_forester::perform_batch_append,
    test_env::{
        initialize_accounts, setup_test_programs, setup_test_programs_with_accounts,
        EnvAccountKeypairs, EnvAccounts,
    },
    test_rpc::ProgramTestRpcConnection,
};
use light_prover_client::gnark::helpers::{spawn_prover, ProofType, ProverConfig, ProverMode};
use light_registry::protocol_config::state::ProtocolConfig;
use light_sdk::merkle_context::QueueIndex as SdkQueueIndex;
use light_system_program::{
    errors::SystemProgramError,
    invoke::processor::CompressedProof,
    sdk::{
        address::{derive_address, derive_address_legacy},
        compressed_account::{
            CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext,
            MerkleContext, QueueIndex,
        },
        invoke::{
            create_invoke_instruction, create_invoke_instruction_data_and_remaining_accounts,
        },
    },
    utils::{get_cpi_authority_pda, get_registered_program_pda},
    InstructionDataInvoke, NewAddressParams,
};
use light_test_utils::{
    airdrop_lamports,
    assert_compressed_tx::assert_created_compressed_accounts,
    assert_custom_error_or_program_error, assert_rpc_error,
    conversions::{
        sdk_to_program_compressed_account, sdk_to_program_compressed_account_with_merkle_context,
        sdk_to_program_compressed_proof, sdk_to_program_merkle_context,
    },
    system_program::{
        compress_sol_test, create_addresses_test, decompress_sol_test, transfer_compressed_sol_test,
    },
    FeeConfig, RpcConnection, RpcError, TransactionParams,
};
use light_utils::{hash_to_bn254_field_size_be, UtilsError};
use light_verifier::VerifierError;
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
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    spawn_prover(
        true,
        ProverConfig {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        },
    )
    .await;
    let payer = context.get_payer().insecure_clone();
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
        &mut context,
        &payer,
        inputs_struct,
        remaining_accounts,
        SystemProgramError::EmptyInputs.into(),
    )
    .await
    .unwrap();

    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, None).await;
    // circuit instantiations allow for 1, 2, 3, 4, 8 inclusion proofs
    let options = [0usize, 1usize, 2usize, 3usize, 4usize, 8usize];

    for mut num_addresses in 0..=2 {
        for j in 0..6 {
            // there is no combined circuit instantiation for 8 inputs and addresses
            if j == 5 {
                num_addresses = 0;
            }
            for num_outputs in 1..8 {
                failing_transaction_inputs(
                    &mut context,
                    &mut test_indexer,
                    &payer,
                    &env,
                    options[j],
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
        for j in 0..6 {
            // there is no combined circuit instantiation for 8 inputs and addresses
            if j == 5 {
                num_addresses = 0;
            }
            for num_outputs in 0..8 {
                failing_transaction_inputs(
                    &mut context,
                    &mut test_indexer,
                    &payer,
                    &env,
                    options[j],
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
pub async fn failing_transaction_inputs<
    R: RpcConnection,
    I: Indexer<R> + TestIndexerExtensions<R>,
>(
    context: &mut R,
    test_indexer: &mut I,
    payer: &Keypair,
    env: &EnvAccounts,
    num_inputs: usize,
    amount: u64,
    num_addresses: usize,
    num_outputs: usize,
    output_compressed_accounts_with_address: bool,
) -> Result<(), RpcError> {
    // create compressed accounts that can be used as inputs
    for _ in 0..num_inputs {
        compress_sol_test(
            context,
            test_indexer,
            payer,
            &[],
            false,
            amount,
            &env.merkle_tree_pubkey,
            None,
        )
        .await
        .unwrap();
    }
    let (mut new_address_params, derived_addresses) =
        create_address_test_inputs(env, num_addresses);
    let input_compressed_accounts = test_indexer
        .get_compressed_accounts_with_merkle_context_by_owner(&payer.pubkey())[0..num_inputs]
        .to_vec();
    let hashes = input_compressed_accounts
        .iter()
        .map(|x| x.hash().unwrap())
        .collect::<Vec<_>>();
    let input_compressed_account_hashes = if num_inputs != 0 { Some(hashes) } else { None };
    let mts = input_compressed_accounts
        .iter()
        .map(|x| x.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<_>>();
    let input_state_merkle_trees = if num_inputs != 0 { Some(mts) } else { None };
    let proof_input_derived_addresses = if num_addresses != 0 {
        Some(derived_addresses.as_slice())
    } else {
        None
    };
    let proof_input_address_merkle_tree_pubkeys = if num_addresses != 0 {
        Some(vec![env.address_merkle_tree_pubkey; num_addresses])
    } else {
        None
    };

    let (root_indices, proof) =
        if input_compressed_account_hashes.is_some() || proof_input_derived_addresses.is_some() {
            let proof_rpc_res = test_indexer
                .create_proof_for_compressed_accounts(
                    input_compressed_account_hashes,
                    input_state_merkle_trees,
                    proof_input_derived_addresses,
                    proof_input_address_merkle_tree_pubkeys,
                    context,
                )
                .await;
            for (i, root_index) in proof_rpc_res.address_root_indices.iter().enumerate() {
                new_address_params[i].address_merkle_tree_root_index = *root_index;
            }
            (
                proof_rpc_res.root_indices,
                Some(sdk_to_program_compressed_proof(proof_rpc_res.proof)),
            )
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
        for i in 0..num_outputs {
            let address = if output_compressed_accounts_with_address && i < num_addresses {
                Some(derived_addresses[i])
            } else {
                None
            };
            output_compressed_accounts.push(CompressedAccount {
                lamports: output_amount,
                owner: payer.pubkey(),
                data: None,
                address,
            });
            output_merkle_tree_pubkeys.push(env.merkle_tree_pubkey);
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
            .map(|x| sdk_to_program_merkle_context(x))
            .collect::<Vec<_>>(),
        &input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .map(|x| sdk_to_program_compressed_account(x))
            .collect::<Vec<_>>(),
        &root_indices,
        &output_merkle_tree_pubkeys,
        &output_compressed_accounts,
        proof,
        None,
        false,
    );
    if num_addresses > 0 {
        failing_transaction_address(
            context,
            payer,
            env,
            &inputs_struct,
            remaining_accounts.clone(),
        )
        .await?;
    }
    if num_inputs > 0 {
        failing_transaction_inputs_inner(
            context,
            payer,
            env,
            &inputs_struct,
            remaining_accounts.clone(),
        )
        .await?;
    }
    if num_outputs > 0 {
        failing_transaction_output(
            context,
            payer,
            env,
            inputs_struct,
            remaining_accounts.clone(),
        )
        .await?;
    }
    Ok(())
}

pub async fn failing_transaction_inputs_inner<R: RpcConnection>(
    context: &mut R,
    payer: &Keypair,
    env: &EnvAccounts,
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
        inputs_struct.proof.as_mut().unwrap().a = inputs_struct.proof.as_ref().unwrap().c.clone();
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            VerifierError::ProofVerificationFailed.into(),
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
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            VerifierError::ProofVerificationFailed.into(),
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
    //         context,
    //         payer,
    //         inputs_struct,
    //         remaining_accounts.clone(),
    //         VerifierError::ProofVerificationFailed.into(),
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
            VerifierError::ProofVerificationFailed.into()
        } else {
            SystemProgramError::SumCheckFailed.into()
        };

        create_instruction_and_failing_transaction(
            context,
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
            .address = Some(hash_to_bn254_field_size_be([1u8; 32].as_slice()).unwrap().0);
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            VerifierError::ProofVerificationFailed.into(),
        )
        .await
        .unwrap();
    }
    // invalid account data (owner)
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
            .compressed_account
            .owner = Keypair::new().pubkey();

        create_instruction_and_failing_transaction(
            context,
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
            data_hash: hash_to_bn254_field_size_be([1u8; 32].as_slice()).unwrap().0,
        };
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
            .compressed_account
            .data = Some(data);
        create_instruction_and_failing_transaction(
            context,
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
            pubkey: env.address_merkle_tree_pubkey,
            is_signer: false,
            is_writable: false,
        };
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch.into(),
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
            .nullifier_queue_pubkey_index as usize] = AccountMeta {
            pubkey: env.address_merkle_tree_queue_pubkey,
            is_signer: false,
            is_writable: true,
        };
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            MerkleTreeMetadataError::InvalidQueueType.into(),
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
            .nullifier_queue_pubkey_index as usize] = AccountMeta {
            pubkey: env.address_merkle_tree_pubkey,
            is_signer: false,
            is_writable: true,
        };
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            ErrorCode::AccountDiscriminatorMismatch.into(),
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
            context,
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
    env: &EnvAccounts,
    num_addresses: usize,
) -> (Vec<NewAddressParams>, Vec<[u8; 32]>) {
    let mut address_seeds = vec![];
    for i in 1..=num_addresses {
        address_seeds.push([i as u8; 32]);
    }

    let mut new_address_params = vec![];
    let mut derived_addresses = Vec::new();
    for (_, address_seed) in address_seeds.iter().enumerate() {
        new_address_params.push(NewAddressParams {
            seed: *address_seed,
            address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
            address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
            address_merkle_tree_root_index: 0,
        });
        let derived_address =
            derive_address_legacy(&env.address_merkle_tree_pubkey, address_seed).unwrap();
        derived_addresses.push(derived_address);
    }
    (new_address_params, derived_addresses)
}

pub async fn failing_transaction_address<R: RpcConnection>(
    context: &mut R,
    payer: &Keypair,
    env: &EnvAccounts,
    inputs_struct: &InstructionDataInvoke,
    remaining_accounts: Vec<AccountMeta>,
) -> Result<(), RpcError> {
    // inconsistent seed
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.new_address_params[0].seed = [100u8; 32];
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            VerifierError::ProofVerificationFailed.into(),
        )
        .await
        .unwrap();
    }
    // invalid proof
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.proof.as_mut().unwrap().a = inputs_struct.proof.as_ref().unwrap().c.clone();
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            VerifierError::ProofVerificationFailed.into(),
        )
        .await
        .unwrap();
    }
    // invalid root index
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.new_address_params[0].address_merkle_tree_root_index = 0;
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            VerifierError::ProofVerificationFailed.into(),
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
                pubkey: env.nullifier_queue_pubkey,
                is_signer: false,
                is_writable: false,
            };
        create_instruction_and_failing_transaction(
            context,
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
                pubkey: env.merkle_tree_pubkey,
                is_signer: false,
                is_writable: false,
            };
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            ErrorCode::AccountDiscriminatorMismatch.into(),
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
                pubkey: env.merkle_tree_pubkey,
                is_signer: false,
                is_writable: false,
            };
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch.into(),
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
pub async fn failing_transaction_output<R: RpcConnection>(
    context: &mut R,
    payer: &Keypair,
    env: &EnvAccounts,
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
            context,
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
            context,
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
            pubkey: env.address_merkle_tree_pubkey,
            is_signer: false,
            is_writable: false,
        };
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct.clone(),
            remaining_accounts.clone(),
            AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch.into(),
        )
        .await
        .unwrap();
    }

    // Address that doesn't exist
    {
        let mut inputs_struct = inputs_struct.clone();

        for account in inputs_struct.output_compressed_accounts.iter_mut() {
            let address = Some(
                hash_to_bn254_field_size_be(Keypair::new().pubkey().to_bytes().as_slice())
                    .unwrap()
                    .0,
            );
            account.compressed_account.address = address;
        }

        create_instruction_and_failing_transaction(
            context,
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
    context: &mut ProgramTestRpcConnection,
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
    let result = context
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[payer])
        .await;
    assert_rpc_error(result, 0, expected_error_code)
}

pub async fn create_instruction_and_failing_transaction<R: RpcConnection>(
    context: &mut R,
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

    let result = context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await;
    assert_rpc_error(result, 0, expected_error_code)
}

/// Tests Execute compressed transaction:
/// 1. should succeed: without compressed account(0 lamports), no in compressed account
/// 2. should fail: in compressed account and invalid zkp
/// 3. should fail: in compressed account and invalid signer
/// 4. should succeed: in compressed account inserted in (1.) and valid zkp
#[serial]
#[tokio::test]
async fn invoke_test() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;

    let payer = context.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::<ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        Some(ProverConfig {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        }),
    )
    .await;

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
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

    let event = context
        .create_and_send_transaction_with_event(
            &[instruction],
            &payer_pubkey,
            &[&payer],
            Some(TransactionParams {
                num_input_compressed_accounts: 0,
                num_output_compressed_accounts: 1,
                num_new_addresses: 0,
                compress: 0,
                fee_config: FeeConfig::default(),
            }),
        )
        .await
        .unwrap()
        .unwrap();
    let slot: u64 = context.get_slot().await.unwrap();
    let (created_compressed_accounts, _) =
        test_indexer.add_event_and_compressed_accounts(slot, &event.0);
    let created_compressed_accounts = created_compressed_accounts
        .into_iter()
        .map(sdk_to_program_compressed_account_with_merkle_context)
        .collect::<Vec<_>>();
    assert_created_compressed_accounts(
        output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys.as_slice(),
        created_compressed_accounts.as_slice(),
    );

    let input_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
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
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey: nullifier_queue_pubkey,
            queue_index: None,
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

    let res = context
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
        .await;
    assert!(res.is_err());

    // check invalid signer for in compressed_account
    let invalid_signer_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Keypair::new().pubkey(),
        data: None,
        address: None,
    }];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &invalid_signer_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey: nullifier_queue_pubkey,
            queue_index: None,
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

    let res = context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(res.is_err());

    // create Merkle proof
    // get zkp from server
    // create instruction as usual with correct zkp
    let compressed_account_with_context = test_indexer.compressed_accounts[0].clone();
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            Some(vec![compressed_account_with_context
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.merkle_context.leaf_index,
                )
                .unwrap()]),
            Some(vec![
                compressed_account_with_context
                    .merkle_context
                    .merkle_tree_pubkey,
            ]),
            None,
            None,
            &mut context,
        )
        .await;
    let proof = sdk_to_program_compressed_proof(proof_rpc_res.proof.clone());
    let input_compressed_accounts = vec![sdk_to_program_compressed_account(
        compressed_account_with_context.compressed_account,
    )];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey: nullifier_queue_pubkey,
            queue_index: None,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof.clone()),
        None,
        false,
        None,
        true,
    );
    println!("Transaction with zkp -------------------------");

    let event = context
        .create_and_send_transaction_with_event(
            &[instruction],
            &payer_pubkey,
            &[&payer],
            Some(TransactionParams {
                num_input_compressed_accounts: 1,
                num_output_compressed_accounts: 1,
                num_new_addresses: 0,
                compress: 0,
                fee_config: FeeConfig::default(),
            }),
        )
        .await
        .unwrap()
        .unwrap();
    let slot: u64 = context.get_slot().await.unwrap();
    test_indexer.add_event_and_compressed_accounts(slot, &event.0);

    println!("Double spend -------------------------");
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Keypair::new().pubkey(),
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
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey: nullifier_queue_pubkey,
            queue_index: None,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof.clone()),
        None,
        false,
        None,
        true,
    );
    let res = context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(res.is_err());
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Keypair::new().pubkey(),
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
            merkle_tree_pubkey,
            leaf_index: 1,
            nullifier_queue_pubkey: nullifier_queue_pubkey,
            queue_index: None,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof.clone()),
        None,
        false,
        None,
        true,
    );
    let res = context
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
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::<ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        Some(ProverConfig {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        }),
    )
    .await;

    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;

    let address_seed = [1u8; 32];
    let derived_address =
        derive_address_legacy(&env.address_merkle_tree_pubkey, &address_seed).unwrap();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
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
        context.get_latest_blockhash().await.unwrap(),
    );

    let res = context.process_transaction(transaction).await;
    assert_custom_error_or_program_error(res, SystemProgramError::InvalidAddress.into()).unwrap();
    // v1 address tree with new derivation should fail
    {
        let derived_address = derive_address(
            &address_seed,
            &env.batch_address_merkle_tree.to_bytes(),
            &payer_pubkey.to_bytes(),
        );
        let output_compressed_accounts = vec![CompressedAccount {
            lamports: 0,
            owner: payer_pubkey,
            data: None,
            address: Some(derived_address), // this should not be sent, only derived on-chain
        }];

        let address_params = vec![NewAddressParams {
            seed: address_seed,
            address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
            address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
            address_merkle_tree_root_index: 0,
        }];
        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &Vec::new(),
            &output_compressed_accounts,
            &Vec::new(),
            &[env.batched_output_queue],
            &Vec::new(),
            &address_params,
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
            context.get_latest_blockhash().await.unwrap(),
        );

        let res = context.process_transaction(transaction).await;
        assert_custom_error_or_program_error(res, SystemProgramError::InvalidAddress.into())
            .unwrap();
    }
    // batch address tree with new derivation should fail with invoke because invoking program is not provided.
    {
        let derived_address = derive_address(
            &address_seed,
            &env.batch_address_merkle_tree.to_bytes(),
            &payer_pubkey.to_bytes(),
        );
        let output_compressed_accounts = vec![CompressedAccount {
            lamports: 0,
            owner: payer_pubkey,
            data: None,
            address: Some(derived_address), // this should not be sent, only derived on-chain
        }];
        let address_params = vec![NewAddressParams {
            seed: address_seed,
            address_queue_pubkey: env.batch_address_merkle_tree,
            address_merkle_tree_pubkey: env.batch_address_merkle_tree,
            address_merkle_tree_root_index: 0,
        }];

        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &Vec::new(),
            &output_compressed_accounts,
            &Vec::new(),
            &[env.batched_output_queue],
            &Vec::new(),
            &address_params,
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
            context.get_latest_blockhash().await.unwrap(),
        );

        let res = context.process_transaction(transaction).await;
        assert_custom_error_or_program_error(res, SystemProgramError::DeriveAddressError.into())
            .unwrap();
    }
    println!("creating address -------------------------");
    create_addresses_test(
        &mut context,
        &mut test_indexer,
        &[env.address_merkle_tree_pubkey],
        &[env.address_merkle_tree_queue_pubkey],
        vec![env.merkle_tree_pubkey],
        &[address_seed],
        &Vec::new(),
        false,
        None,
    )
    .await
    .unwrap();
    // transfer with address
    println!("transfer with address-------------------------");

    let compressed_account_with_context = sdk_to_program_compressed_account_with_merkle_context(
        test_indexer.compressed_accounts[0].clone(),
    );
    let recipient_pubkey = Keypair::new().pubkey();
    transfer_compressed_sol_test(
        &mut context,
        &mut test_indexer,
        &payer,
        &[compressed_account_with_context.clone()],
        &[recipient_pubkey],
        &[compressed_account_with_context
            .merkle_context
            .merkle_tree_pubkey],
        None,
    )
    .await
    .unwrap();
    assert_eq!(test_indexer.compressed_accounts.len(), 1);
    assert_eq!(
        test_indexer.compressed_accounts[0]
            .compressed_account
            .address
            .unwrap(),
        derived_address
    );
    assert_eq!(
        test_indexer.compressed_accounts[0].compressed_account.owner,
        recipient_pubkey
    );

    let address_seed_2 = [2u8; 32];

    let event = create_addresses_test(
        &mut context,
        &mut test_indexer,
        &[
            env.address_merkle_tree_pubkey,
            env.address_merkle_tree_pubkey,
        ],
        &[
            env.address_merkle_tree_queue_pubkey,
            env.address_merkle_tree_queue_pubkey,
        ],
        vec![env.merkle_tree_pubkey, env.merkle_tree_pubkey],
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
    create_addresses_test(
        &mut context,
        &mut test_indexer,
        &[
            env.address_merkle_tree_pubkey,
            env.address_merkle_tree_pubkey,
        ],
        &[
            env.address_merkle_tree_queue_pubkey,
            env.address_merkle_tree_queue_pubkey,
        ],
        vec![env.merkle_tree_pubkey, env.merkle_tree_pubkey],
        &[address_seed_2, address_seed_3],
        &Vec::new(),
        false,
        None,
    )
    .await
    .unwrap();

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
        let compressed_input_accounts = test_indexer
            .get_compressed_accounts_with_merkle_context_by_owner(&payer_pubkey)
            [0..n_input_compressed_accounts]
            .to_vec();
        let compressed_input_accounts = compressed_input_accounts
            .into_iter()
            .map(sdk_to_program_compressed_account_with_merkle_context)
            .collect::<Vec<_>>();
        let mut address_vec = Vec::new();
        // creates multiple seeds by taking the number of input accounts and zeroing out the jth byte
        for j in 0..n_new_addresses {
            let mut address_seed = [n_input_compressed_accounts as u8; 32];
            address_seed[j + (n_new_addresses * 2)] = 0_u8;
            address_vec.push(address_seed);
        }

        create_addresses_test(
            &mut context,
            &mut test_indexer,
            &vec![env.address_merkle_tree_pubkey; n_new_addresses],
            &vec![env.address_merkle_tree_queue_pubkey; n_new_addresses],
            vec![env.merkle_tree_pubkey; n_new_addresses],
            &address_vec,
            &compressed_input_accounts,
            true,
            None,
        )
        .await
        .unwrap();
    }
}

#[serial]
#[tokio::test]
async fn test_with_compression() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.get_payer().insecure_clone();

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mut test_indexer = TestIndexer::<ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        Some(ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::Inclusion],
        }),
    )
    .await;
    let compress_amount = 1_000_000;
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: compress_amount + 1,
        owner: payer_pubkey,
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
        context.get_latest_blockhash().await.unwrap(),
    );

    let result = context.process_transaction(transaction).await;
    // should fail because of insufficient input funds
    assert_custom_error_or_program_error(result, SystemProgramError::ComputeOutputSumFailed.into())
        .unwrap();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: compress_amount,
        owner: payer_pubkey,
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
        context.get_latest_blockhash().await.unwrap(),
    );

    let result = context.process_transaction(transaction).await;
    // should fail because of insufficient decompress amount funds
    assert_custom_error_or_program_error(result, SystemProgramError::ComputeOutputSumFailed.into())
        .unwrap();

    compress_sol_test(
        &mut context,
        &mut test_indexer,
        &payer,
        &Vec::new(),
        false,
        compress_amount,
        &env.merkle_tree_pubkey,
        None,
    )
    .await
    .unwrap();

    let compressed_account_with_context = test_indexer.compressed_accounts.last().unwrap().clone();
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            Some(vec![compressed_account_with_context
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.merkle_context.leaf_index,
                )
                .unwrap()]),
            Some(vec![
                compressed_account_with_context
                    .merkle_context
                    .merkle_tree_pubkey,
            ]),
            None,
            None,
            &mut context,
        )
        .await;
    let proof = sdk_to_program_compressed_proof(proof_rpc_res.proof.clone());
    let input_compressed_accounts = vec![sdk_to_program_compressed_account(
        compressed_account_with_context.clone().compressed_account,
    )];
    let recipient_pubkey = Keypair::new().pubkey();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: recipient_pubkey,
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
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey: nullifier_queue_pubkey,
            queue_index: None,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof.clone()),
        Some(compress_amount),
        true,
        Some(recipient),
        true,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_latest_blockhash().await.unwrap(),
    );
    println!("Transaction with zkp -------------------------");

    let result = context.process_transaction(transaction).await;
    // should fail because of insufficient output funds
    assert_custom_error_or_program_error(result, SystemProgramError::SumCheckFailed.into())
        .unwrap();

    let compressed_account_with_context =
        test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&payer_pubkey)[0].clone();
    let compressed_account_with_context =
        sdk_to_program_compressed_account_with_merkle_context(compressed_account_with_context);

    decompress_sol_test(
        &mut context,
        &mut test_indexer,
        &payer,
        &vec![compressed_account_with_context],
        &recipient_pubkey,
        compress_amount,
        &env.merkle_tree_pubkey,
        None,
    )
    .await
    .unwrap();
}

#[ignore = "this is a helper function to regenerate accounts"]
#[serial]
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

    let context = setup_test_programs(None).await;
    let mut context = ProgramTestRpcConnection { context };
    let keypairs = EnvAccountKeypairs::for_regenerate_accounts();

    airdrop_lamports(
        &mut context,
        &keypairs.governance_authority.pubkey(),
        100_000_000_000,
    )
    .await
    .unwrap();

    airdrop_lamports(&mut context, &keypairs.forester.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    // Note this will not regenerate the registered program accounts.
    let skip_register_programs = true;
    let env = initialize_accounts(
        &mut context,
        keypairs,
        protocol_config,
        true,
        skip_register_programs,
        InitStateTreeAccountsInstructionData::test_default(),
        InitAddressTreeAccountsInstructionData::test_default(),
    )
    .await;

    // List of public keys to fetch and export
    let pubkeys = vec![
        ("merkle_tree_pubkey", env.merkle_tree_pubkey),
        ("nullifier_queue_pubkey", env.nullifier_queue_pubkey),
        ("governance_authority_pda", env.governance_authority_pda),
        ("group_pda", env.group_pda),
        ("registered_program_pda", env.registered_program_pda),
        ("address_merkle_tree", env.address_merkle_tree_pubkey),
        (
            "address_merkle_tree_queue",
            env.address_merkle_tree_queue_pubkey,
        ),
        ("cpi_context", env.cpi_context_account_pubkey),
        (
            "registered_registry_program_pda",
            env.registered_registry_program_pda,
        ),
        ("registered_forester_pda", env.registered_forester_pda),
        (
            "forester_epoch_pda",
            env.forester_epoch.as_ref().unwrap().forester_epoch_pda,
        ),
        ("epoch_pda", env.forester_epoch.as_ref().unwrap().epoch_pda),
        ("batch_state_merkle_tree", env.batched_state_merkle_tree),
        ("batched_output_queue", env.batched_output_queue),
        ("batch_address_merkle_tree", env.batch_address_merkle_tree),
    ];

    let mut rust_file = String::new();
    let code = quote::quote! {
        use solana_sdk::account::Account;
        use solana_sdk::pubkey::Pubkey;
        use std::str::FromStr;
    };
    rust_file.push_str(&code.to_string());
    for (name, pubkey) in pubkeys {
        // Fetch account data. Adjust this part to match how you retrieve and structure your account data.
        let account = context.get_account(pubkey).await.unwrap();
        println!(
            "{} DISCRIMINATOR {:?}",
            name,
            account.as_ref().unwrap().data[0..8].to_vec()
        );
        let unwrapped_account = account.unwrap();
        let account = CliAccount::new(&pubkey, &unwrapped_account, true);

        // Serialize the account data to JSON. Adjust according to your data structure.
        let json_data = serde_json::to_vec(&account).unwrap();

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

    use light_utils::rustfmt;
    let output_path = "../../sdk-libs/program-test/src/env_accounts.rs";
    let mut file = std::fs::File::create(&output_path).unwrap();
    file.write_all(
        b"// This file is generated by getAccountState.sh. Do not edit it manually.\n\n",
    )
    .unwrap();
    file.write_all(&rustfmt(rust_file).unwrap()).unwrap();
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
    let (mut context, env) = setup_test_programs_with_accounts(None).await;

    let payer = context.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::<ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        Some(ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::Inclusion, ProofType::BatchAppendWithProofsTest],
        }),
    )
    .await;
    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.batched_state_merkle_tree;
    let output_queue_pubkey = env.batched_output_queue;
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer.pubkey(),
        data: None,
        address: None,
    }];
    // 1. Should succeed: without compressed account (0 lamports), no input compressed account.
    create_output_accounts(
        &mut context,
        &payer,
        &mut test_indexer,
        output_queue_pubkey,
        1,
        true,
    )
    .await
    .unwrap();

    let input_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
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
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey: output_queue_pubkey,
            queue_index: None,
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

    let result = context
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
        .await;
    assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();

    // 3. Should fail: input compressed account with invalid signer.
    let invalid_signer_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Keypair::new().pubkey(),
        data: None,
        address: None,
    }];

    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &invalid_signer_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey: output_queue_pubkey,
            queue_index: None,
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

    let result = context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert_rpc_error(result, 0, SystemProgramError::SignerCheckFailed.into()).unwrap();

    // 4. Should succeed: prove inclusion by index.
    {
        let compressed_account_with_context = test_indexer.compressed_accounts[0].clone();
        let proof_rpc_result = test_indexer
            .create_proof_for_compressed_accounts2(
                Some(vec![compressed_account_with_context.hash().unwrap()]),
                Some(vec![
                    compressed_account_with_context
                        .merkle_context
                        .merkle_tree_pubkey,
                ]),
                None,
                None,
                &mut context,
            )
            .await;
        // No proof since value is in output queue
        assert!(proof_rpc_result.proof.is_none());
        // No root index since value is in output queue
        assert!(proof_rpc_result.root_indices[0].is_none());

        let input_compressed_accounts = vec![sdk_to_program_compressed_account(
            compressed_account_with_context.compressed_account,
        )];

        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &input_compressed_accounts,
            &output_compressed_accounts,
            &[MerkleContext {
                merkle_tree_pubkey,
                leaf_index: compressed_account_with_context.merkle_context.leaf_index,
                nullifier_queue_pubkey: output_queue_pubkey,
                // Values are not used, it only has to be Some
                queue_index: Some(QueueIndex {
                    index: 123,
                    queue_id: 200,
                }),
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

        let event = context
            .create_and_send_transaction_with_event(
                &[instruction],
                &payer_pubkey,
                &[&payer],
                Some(TransactionParams {
                    num_input_compressed_accounts: 1,
                    num_output_compressed_accounts: 1,
                    num_new_addresses: 0,
                    compress: 0,
                    fee_config: FeeConfig::test_batched(),
                }),
            )
            .await
            .unwrap()
            .unwrap();
        let slot: u64 = context.get_slot().await.unwrap();
        test_indexer.add_event_and_compressed_accounts(slot, &event.0);
    }

    // 5. Should fail: double spend by index
    {
        let output_compressed_accounts = vec![CompressedAccount {
            lamports: 0,
            owner: Keypair::new().pubkey(),
            data: None,
            address: None,
        }];
        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &input_compressed_accounts,
            &output_compressed_accounts,
            &[MerkleContext {
                merkle_tree_pubkey,
                leaf_index: 0,
                nullifier_queue_pubkey: output_queue_pubkey,
                queue_index: Some(QueueIndex {
                    index: 123,
                    queue_id: 200,
                }),
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
        let result = context
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
        let input_compressed_account = test_indexer
            .get_compressed_accounts_with_merkle_context_by_owner(&payer_pubkey)
            .iter()
            .filter(|x| x.merkle_context.nullifier_queue_pubkey == output_queue_pubkey)
            .last()
            .unwrap()
            .clone();
        let output_compressed_accounts = vec![CompressedAccount {
            lamports: 0,
            owner: Keypair::new().pubkey(),
            data: None,
            address: None,
        }];
        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &[sdk_to_program_compressed_account(
                input_compressed_account.compressed_account,
            )],
            &output_compressed_accounts,
            &[MerkleContext {
                merkle_tree_pubkey,
                leaf_index: input_compressed_account.merkle_context.leaf_index - 1,
                nullifier_queue_pubkey: output_queue_pubkey,
                queue_index: Some(QueueIndex {
                    index: 123,
                    queue_id: 200,
                }),
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
        let result = context
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
        let merkle_tree_pubkey = env.merkle_tree_pubkey;
        create_output_accounts(
            &mut context,
            &payer,
            &mut test_indexer,
            merkle_tree_pubkey,
            1,
            false,
        )
        .await
        .unwrap();
    }
    // 7. Should success: Spend compressed accounts by zkp and index, with v1 and v2 trees
    {
        let compressed_account_with_context_1 = test_indexer
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner == payer_pubkey
                    && x.merkle_context.nullifier_queue_pubkey == output_queue_pubkey
            })
            .cloned()
            .collect::<Vec<_>>()
            .last()
            .unwrap()
            .clone();

        let compressed_account_with_context_2 = test_indexer
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner == payer_pubkey
                    && x.merkle_context.nullifier_queue_pubkey == env.nullifier_queue_pubkey
            })
            .collect::<Vec<_>>()[0]
            .clone();
        let proof_rpc_result = test_indexer
            .create_proof_for_compressed_accounts2(
                Some(vec![
                    compressed_account_with_context_1.hash().unwrap(),
                    compressed_account_with_context_2.hash().unwrap(),
                ]),
                Some(vec![
                    compressed_account_with_context_1
                        .merkle_context
                        .merkle_tree_pubkey,
                    compressed_account_with_context_2
                        .merkle_context
                        .merkle_tree_pubkey,
                ]),
                None,
                None,
                &mut context,
            )
            .await;

        let mut proof = None;
        if let Some(proof_rpc) = proof_rpc_result.proof {
            proof = Some(sdk_to_program_compressed_proof(proof_rpc));
        }

        let input_compressed_accounts = vec![
            compressed_account_with_context_1.compressed_account,
            compressed_account_with_context_2.compressed_account,
        ]
        .iter()
        .map(|x| sdk_to_program_compressed_account(x.clone()))
        .collect::<Vec<_>>();

        let merkle_context = vec![
            compressed_account_with_context_1.merkle_context,
            compressed_account_with_context_2.merkle_context,
        ]
        .iter()
        .map(|x| sdk_to_program_merkle_context(x.clone()))
        .collect::<Vec<_>>();
        let output_compressed_accounts = vec![
            CompressedAccount {
                lamports: 0,
                owner: payer_pubkey,
                data: None,
                address: None,
            },
            CompressedAccount {
                lamports: 0,
                owner: payer_pubkey,
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
                merkle_context_1.nullifier_queue_pubkey, // output queue
                merkle_context_2.merkle_tree_pubkey,
            ],
            &proof_rpc_result.root_indices,
            &Vec::new(),
            proof,
            None,
            false,
            None,
            true,
        );
        println!("Combined Transaction with index and zkp -------------------------");

        let event = context
            .create_and_send_transaction_with_event(&[instruction], &payer_pubkey, &[&payer], None)
            .await
            .unwrap()
            .unwrap();
        let slot = context.get_slot().await.unwrap();
        test_indexer.add_event_and_compressed_accounts(slot, &event.0);
    }
    create_compressed_accounts_in_batch_merkle_tree(
        &mut context,
        &mut test_indexer,
        &payer,
        output_queue_pubkey,
        &env,
    )
    .await
    .unwrap();

    // 8. spend account by zkp -> double spend by index
    {
        // Selecting compressed account:
        // - from the end of the array (accounts at the end are in the Merkle tree (onyl 10 are inserted))
        // - Compressed account in the batched Merkle tree
        let compressed_account_with_context_1 = test_indexer
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner == payer_pubkey
                    && x.merkle_context.nullifier_queue_pubkey == output_queue_pubkey
            })
            .last()
            .unwrap()
            .clone();
        let result = double_spend_compressed_account(
            &mut context,
            &mut test_indexer,
            &payer,
            TestMode::ByZkpThenIndex,
            sdk_to_program_compressed_account_with_merkle_context(
                compressed_account_with_context_1.clone(),
            ),
        )
        .await;
        assert_rpc_error(
            result,
            1,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }

    // 9. spend account by index -> double spend by zkp
    {
        // Selecting compressed account:
        // - from the end of the array (accounts at the end are in the Merkle tree (onyl 10 are inserted))
        // - Compressed account in the batched Merkle tree
        let compressed_account_with_context_1 = test_indexer
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner == payer_pubkey
                    && x.merkle_context.nullifier_queue_pubkey == output_queue_pubkey
            })
            .last()
            .unwrap()
            .clone();
        let result = double_spend_compressed_account(
            &mut context,
            &mut test_indexer,
            &payer,
            TestMode::ByIndexThenZkp,
            sdk_to_program_compressed_account_with_merkle_context(
                compressed_account_with_context_1.clone(),
            ),
        )
        .await;
        assert_rpc_error(
            result,
            1,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    // 10. spend account by index -> double spend by index
    {
        // Selecting compressed account:
        // - from the end of the array (accounts at the end are in the Merkle tree (onyl 10 are inserted))
        // - Compressed account in the batched Merkle tree
        let compressed_account_with_context_1 = test_indexer
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner == payer_pubkey
                    && x.merkle_context.nullifier_queue_pubkey == output_queue_pubkey
            })
            .last()
            .unwrap()
            .clone();
        let result = double_spend_compressed_account(
            &mut context,
            &mut test_indexer,
            &payer,
            TestMode::ByIndexThenIndex,
            sdk_to_program_compressed_account_with_merkle_context(
                compressed_account_with_context_1.clone(),
            ),
        )
        .await;
        assert_rpc_error(
            result,
            1,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    // 11. spend account by zkp -> double spend by zkp
    {
        // Selecting compressed account:
        // - from the end of the array (accounts at the end are in the Merkle tree (onyl 10 are inserted))
        // - Compressed account in the batched Merkle tree
        let compressed_account_with_context_1 = test_indexer
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner == payer_pubkey
                    && x.merkle_context.nullifier_queue_pubkey == output_queue_pubkey
            })
            .last()
            .unwrap()
            .clone();
        let result = double_spend_compressed_account(
            &mut context,
            &mut test_indexer,
            &payer,
            TestMode::ByZkpThenZkp,
            sdk_to_program_compressed_account_with_merkle_context(
                compressed_account_with_context_1.clone(),
            ),
        )
        .await;
        assert_rpc_error(
            result,
            1,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    // 12. spend account by zkp  but mark as spent by index
    {
        create_output_accounts(
            &mut context,
            &payer,
            &mut test_indexer,
            output_queue_pubkey,
            1,
            true,
        )
        .await
        .unwrap();
        let accounts = test_indexer
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner == payer_pubkey
                    && x.merkle_context.nullifier_queue_pubkey == output_queue_pubkey
            })
            .collect::<Vec<_>>();
        let compressed_account_with_context_1 = accounts[1].clone();
        // overwrite both output queue batches -> all prior values only exist in the Merkle tree not in the output queue
        for _ in 0..2 {
            create_compressed_accounts_in_batch_merkle_tree(
                &mut context,
                &mut test_indexer,
                &payer,
                output_queue_pubkey,
                &env,
            )
            .await
            .unwrap();
        }

        let proof_rpc_result = test_indexer
            .create_proof_for_compressed_accounts2(
                Some(vec![compressed_account_with_context_1.hash().unwrap()]),
                Some(vec![
                    compressed_account_with_context_1
                        .merkle_context
                        .merkle_tree_pubkey,
                ]),
                None,
                None,
                &mut context,
            )
            .await;
        let mut merkle_context =
            sdk_to_program_merkle_context(compressed_account_with_context_1.merkle_context);
        merkle_context.queue_index = Some(QueueIndex::default());
        let mut proof = None;
        if let Some(proof_rpc) = proof_rpc_result.proof {
            proof = Some(sdk_to_program_compressed_proof(proof_rpc));
        }

        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &[sdk_to_program_compressed_account(
                compressed_account_with_context_1.compressed_account,
            )],
            &output_compressed_accounts,
            &[merkle_context],
            &[merkle_context.nullifier_queue_pubkey],
            &[None],
            &Vec::new(),
            proof,
            None,
            false,
            None,
            true,
        );

        let result = context
            .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            BatchedMerkleTreeError::InclusionProofByIndexFailed.into(),
        )
        .unwrap();
    }
    // 13. failing - spend account v1 by zkp  but mark as spent by index
    // v1 accounts cannot be spent by index
    {
        // Selecting compressed account in v1 Merkle tree
        let compressed_account_with_context_1 = test_indexer
            .compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.owner == payer_pubkey
                    && x.merkle_context.nullifier_queue_pubkey != output_queue_pubkey
            })
            .last()
            .unwrap()
            .clone();

        let mut merkle_context = compressed_account_with_context_1.merkle_context;
        merkle_context.queue_index = Some(SdkQueueIndex::default());
        let instruction = create_invoke_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &input_compressed_accounts,
            &output_compressed_accounts,
            &[sdk_to_program_merkle_context(merkle_context)],
            &[merkle_context.merkle_tree_pubkey],
            &[None],
            &Vec::new(),
            None,
            None,
            false,
            None,
            true,
        );

        let result = context
            .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
            .await;
        // Should fail because it tries to deserialize an output queue account from a nullifier queue account
        assert_rpc_error(result, 0, UtilsError::InvalidDiscriminator.into()).unwrap();
    }
}

#[derive(Debug, PartialEq)]
pub enum TestMode {
    ByZkpThenIndex,
    ByIndexThenZkp,
    ByIndexThenIndex,
    ByZkpThenZkp,
}

pub async fn double_spend_compressed_account<
    R: RpcConnection,
    I: Indexer<R> + TestIndexerExtensions<R>,
>(
    context: &mut R,
    test_indexer: &mut I,
    payer: &Keypair,
    mode: TestMode,
    compressed_account_with_context_1: CompressedAccountWithMerkleContext,
) -> Result<(), RpcError> {
    let proof_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(vec![compressed_account_with_context_1.hash().unwrap()]),
            Some(vec![
                compressed_account_with_context_1
                    .merkle_context
                    .merkle_tree_pubkey,
            ]),
            None,
            None,
            context,
        )
        .await;
    let proof = Some(sdk_to_program_compressed_proof(proof_rpc_result.proof));
    let input_compressed_accounts = vec![compressed_account_with_context_1.compressed_account];
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer.pubkey(),
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
        &[merkle_context_1.nullifier_queue_pubkey],
        &proof_rpc_result.root_indices,
        &Vec::new(),
        proof,
        None,
        false,
        None,
        true,
    )];

    {
        let mut merkle_context = merkle_context_1;
        merkle_context.queue_index = Some(QueueIndex {
            queue_id: 1,
            index: 0,
        });
        let instruction = create_invoke_instruction(
            &payer.pubkey(),
            &payer.pubkey(),
            &input_compressed_accounts,
            &output_compressed_accounts,
            &[merkle_context],
            &[merkle_context.nullifier_queue_pubkey],
            &vec![None],
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
    let event = context
        .create_and_send_transaction_with_event(&instructions, &payer.pubkey(), &[&payer], None)
        .await?
        .unwrap();
    let slot: u64 = context.get_slot().await.unwrap();
    test_indexer.add_event_and_compressed_accounts(slot, &event.0);
    Ok(())
}

/// fill batch and perform batch append
pub async fn create_compressed_accounts_in_batch_merkle_tree(
    context: &mut ProgramTestRpcConnection,
    test_indexer: &mut TestIndexer<ProgramTestRpcConnection>,
    payer: &Keypair,
    output_queue_pubkey: Pubkey,
    env: &EnvAccounts,
) -> Result<(), RpcError> {
    let mut output_queue_account = context
        .get_account(output_queue_pubkey)
        .await
        .unwrap()
        .unwrap();
    let output_queue =
        BatchedQueueAccount::output_queue_from_bytes_mut(&mut output_queue_account.data).unwrap();
    let fullness = output_queue.get_batch_num_inserted_in_current_batch();
    let remaining_leaves = output_queue.get_metadata().batch_metadata.batch_size - fullness;
    for _ in 0..remaining_leaves {
        create_output_accounts(context, &payer, test_indexer, output_queue_pubkey, 1, true).await?;
    }
    for i in 0..output_queue
        .get_metadata()
        .batch_metadata
        .get_num_zkp_batches()
    {
        println!("Performing batch append {}", i);
        let bundle = test_indexer
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.nullifier_queue == output_queue_pubkey)
            .unwrap();
        perform_batch_append(context, bundle, &env.forester, 0, false, None).await?;
    }
    Ok(())
}
pub async fn create_output_accounts(
    context: &mut ProgramTestRpcConnection,
    payer: &Keypair,
    test_indexer: &mut TestIndexer<ProgramTestRpcConnection>,
    output_queue_pubkey: Pubkey,
    num_accounts: usize,
    is_batched: bool,
) -> Result<Signature, RpcError> {
    let output_compressed_accounts = vec![
        CompressedAccount {
            lamports: 0,
            owner: payer.pubkey(),
            data: None,
            address: None,
        };
        num_accounts
    ];
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

    let (event, signature, _) = context
        .create_and_send_transaction_with_event(
            &[instruction],
            &payer.pubkey(),
            &[&payer],
            Some(TransactionParams {
                num_input_compressed_accounts: 0,
                num_output_compressed_accounts: num_accounts as u8,
                num_new_addresses: 0,
                compress: 0,
                fee_config,
            }),
        )
        .await
        .unwrap()
        .unwrap();
    let slot: u64 = context.get_slot().await.unwrap();
    let (created_compressed_accounts, _) =
        test_indexer.add_event_and_compressed_accounts(slot, &event);
    let created_compressed_accounts = created_compressed_accounts
        .into_iter()
        .map(sdk_to_program_compressed_account_with_merkle_context)
        .collect::<Vec<_>>();
    assert_created_compressed_accounts(
        output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys.as_slice(),
        created_compressed_accounts.as_slice(),
    );
    Ok(signature)
}
