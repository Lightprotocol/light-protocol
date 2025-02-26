#![cfg(feature = "test-sbf")]

use std::collections::HashMap;

use anchor_lang::prelude::borsh::BorshSerialize;
use create_address_test_program::create_invoke_cpi_instruction;
use light_compressed_account::{
    address::pack_new_address_params,
    compressed_account::{
        pack_compressed_accounts, pack_output_compressed_accounts, CompressedAccount,
        CompressedAccountData, CompressedAccountWithMerkleContext,
        PackedCompressedAccountWithMerkleContext,
    },
    event::{BatchPublicTransactionEvent, MerkleTreeSequenceNumber, PublicTransactionEvent},
    instruction_data::{
        data::{
            NewAddressParams, OutputCompressedAccountWithContext,
            OutputCompressedAccountWithPackedContext,
        },
        invoke_cpi::{InstructionDataInvokeCpi, InstructionDataInvokeCpiWithReadOnly},
    },
};
use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
use light_hasher::Poseidon;
use light_program_test::test_env::setup_test_programs_with_accounts;
use light_test_utils::{RpcConnection, RpcError};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// 1 output compressed account, 1 new address
#[tokio::test]
async fn batched_event_functional() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("create_address_test_program"),
        create_address_test_program::ID,
    )]))
    .await;
    let payer = rpc.get_payer().insecure_clone();
    let output_accounts = vec![get_compressed_output_account(
        true,
        env.batched_output_queue,
    )];
    let (events, output_accounts, _) =
        perform_test_transaction(&mut rpc, &payer, vec![], output_accounts, vec![], Some(2))
            .await
            .unwrap()
            .unwrap();
    let expected_batched_event = BatchPublicTransactionEvent {
        event: PublicTransactionEvent {
            input_compressed_account_hashes: Vec::new(),
            output_leaf_indices: vec![0],
            output_compressed_account_hashes: vec![output_accounts[0]
                .compressed_account
                .hash::<Poseidon>(&env.batched_state_merkle_tree, &0u32)
                .unwrap()],
            output_compressed_accounts: output_accounts.to_vec(),
            sequence_numbers: vec![MerkleTreeSequenceNumber {
                pubkey: env.batched_output_queue,
                seq: 0,
            }],
            relay_fee: None,
            message: None,
            is_compress: false,
            compress_or_decompress_lamports: None,
            pubkey_array: vec![env.batched_output_queue],
        },
        address_sequence_numbers: Vec::new(),
        input_sequence_numbers: Vec::new(),
        batch_input_accounts: Vec::new(),
        new_addresses: Vec::new(),
        tx_hash: [0u8; 32],
    };
    assert_eq!(events[0], expected_batched_event);
    let mut expected_event = expected_batched_event;
    expected_event.event.sequence_numbers = vec![MerkleTreeSequenceNumber {
        pubkey: env.batched_output_queue,
        seq: 1,
    }];
    expected_event.event.output_compressed_account_hashes = vec![output_accounts[0]
        .clone()
        .compressed_account
        .hash::<Poseidon>(&env.batched_state_merkle_tree, &1u32)
        .unwrap()];
    expected_event.event.output_leaf_indices = vec![1];
    assert_eq!(events[1], expected_event);
}

fn get_compressed_output_account(
    data: bool,
    merkle_tree: Pubkey,
) -> OutputCompressedAccountWithContext {
    OutputCompressedAccountWithContext {
        compressed_account: CompressedAccount {
            owner: create_address_test_program::ID,
            lamports: 0,
            address: None,
            data: if data {
                Some(CompressedAccountData {
                    data: vec![2u8; 31],
                    discriminator: u64::MAX.to_be_bytes(),
                    data_hash: [3u8; 32],
                })
            } else {
                None
            },
        },
        merkle_tree,
    }
}

async fn perform_test_transaction(
    rpc: &mut light_program_test::test_rpc::ProgramTestRpcConnection,
    payer: &Keypair,
    input_accounts: Vec<CompressedAccountWithMerkleContext>,
    output_accounts: Vec<OutputCompressedAccountWithContext>,
    new_addresses: Vec<NewAddressParams>,
    num_cpis: Option<u8>,
) -> Result<
    Option<(
        Vec<BatchPublicTransactionEvent>,
        Vec<OutputCompressedAccountWithPackedContext>,
        Vec<PackedCompressedAccountWithMerkleContext>,
    )>,
    RpcError,
> {
    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();

    let packed_new_address_params =
        pack_new_address_params(new_addresses.as_slice(), &mut remaining_accounts);

    let packed_inputs = pack_compressed_accounts(
        input_accounts.as_slice(),
        &vec![None; input_accounts.len()],
        &mut remaining_accounts,
    );
    let output_compressed_accounts = pack_output_compressed_accounts(
        output_accounts
            .iter()
            .map(|x| x.compressed_account.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        output_accounts
            .iter()
            .map(|x| x.merkle_tree)
            .collect::<Vec<_>>()
            .as_slice(),
        &mut remaining_accounts,
    );
    let invoke_cpi = InstructionDataInvokeCpi {
        proof: None,
        new_address_params: packed_new_address_params,
        input_compressed_accounts_with_merkle_context: packed_inputs.clone(),
        output_compressed_accounts: output_compressed_accounts.clone(),
        relay_fee: None,
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context: None,
    };
    let ix_data = InstructionDataInvokeCpiWithReadOnly {
        invoke_cpi,
        read_only_accounts: None,
        read_only_addresses: None,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);
    let instruction = create_invoke_cpi_instruction(
        payer.pubkey(),
        ix_data.try_to_vec().unwrap(),
        remaining_accounts,
        num_cpis,
    );
    let res = rpc
        .create_and_send_transaction_with_batched_event(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await?;
    if let Some(res) = res {
        Ok(Some((res.0, output_compressed_accounts, packed_inputs)))
    } else {
        Ok(None)
    }
}
