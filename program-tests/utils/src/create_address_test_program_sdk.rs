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
use light_hasher::{Hasher, Poseidon};
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

#[derive(Debug, Clone)]
pub struct ProoflessShieldedAppendInstructionInputs<'a> {
    pub signer: &'a Pubkey,
    pub output_compressed_account_merkle_tree_pubkey: &'a Pubkey,
    pub registered_program_pda: &'a Pubkey,
    pub args: create_address_test_program::ProoflessShieldedAppendArgs,
}

#[derive(Debug, Clone)]
pub struct ProoflessShieldedSpendInstructionInputs<'a> {
    pub signer: &'a Pubkey,
    pub registered_program_pda: &'a Pubkey,
    pub args: create_address_test_program::ProoflessShieldedSpendArgs,
    pub proof: Option<CompressedProof>,
    pub nullifier_address_params: Vec<NewAddressParams>,
}

#[derive(Debug, Clone)]
pub struct ProoflessShieldedAppendPlaintext {
    pub zone_config_hash: [u8; 32],
    pub operation_commitment: [u8; 32],
    pub owner_hash: [u8; 32],
    pub token_mint: [u8; 32],
    pub spl_amount: u64,
    pub sol_amount: u64,
    pub blinding: [u8; 32],
    pub encrypted_utxo: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProoflessShieldedAppendCommitments {
    pub utxo_hash: [u8; 32],
    pub data_hash: [u8; 32],
    pub encrypted_utxo_hash: [u8; 32],
}

pub const LOCAL_DEV_MASP_PROGRAM_ID: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0x10,
];
pub const LOCAL_DEV_MASP_INPUT_SEED: u64 = 0x1001;
pub const LOCAL_DEV_ZONE_CONFIG_HASH: [u8; 32] = [0x77; 32];
pub const LOCAL_DEV_OPERATION_COMMITMENT: [u8; 32] = [0x66; 32];
pub const LOCAL_DEV_NULLIFIER_TREE: [u8; 32] = [0xCD; 32];
pub const LOCAL_DEV_UTXO_LEAF_INDEX: u64 = 0;
pub const LOCAL_DEV_TOKEN_MINT: [u8; 32] = [0xBB; 32];
pub const LOCAL_DEV_SPL_AMOUNT: u64 = 1_000_000;
pub const LOCAL_DEV_SOL_AMOUNT: u64 = 42;
pub const LOCAL_DEV_BLINDING: [u8; 32] = [0xCC; 32];
pub const LOCAL_DEV_DATA_HASH: [u8; 32] = [
    23, 250, 137, 116, 255, 248, 202, 19, 117, 168, 19, 69, 127, 63, 120, 252, 52, 212, 235, 217,
    34, 186, 148, 18, 11, 14, 103, 173, 44, 236, 230, 182,
];
pub const LOCAL_DEV_UTXO_HASH: [u8; 32] = [
    18, 201, 110, 230, 164, 92, 38, 97, 172, 235, 207, 8, 168, 161, 179, 51, 28, 52, 63, 194, 103,
    151, 206, 208, 244, 144, 246, 250, 33, 155, 4, 22,
];
pub const LOCAL_DEV_ENCRYPTED_UTXO_HASH: [u8; 32] = [
    35, 235, 97, 155, 138, 208, 6, 8, 152, 164, 238, 115, 103, 109, 40, 186, 70, 93, 26, 111, 184,
    189, 241, 92, 17, 145, 202, 78, 85, 61, 124, 210,
];

pub fn local_dev_proofless_shielded_append_plaintext(
    encrypted_utxo: Vec<u8>,
) -> ProoflessShieldedAppendPlaintext {
    ProoflessShieldedAppendPlaintext {
        zone_config_hash: LOCAL_DEV_ZONE_CONFIG_HASH,
        operation_commitment: LOCAL_DEV_OPERATION_COMMITMENT,
        owner_hash: shielded_program_owner_hash(
            LOCAL_DEV_MASP_PROGRAM_ID,
            LOCAL_DEV_MASP_INPUT_SEED,
        ),
        token_mint: LOCAL_DEV_TOKEN_MINT,
        spl_amount: LOCAL_DEV_SPL_AMOUNT,
        sol_amount: LOCAL_DEV_SOL_AMOUNT,
        blinding: LOCAL_DEV_BLINDING,
        encrypted_utxo,
    }
}

