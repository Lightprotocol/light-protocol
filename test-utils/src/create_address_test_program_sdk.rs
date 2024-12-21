use std::collections::HashMap;

use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::rpc::{RpcConnection, RpcError};
use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
use light_program_test::test_env::EnvAccounts;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::address::{derive_address, pack_new_address_params},
    NewAddressParams,
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer};

use crate::{indexer::TestIndexer, Indexer};

#[derive(Debug, Clone)]
pub struct CreateCompressedPdaInstructionInputs<'a> {
    pub data: [u8; 31],
    pub signer: &'a Pubkey,
    pub output_compressed_account_merkle_tree_pubkey: &'a Pubkey,
    pub proof: &'a CompressedProof,
    pub new_address_params: NewAddressParams,
    pub registered_program_pda: &'a Pubkey,
}

pub fn create_pda_instruction(input_params: CreateCompressedPdaInstructionInputs) -> Instruction {
    let (cpi_signer, bump) = Pubkey::find_program_address(
        &[CPI_AUTHORITY_PDA_SEED],
        &create_address_test_program::id(),
    );
    let mut remaining_accounts = HashMap::new();
    remaining_accounts.insert(
        *input_params.output_compressed_account_merkle_tree_pubkey,
        0,
    );
    let new_address_params =
        pack_new_address_params(&[input_params.new_address_params], &mut remaining_accounts);

    let instruction_data = create_address_test_program::instruction::CreateCompressedPda {
        data: input_params.data,
        proof: Some(input_params.proof.clone()),
        new_address_parameters: new_address_params[0],
        bump,
    };

    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);

    let accounts = create_address_test_program::accounts::CreateCompressedPda {
        signer: *input_params.signer,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda: *input_params.registered_program_pda,
        account_compression_authority,
        self_program: create_address_test_program::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

    Instruction {
        program_id: create_address_test_program::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}

pub async fn perform_create_pda_with_event_rnd<R: RpcConnection>(
    test_indexer: &mut TestIndexer<R>,
    rpc: &mut R,
    env: &EnvAccounts,
    payer: &Keypair,
) -> Result<(), RpcError> {
    let seed = rand::random();
    let data = rand::random();
    perform_create_pda_with_event(test_indexer, rpc, env, payer, seed, &data).await
}
pub async fn perform_create_pda_with_event<R: RpcConnection>(
    test_indexer: &mut TestIndexer<R>,
    rpc: &mut R,
    env: &EnvAccounts,
    payer: &Keypair,
    seed: [u8; 32],
    data: &[u8; 31],
) -> Result<(), RpcError> {
    let (address, address_merkle_tree_pubkey, address_queue_pubkey) = {
        let address = derive_address(
            &seed,
            &env.batch_address_merkle_tree.to_bytes(),
            &create_address_test_program::ID.to_bytes(),
        );
        println!("address: {:?}", address);
        println!(
            "address_merkle_tree_pubkey: {:?}",
            env.address_merkle_tree_pubkey
        );
        println!("program_id: {:?}", create_address_test_program::ID);
        println!("seed: {:?}", seed);
        (
            address,
            env.batch_address_merkle_tree,
            env.batch_address_merkle_tree,
        )
    };

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(&[address]),
            Some(vec![address_merkle_tree_pubkey]),
            rpc,
        )
        .await;

    let new_address_params = NewAddressParams {
        seed,
        address_merkle_tree_pubkey,
        address_queue_pubkey,
        address_merkle_tree_root_index: rpc_result.address_root_indices[0],
    };
    let create_ix_inputs = CreateCompressedPdaInstructionInputs {
        data: *data,
        signer: &payer.pubkey(),
        output_compressed_account_merkle_tree_pubkey: &env.merkle_tree_pubkey,
        proof: &rpc_result.proof,
        new_address_params,

        registered_program_pda: &env.registered_program_pda,
    };
    let instruction = create_pda_instruction(create_ix_inputs);
    let pre_test_indexer_queue_len = test_indexer.address_merkle_trees[1].queue_elements.len();
    let event = rpc
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?
        .unwrap();
    let slot: u64 = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.0);
    assert_eq!(
        test_indexer.address_merkle_trees[1].queue_elements.len(),
        pre_test_indexer_queue_len + 1
    );
    Ok(())
}
