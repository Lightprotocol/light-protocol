#![cfg(feature = "test-sbf")]
use account_compression::errors::AccountCompressionErrorCode;
use anchor_lang::error::ErrorCode;
use light_hasher::Poseidon;
use light_system_program::{
    errors::CompressedPdaError,
    sdk::{
        address::derive_address,
        compressed_account::{CompressedAccount, CompressedAccountData, MerkleContext},
        invoke::{
            create_invoke_instruction, create_invoke_instruction_data_and_remaining_accounts,
        },
    },
    utils::{get_cpi_authority_pda, get_registered_program_pda},
    InstructionDataInvoke, NewAddressParams,
};
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::rpc::{errors::assert_rpc_error, rpc_connection::RpcConnection};
use light_test_utils::transaction_params::{FeeConfig, TransactionParams};
use light_test_utils::{
    assert_compressed_tx::assert_created_compressed_accounts,
    assert_custom_error_or_program_error,
    system_program::{
        compress_sol_test, create_addresses_test, decompress_sol_test, transfer_compressed_sol_test,
    },
    test_env::setup_test_programs_with_accounts,
    test_indexer::TestIndexer,
};
use light_test_utils::{rpc::errors::RpcError, test_env::EnvAccounts};
use light_utils::hash_to_bn254_field_size_be;
use light_verifier::VerifierError;
use solana_cli_output::CliAccount;
use solana_sdk::{
    instruction::{AccountMeta, Instruction, InstructionError},
    pubkey::Pubkey,
    signer::Signer,
    transaction::Transaction,
};
use solana_sdk::{signature::Keypair, transaction::TransactionError};
use tokio::fs::write as async_write;

// TODO: use lazy_static to spawn the server once

// invoke_failing_test
// - inputs, outputs, new addresses, (fail with every input)
#[tokio::test]
async fn invoke_failing_test() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;

    let payer = context.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    for i in 1..5 {
        failing_transaction_output(&mut context, &payer, &env, i)
            .await
            .unwrap();
    }
    for i in 1..=2 {
        failing_transaction_address(&mut context, &mut test_indexer, &payer, &env, i)
            .await
            .unwrap();
    }
    let options = [1usize, 2usize, 3usize, 4usize, 8usize];
    for i in 0..5 {
        failing_transaction_inputs(&mut context, &mut test_indexer, &payer, &env, options[i])
            .await
            .unwrap();
    }
}

pub async fn failing_transaction_inputs(
    context: &mut ProgramTestRpcConnection,
    test_indexer: &mut TestIndexer<200, ProgramTestRpcConnection>,
    payer: &Keypair,
    env: &EnvAccounts,
    num_inputs: usize,
) -> Result<(), RpcError> {
    println!("num_inputs: {:?}", num_inputs);
    // create compressed accounts that can be used as inputs
    for _ in 0..num_inputs {
        compress_sol_test(
            context,
            test_indexer,
            payer,
            &[],
            false,
            1_000_000,
            &env.merkle_tree_pubkey,
            None,
        )
        .await
        .unwrap();
    }
    let input_compressed_accounts =
        test_indexer.get_compressed_accounts_by_owner(&payer.pubkey())[0..num_inputs].to_vec();
    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.hash().unwrap())
        .collect::<Vec<_>>();
    let input_state_merkle_trees = input_compressed_accounts
        .iter()
        .map(|x| x.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<_>>();
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            Some(input_compressed_account_hashes.as_slice()),
            Some(input_state_merkle_trees.as_slice()),
            None,
            None,
            context,
        )
        .await;
    let output_compressed_account = CompressedAccount {
        lamports: input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.lamports)
            .sum(),
        owner: payer.pubkey(),
        data: None,
        address: None,
    };
    let (remaining_accounts, inputs_struct) = create_invoke_instruction_data_and_remaining_accounts(
        &[],
        &input_compressed_accounts
            .iter()
            .map(|x| x.merkle_context.clone())
            .collect::<Vec<_>>(),
        &input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>(),
        &proof_rpc_res.root_indices,
        &[env.merkle_tree_pubkey],
        &[output_compressed_account], // no need for output compressed accounts the tx will fail before these are used
        Some(proof_rpc_res.proof),
        None,
        false,
    );
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
    // invalid leaf index
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
            .merkle_context
            .leaf_index = inputs_struct.input_compressed_accounts_with_merkle_context
            [num_inputs - 1]
            .merkle_context
            .leaf_index
            + 1;
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
    // invalid account data (lamports)
    {
        let mut inputs_struct = inputs_struct.clone();
        inputs_struct.input_compressed_accounts_with_merkle_context[num_inputs - 1]
            .compressed_account
            .lamports = 1;
        // adapting compressed ouput account so that sumcheck passes
        inputs_struct.output_compressed_accounts[0]
            .compressed_account
            .lamports = inputs_struct.output_compressed_accounts[0]
            .compressed_account
            .lamports
            - 999_999;
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
            .owner = Pubkey::new_unique();
        create_instruction_and_failing_transaction(
            context,
            payer,
            inputs_struct,
            remaining_accounts.clone(),
            CompressedPdaError::SignerCheckFailed.into(),
        )
        .await
        .unwrap();
    }
    // invalid account data (owner)
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
            VerifierError::ProofVerificationFailed.into(),
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
            ErrorCode::AccountDiscriminatorMismatch.into(),
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
            AccountCompressionErrorCode::InvalidQueueType.into(),
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
    Ok(())
}