pub fn proofless_shielded_append_args_from_plaintext(
    plaintext: ProoflessShieldedAppendPlaintext,
) -> (
    create_address_test_program::ProoflessShieldedAppendArgs,
    ProoflessShieldedAppendCommitments,
) {
    let data_hash = shielded_utxo_data_hash(plaintext.token_mint, plaintext.zone_config_hash);
    let utxo_hash = shielded_utxo_hash(
        plaintext.owner_hash,
        plaintext.spl_amount,
        plaintext.sol_amount,
        plaintext.blinding,
        data_hash,
    );
    let encrypted_utxo_hash =
        Poseidon::hashv(&[&utxo_hash]).expect("encrypted UTXO hash fixture should hash");
    (
        create_address_test_program::ProoflessShieldedAppendArgs {
            zone_config_hash: plaintext.zone_config_hash,
            operation_commitment: plaintext.operation_commitment,
            utxo_hash,
            encrypted_utxo: plaintext.encrypted_utxo,
            encrypted_utxo_hash,
        },
        ProoflessShieldedAppendCommitments {
            utxo_hash,
            data_hash,
            encrypted_utxo_hash,
        },
    )
}

pub fn shielded_utxo_data_hash(token_mint: [u8; 32], zone_config_hash: [u8; 32]) -> [u8; 32] {
    Poseidon::hashv(&[&field_bytes(token_mint), &field_bytes(zone_config_hash)])
        .expect("shielded UTXO data hash fixture should hash")
}

pub fn shielded_utxo_hash(
    owner_hash: [u8; 32],
    spl_amount: u64,
    sol_amount: u64,
    blinding: [u8; 32],
    data_hash: [u8; 32],
) -> [u8; 32] {
    Poseidon::hashv(&[
        &u64_to_be_32(0),
        &owner_hash,
        &u64_to_be_32(spl_amount),
        &u64_to_be_32(sol_amount),
        &field_bytes(blinding),
        &data_hash,
    ])
    .expect("shielded UTXO hash fixture should hash")
}

pub fn shielded_program_owner_hash(program_id: [u8; 32], seed: u64) -> [u8; 32] {
    Poseidon::hashv(&[&program_id, &u64_to_be_32(seed)])
        .expect("shielded program owner hash fixture should hash")
}

pub fn shielded_spend_nullifier(
    utxo_hash: [u8; 32],
    leaf_index: u64,
    program_id: [u8; 32],
) -> [u8; 32] {
    let domain_dns = Poseidon::hashv(&[&utxo_hash, &program_id])
        .expect("shielded nullifier domain fixture should hash");
    Poseidon::hashv(&[&utxo_hash, &u64_to_be_32(leaf_index), &domain_dns])
        .expect("shielded spend nullifier fixture should hash")
}

pub fn shielded_spend_nullifier_address(
    spend_nullifier: [u8; 32],
    nullifier_tree: [u8; 32],
    shielded_pool_program_id: [u8; 32],
) -> [u8; 32] {
    derive_address(&spend_nullifier, &nullifier_tree, &shielded_pool_program_id)
}

pub fn shielded_nullifier_chain(nullifiers: &[[u8; 32]]) -> Option<[u8; 32]> {
    let mut iter = nullifiers.iter().rev();
    let mut hash = *iter.next()?;
    for nullifier in iter {
        hash = Poseidon::hashv(&[nullifier, &hash])
            .expect("shielded nullifier hashchain fixture should hash");
    }
    Some(hash)
}

