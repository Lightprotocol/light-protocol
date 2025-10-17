use std::collections::HashMap;

use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::{
    indexer::{AddressWithTree, Indexer},
    rpc::{Rpc, RpcError},
};
use light_compressed_account::{
    address::derive_address,
    instruction_data::{compressed_proof::CompressedProof, data::NewAddressParams},
};
use light_program_test::{accounts::test_accounts::TestAccounts, indexer::TestIndexerExtensions};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer};

use crate::e2e_test_env::to_account_metas_light;

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
    let mut remaining_accounts = HashMap::<light_compressed_account::Pubkey, usize>::new();
    remaining_accounts.insert(
        (*input_params.output_compressed_account_merkle_tree_pubkey).into(),
        0,
    );
    let new_address_params = crate::compressed_account_pack::pack_new_address_params(
        &[input_params.new_address_params],
        &mut remaining_accounts,
    );

    let instruction_data = create_address_test_program::instruction::CreateCompressedPda {
        data: input_params.data,
        proof: Some(*input_params.proof),
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
    let remaining_accounts = to_account_metas_light(remaining_accounts);

    Instruction {
        program_id: create_address_test_program::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}

pub async fn perform_create_pda_with_event_rnd<
    R: Rpc + light_program_test::program_test::TestRpc + Indexer,
    I: Indexer + TestIndexerExtensions,
>(
    test_indexer: &mut I,
    rpc: &mut R,
    env: &TestAccounts,
    payer: &Keypair,
) -> Result<(), RpcError> {
    let seed = rand::random();
    let data = rand::random();
    perform_create_pda_with_event(test_indexer, rpc, env, payer, seed, &data).await
}

pub async fn perform_create_pda_with_event<
    R: Rpc + light_program_test::program_test::TestRpc + Indexer,
    I: Indexer + TestIndexerExtensions,
>(
    test_indexer: &mut I,
    rpc: &mut R,
    env: &TestAccounts,
    payer: &Keypair,
    seed: [u8; 32],
    data: &[u8; 31],
) -> Result<(), RpcError> {
    let address_with_tree = {
        let address = derive_address(
            &seed,
            &env.v2_address_trees[0].to_bytes(),
            &create_address_test_program::ID.to_bytes(),
        );
        println!("address: {:?}", address);
        println!("address_merkle_tree_pubkey: {:?}", env.v2_address_trees[0]);
        println!("program_id: {:?}", create_address_test_program::ID);
        println!("seed: {:?}", seed);
        AddressWithTree {
            address,
            tree: env.v2_address_trees[0],
        }
    };

    let rpc_result = test_indexer
        .get_validity_proof(Vec::new(), vec![address_with_tree], None)
        .await
        .unwrap();

    let new_address_params = NewAddressParams {
        seed,
        address_merkle_tree_pubkey: env.v2_address_trees[0].into(),
        address_queue_pubkey: env.v2_address_trees[0].into(),
        address_merkle_tree_root_index: rpc_result.value.addresses[0].root_index,
    };
    let create_ix_inputs = CreateCompressedPdaInstructionInputs {
        data: *data,
        signer: &payer.pubkey(),
        output_compressed_account_merkle_tree_pubkey: &env.v2_state_trees[0].output_queue,
        proof: &rpc_result.value.proof.0.unwrap(),
        new_address_params,
        registered_program_pda: &env.protocol.registered_program_pda,
    };
    let instruction = create_pda_instruction(create_ix_inputs);
    let pre_test_indexer_queue_len = test_indexer
        .get_address_merkle_tree(env.v2_address_trees[0])
        .unwrap()
        .queue_elements
        .len();
    let event =
        light_program_test::program_test::TestRpc::create_and_send_transaction_with_public_event(
            rpc,
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await?
        .unwrap();
    let slot: u64 = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.0);
    assert_eq!(
        test_indexer
            .get_address_merkle_tree(env.v2_address_trees[0])
            .unwrap()
            .queue_elements
            .len(),
        pre_test_indexer_queue_len + 1
    );
    Ok(())
}