pub async fn failing_transaction_address(
    context: &mut ProgramTestRpcConnection,
    test_indexer: &mut TestIndexer<200, ProgramTestRpcConnection>,
    payer: &Keypair,
    env: &EnvAccounts,
    num_addresses: usize,
) -> Result<(), RpcError> {
    if num_addresses > 2 {
        panic!("num_output_compressed_accounts should be less than 8");
    }

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
            derive_address(&env.address_merkle_tree_pubkey, address_seed).unwrap();
        derived_addresses.push(derived_address);
    }
    let proof_rpc_res = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(derived_addresses.as_slice()),
            Some(vec![env.address_merkle_tree_pubkey; num_addresses]),
            context,
        )
        .await;
    for (i, root_index) in proof_rpc_res.address_root_indices.iter().enumerate() {
        new_address_params[i as usize].address_merkle_tree_root_index = *root_index;
    }
    let (remaining_accounts, inputs_struct) = create_invoke_instruction_data_and_remaining_accounts(
        &new_address_params,
        &[],
        &[],
        &[],
        &[],
        &[],
        Some(proof_rpc_res.proof),
        None,
        false,
    );
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
            AccountCompressionErrorCode::InvalidQueueType.into(),
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
            ErrorCode::AccountDiscriminatorMismatch.into(),
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
pub async fn failing_transaction_output(
    context: &mut ProgramTestRpcConnection,
    payer: &Keypair,
    env: &EnvAccounts,
    num_output_compressed_accounts: usize,
    // address: Option<[u8; 32]>,
    // proof: Option<ProofCompressed>,
) -> Result<(), RpcError> {
    if num_output_compressed_accounts > 8 {
        panic!("num_output_compressed_accounts should be less than 8");
    }
    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    // invalid lamports
    {
        let mut output_compressed_accounts = vec![];
        let mut output_merkle_tree_pubkeys = vec![];

        for i in 0..num_output_compressed_accounts {
            output_compressed_accounts.push(CompressedAccount {
                lamports: (i + 1) as u64,
                owner: payer_pubkey,
                data: None,
                address: None,
            });
            output_merkle_tree_pubkeys.push(merkle_tree_pubkey);
        }
        perform_tx_with_output_compressed_accounts(
            context,
            payer,
            payer_pubkey,
            output_compressed_accounts,
            output_merkle_tree_pubkeys,
            CompressedPdaError::ComputeOutputSumFailed.into(),
        )
        .await?;
    }
    // Data but signer is not a program
    {
        let mut output_compressed_accounts = vec![];
        let mut output_merkle_tree_pubkeys = vec![];

        for i in 0..num_output_compressed_accounts {
            let data = CompressedAccountData {
                discriminator: [i as u8; 8],
                data: vec![i as u8; i],
                data_hash: [i as u8; 32],
            };
            output_compressed_accounts.push(CompressedAccount {
                lamports: 0,
                owner: payer_pubkey,
                data: Some(data),
                address: None,
            });
            output_merkle_tree_pubkeys.push(merkle_tree_pubkey);
        }
        perform_tx_with_output_compressed_accounts(
            context,
            payer,
            payer_pubkey,
            output_compressed_accounts.clone(),
            output_merkle_tree_pubkeys.clone(),
            CompressedPdaError::InvokingProgramNotProvided.into(),
        )
        .await?;
        // only one account has data
        output_compressed_accounts
            .iter_mut()
            .enumerate()
            .for_each(|(i, compressed_account)| {
                if i != num_output_compressed_accounts - 1 {
                    compressed_account.data = None;
                }
            });
        perform_tx_with_output_compressed_accounts(
            context,
            payer,
            payer_pubkey,
            output_compressed_accounts,
            output_merkle_tree_pubkeys,
            CompressedPdaError::InvokingProgramNotProvided.into(),
        )
        .await?;
    }
    // Invalid output Merkle tree
    {
        let mut output_compressed_accounts = vec![];
        let mut output_merkle_tree_pubkeys = vec![];
        for _ in 0..num_output_compressed_accounts {
            output_compressed_accounts.push(CompressedAccount {
                lamports: 0,
                owner: payer_pubkey,
                data: None,
                address: None,
            });
            output_merkle_tree_pubkeys.push(merkle_tree_pubkey);
        }
        output_merkle_tree_pubkeys[num_output_compressed_accounts - 1] =
            env.address_merkle_tree_pubkey;
        perform_tx_with_output_compressed_accounts(
            context,
            payer,
            payer_pubkey,
            output_compressed_accounts,
            output_merkle_tree_pubkeys,
            ErrorCode::AccountDiscriminatorMismatch.into(),
        )
        .await?;
    }

    // Address that doesn't exist
    {
        let mut output_compressed_accounts = vec![];
        let mut output_merkle_tree_pubkeys = vec![];
        for i in 0..num_output_compressed_accounts {
            let address = if i == num_output_compressed_accounts - 1 {
                Some(
                    hash_to_bn254_field_size_be(Pubkey::new_unique().to_bytes().as_slice())
                        .unwrap()
                        .0,
                )
            } else {
                None
            };
            output_compressed_accounts.push(CompressedAccount {
                lamports: 0,
                owner: payer_pubkey,
                data: None,
                address,
            });
            output_merkle_tree_pubkeys.push(merkle_tree_pubkey);
        }

        perform_tx_with_output_compressed_accounts(
            context,
            payer,
            payer_pubkey,
            output_compressed_accounts.clone(),
            output_merkle_tree_pubkeys.clone(),
            CompressedPdaError::InvalidAddress.into(),
        )
        .await?;
        if num_output_compressed_accounts >= 2 {
            let address = Some(
                hash_to_bn254_field_size_be(Pubkey::new_unique().to_bytes().as_slice())
                    .unwrap()
                    .0,
            );
            output_compressed_accounts[num_output_compressed_accounts - 2].address = address;
        }
        perform_tx_with_output_compressed_accounts(
            context,
            payer,
            payer_pubkey,
            output_compressed_accounts,
            output_merkle_tree_pubkeys,
            CompressedPdaError::InvalidAddress.into(),
        )
        .await?;
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
    );
    let result = context
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
        .await
        .unwrap_err();
    assert_rpc_error(Err(result), 0, expected_error_code)
}

