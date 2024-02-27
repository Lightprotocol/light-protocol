#![cfg(feature = "test-sbf")]

use light_test_utils::{create_and_send_transaction, test_env::setup_test_programs_with_accounts};
use psp_compressed_pda::{
    sdk::{
        create_execute_compressed_instruction, create_execute_compressed_opt_instruction,
        CompressedProof,
    },
    utxo::{OutUtxo, Utxo},
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};

#[tokio::test]
async fn test_execute_compressed_transactio() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let in_utxos = vec![Utxo {
        lamports: 0,
        owner: payer_pubkey,
        blinding: [1u8; 32],
        data: None,
    }];

    let out_utxos = vec![OutUtxo {
        lamports: 0,
        owner: payer_pubkey,
        data: None,
    }];
    let proof_mock = CompressedProof {
        proof_a: [0u8; 32],
        proof_b: [0u8; 64],
        proof_c: [0u8; 32],
    };

    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &in_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &vec![0u16],
        &proof_mock,
    );

    create_and_send_transaction(&mut context, &[instruction], &payer)
        .await
        .unwrap();
    let invalid_signer_utxos = vec![Utxo {
        lamports: 0,
        owner: Pubkey::new_unique(),
        blinding: [1u8; 32],
        data: None,
    }];
    let instruction = create_execute_compressed_instruction(
        &payer_pubkey,
        &invalid_signer_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &vec![0u16],
        &proof_mock,
    );
    let res = create_and_send_transaction(&mut context, &[instruction], &payer).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_create_execute_compressed_transaction_2() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let mut in_utxo = Utxo {
        lamports: 0,
        owner: payer_pubkey,
        blinding: [0u8; 32],
        data: None,
    };
    in_utxo.update_blinding(merkle_tree_pubkey, 0).unwrap();

    let in_utxos = vec![in_utxo];

    let out_utxos = vec![OutUtxo {
        lamports: 0,
        owner: payer_pubkey,
        data: None,
    }];
    let proof_mock = CompressedProof {
        proof_a: [0u8; 32],
        proof_b: [0u8; 64],
        proof_c: [0u8; 32],
    };

    let instruction = create_execute_compressed_opt_instruction(
        &payer_pubkey,
        &in_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &vec![0u32],
        &vec![0u16],
        &proof_mock,
    );

    create_and_send_transaction(&mut context, &[instruction], &payer)
        .await
        .unwrap();
    let invalid_signer_utxos = vec![Utxo {
        lamports: 0,
        owner: Pubkey::new_unique(),
        blinding: [1u8; 32],
        data: None,
    }];
    let instruction = create_execute_compressed_opt_instruction(
        &payer_pubkey,
        &invalid_signer_utxos,
        &out_utxos,
        &vec![merkle_tree_pubkey],
        &vec![indexed_array_pubkey],
        &vec![merkle_tree_pubkey],
        &vec![0u32],
        &vec![0u16],
        &proof_mock,
    );
    let res = create_and_send_transaction(&mut context, &[instruction], &payer).await;
    assert!(res.is_err());
}