pub fn local_dev_proofless_shielded_spend_args(
    utxo_hash: [u8; 32],
    leaf_index: u64,
) -> create_address_test_program::ProoflessShieldedSpendArgs {
    let nullifier = shielded_spend_nullifier(utxo_hash, leaf_index, LOCAL_DEV_MASP_PROGRAM_ID);
    let nullifiers = vec![nullifier];
    create_address_test_program::ProoflessShieldedSpendArgs {
        zone_config_hash: LOCAL_DEV_ZONE_CONFIG_HASH,
        operation_commitment: LOCAL_DEV_OPERATION_COMMITMENT,
        nullifier_tree: LOCAL_DEV_NULLIFIER_TREE,
        nullifier_chain: shielded_nullifier_chain(&nullifiers),
        nullifiers,
        utxo_public_inputs_hash: None,
        tree_public_inputs_hash: None,
        public_input_hash: None,
    }
}

pub fn proofless_shielded_append_instruction(
    input_params: ProoflessShieldedAppendInstructionInputs,
) -> Instruction {
    let (cpi_signer, bump) = Pubkey::find_program_address(
        &[CPI_AUTHORITY_PDA_SEED],
        &create_address_test_program::id(),
    );
    let mut remaining_accounts = HashMap::<light_compressed_account::Pubkey, usize>::new();
    remaining_accounts.insert(
        (*input_params.output_compressed_account_merkle_tree_pubkey).into(),
        0,
    );

    let instruction_data = create_address_test_program::instruction::ProoflessShieldedAppend {
        args: input_params.args,
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

pub fn proofless_shielded_spend_instruction(
    input_params: ProoflessShieldedSpendInstructionInputs,
) -> Instruction {
    let (cpi_signer, bump) = Pubkey::find_program_address(
        &[CPI_AUTHORITY_PDA_SEED],
        &create_address_test_program::id(),
    );
    let mut remaining_accounts = HashMap::<light_compressed_account::Pubkey, usize>::new();
    let nullifier_address_params = crate::compressed_account_pack::pack_new_address_params(
        input_params.nullifier_address_params.as_slice(),
        &mut remaining_accounts,
    );

    let instruction_data = create_address_test_program::instruction::ProoflessShieldedSpend {
        args: input_params.args,
        proof: input_params.proof,
        nullifier_address_params,
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

fn field_bytes(mut bytes: [u8; 32]) -> [u8; 32] {
    bytes[0] = 0;
    bytes
}

fn u64_to_be_32(value: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&value.to_be_bytes());
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_dev_shielded_utxo_vector_is_pinned() {
        let (args, commitments) = proofless_shielded_append_args_from_plaintext(
            local_dev_proofless_shielded_append_plaintext(Vec::new()),
        );
        assert_eq!(commitments.data_hash, LOCAL_DEV_DATA_HASH);
        assert_eq!(commitments.utxo_hash, LOCAL_DEV_UTXO_HASH);
        assert_eq!(
            commitments.encrypted_utxo_hash,
            LOCAL_DEV_ENCRYPTED_UTXO_HASH
        );
        assert_eq!(args.zone_config_hash, LOCAL_DEV_ZONE_CONFIG_HASH);
        assert_eq!(args.operation_commitment, LOCAL_DEV_OPERATION_COMMITMENT);
        assert_eq!(args.utxo_hash, LOCAL_DEV_UTXO_HASH);
        assert_eq!(args.encrypted_utxo_hash, LOCAL_DEV_ENCRYPTED_UTXO_HASH);
    }

    #[test]
    fn local_dev_shielded_spend_vector_is_self_consistent() {
        let args =
            local_dev_proofless_shielded_spend_args(LOCAL_DEV_UTXO_HASH, LOCAL_DEV_UTXO_LEAF_INDEX);
        assert_eq!(args.zone_config_hash, LOCAL_DEV_ZONE_CONFIG_HASH);
        assert_eq!(args.operation_commitment, LOCAL_DEV_OPERATION_COMMITMENT);
        assert_eq!(args.nullifier_tree, LOCAL_DEV_NULLIFIER_TREE);
        assert_eq!(args.nullifiers.len(), 1);
        assert_eq!(args.nullifier_chain, Some(args.nullifiers[0]));
        let nullifier_address = shielded_spend_nullifier_address(
            args.nullifiers[0],
            LOCAL_DEV_NULLIFIER_TREE,
            LOCAL_DEV_MASP_PROGRAM_ID,
        );
        assert_ne!(nullifier_address, args.nullifiers[0]);
    }
}