use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
pub async fn create_instruction_and_failing_transaction(
    context: &mut ProgramTestRpcConnection,
    payer: &Keypair,
    inputs_struct: InstructionDataInvoke,
    remaining_accounts: Vec<AccountMeta>,
    expected_error_code: u32,
) -> Result<(), RpcError> {
    let mut inputs = Vec::new();

    InstructionDataInvoke::serialize(&inputs_struct, &mut inputs).unwrap();

    let instruction_data = light_system_program::instruction::Invoke { inputs };

    let compressed_sol_pda = None;

    let accounts = light_system_program::accounts::InvokeInstruction {
        fee_payer: payer.pubkey(),
        authority: payer.pubkey(),
        registered_program_pda: get_registered_program_pda(&light_system_program::ID),
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        account_compression_program: account_compression::ID,
        account_compression_authority: get_cpi_authority_pda(&light_system_program::ID),
        compressed_sol_pda,
        compression_recipient: None,
        system_program: solana_sdk::system_program::ID,
    };
    let instruction = Instruction {
        program_id: light_system_program::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };
    let result = context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap_err();
    println!("result: {:?}", result);
    assert_rpc_error(Err(result), 0, expected_error_code)
}

/// Tests Execute compressed transaction:
/// 1. should succeed: without compressed account(0 lamports), no in compressed account
/// 2. should fail: in compressed account and invalid zkp
/// 3. should fail: in compressed account and invalid signer
/// 4. should succeed: in compressed account inserted in (1.) and valid zkp
#[tokio::test]
async fn invoke_test() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;

    let payer = context.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
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
    let (created_compressed_accounts, _) = test_indexer.add_event_and_compressed_accounts(&event);
    assert_created_compressed_accounts(
        output_compressed_accounts.as_slice(),
        output_merkle_tree_pubkeys.as_slice(),
        created_compressed_accounts.as_slice(),
        false,
    );

    let input_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: payer_pubkey,
        data: None,
        address: None,
    }];
    // TODO: assert all compressed account properties
    // check invalid proof
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &[0u16],
        &Vec::new(),
        None,
        None,
        false,
        None,
    );

    let res = context
        .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
        .await;
    assert!(res.is_err());

    // check invalid signer for in compressed_account
    let invalid_signer_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Pubkey::new_unique(),
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
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &[0u16],
        &Vec::new(),
        None,
        None,
        false,
        None,
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
            Some(&[compressed_account_with_context
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.merkle_context.leaf_index,
                )
                .unwrap()]),
            Some(&[compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey]),
            None,
            None,
            &mut context,
        )
        .await;
    let input_compressed_accounts = vec![compressed_account_with_context.compressed_account];
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        None,
        false,
        None,
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
    test_indexer.add_event_and_compressed_accounts(&event);

    println!("Double spend -------------------------");
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Pubkey::new_unique(),
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
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        None,
        false,
        None,
    );
    let res = context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(res.is_err());
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: Pubkey::new_unique(),
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
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        None,
        false,
        None,
    );
    let res = context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;
    assert!(res.is_err());
}

/// Tests Execute compressed transaction with address:
/// 1. should fail: create out compressed account with address without input compressed account with address or created address
/// 2. should succeed: create out compressed account with new created address
/// 3. should fail: create two addresses with the same seeds
/// 4. should succeed: create two addresses with different seeds
/// 5. should succeed: create multiple addresses with different seeds and spend input compressed accounts
///    testing: (input accounts, new addresses) (1, 1), (1, 2), (2, 1), (2, 2)
#[tokio::test]
async fn test_with_address() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;

    let address_seed = [1u8; 32];
    let derived_address = derive_address(&env.address_merkle_tree_pubkey, &address_seed).unwrap();
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
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_latest_blockhash().await.unwrap(),
    );

    let res = context.process_transaction(transaction).await;
    assert_custom_error_or_program_error(res, CompressedPdaError::InvalidAddress.into()).unwrap();
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

    let compressed_account_with_context = test_indexer.compressed_accounts[0].clone();
    let recipient_pubkey = Pubkey::new_unique();
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
            .get_compressed_accounts_by_owner(&payer_pubkey)[0..n_input_compressed_accounts]
            .to_vec();
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

#[tokio::test]
async fn test_with_compression() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.get_payer().insecure_clone();

    let payer_pubkey = payer.pubkey();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
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
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_latest_blockhash().await.unwrap(),
    );

    let result = context.process_transaction(transaction).await;
    // should fail because of insufficient input funds
    assert_custom_error_or_program_error(result, CompressedPdaError::ComputeOutputSumFailed.into())
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
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_latest_blockhash().await.unwrap(),
    );

    let result = context.process_transaction(transaction).await;
    // should fail because of insufficient decompress amount funds
    assert_custom_error_or_program_error(result, CompressedPdaError::ComputeOutputSumFailed.into())
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
            Some(&[compressed_account_with_context
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &compressed_account_with_context.merkle_context.leaf_index,
                )
                .unwrap()]),
            Some(&[compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey]),
            None,
            None,
            &mut context,
        )
        .await;
    let input_compressed_accounts =
        vec![compressed_account_with_context.clone().compressed_account];
    let recipient_pubkey = Pubkey::new_unique();
    let output_compressed_accounts = vec![CompressedAccount {
        lamports: 0,
        owner: recipient_pubkey,
        data: None,
        address: None,
    }];
    let recipient = Pubkey::new_unique();
    let instruction = create_invoke_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &input_compressed_accounts,
        &output_compressed_accounts,
        &[MerkleContext {
            merkle_tree_pubkey,
            leaf_index: 0,
            nullifier_queue_pubkey,
        }],
        &[merkle_tree_pubkey],
        &proof_rpc_res.root_indices,
        &Vec::new(),
        Some(proof_rpc_res.proof.clone()),
        Some(compress_amount),
        true,
        Some(recipient),
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
    assert_custom_error_or_program_error(result, CompressedPdaError::SumCheckFailed.into())
        .unwrap();

    let compressed_account_with_context =
        test_indexer.get_compressed_accounts_by_owner(&payer_pubkey)[0].clone();
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
#[tokio::test]
async fn regenerate_accounts() {
    let output_dir = "../../cli/accounts/";
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
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
    ];

    for (name, pubkey) in pubkeys {
        // Fetch account data. Adjust this part to match how you retrieve and structure your account data.
        let account = context.get_account(pubkey).await.unwrap();
        println!(
            "{} DISCRIMINATOR {:?}",
            name,
            account.as_ref().unwrap().data[0..8].to_vec()
        );
        let account = CliAccount::new(&pubkey, &account.unwrap(), true);

        // Serialize the account data to JSON. Adjust according to your data structure.
        let json_data = serde_json::to_vec(&account).unwrap();

        // Construct the output file path
        let file_name = format!("{}_{}.json", name, pubkey);
        let file_path = format!("{}{}", output_dir, file_name);
        println!("Writing account data to {}", file_path);

        // Write the JSON data to a file in the specified directory
        async_write(file_path.clone(), json_data).await.unwrap();
    }
}
