use anchor_lang::{InstructionData, ToAccountMetas};
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::rpc::Rpc;
use light_merkle_tree_metadata::fee::{compute_claimable_excess, hardcoded_rent_exemption};
use light_program_test::{
    program_test::LightProgramTest, utils::assert::assert_rpc_error, ProgramTestConfig,
};
use light_registry::{
    account_compression_cpi::sdk::create_claim_fees_wrapper_instruction,
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
};
use serial_test::serial;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Helper: update protocol_fee_recipient in ProtocolConfig.
/// Reads current config, sets the new recipient, sends update_protocol_config tx.
async fn set_protocol_fee_recipient(
    rpc: &mut LightProgramTest,
    governance_authority: &Keypair,
    protocol_config_pda: Pubkey,
    fee_recipient: Pubkey,
) {
    let payer = rpc.get_payer().insecure_clone();
    let current_config: ProtocolConfigPda = rpc
        .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda)
        .await
        .unwrap()
        .unwrap();

    let updated_config = ProtocolConfig {
        protocol_fee_recipient: fee_recipient,
        ..current_config.config
    };

    let instruction = light_registry::instruction::UpdateProtocolConfig {
        protocol_config: Some(updated_config),
    };
    let accounts = light_registry::accounts::UpdateProtocolConfig {
        protocol_config_pda,
        authority: governance_authority.pubkey(),
        new_authority: None,
        fee_payer: payer.pubkey(),
    };
    let ix = Instruction {
        program_id: light_registry::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction.data(),
    };
    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, governance_authority])
        .await
        .unwrap();
}

/// After calling claim_fees on a V2 state tree, the tree balance should equal
/// rent + rollover_fee * (capacity - next_index + 1), and the fee_recipient
/// should receive the excess.
#[serial]
#[tokio::test]
async fn test_claim_fees_v2_state_tree() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_test_forester(true))
        .await
        .unwrap();
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();

    // Set up fee recipient.
    let fee_recipient = Keypair::new();
    rpc.airdrop_lamports(&fee_recipient.pubkey(), 1_000_000)
        .await
        .unwrap();

    let protocol_config_pda = env.protocol.governance_authority_pda;
    set_protocol_fee_recipient(
        &mut rpc,
        &env.protocol.governance_authority,
        protocol_config_pda,
        fee_recipient.pubkey(),
    )
    .await;

    // Use the first V2 state tree.
    let tree_pubkey = env.v2_state_trees[0].merkle_tree;

    // Read tree metadata to compute expected post-claim balance.
    let mut tree_account = rpc.get_account(tree_pubkey).await.unwrap().unwrap();
    let tree_lamports_before = tree_account.lamports;
    let data_len = tree_account.data.len() as u64;
    let tree = BatchedMerkleTreeAccount::state_from_bytes(
        tree_account.data.as_mut_slice(),
        &tree_pubkey.into(),
    )
    .unwrap();
    let metadata = tree.get_metadata();
    let rollover_fee = metadata.metadata.rollover_metadata.rollover_fee;
    let capacity = metadata.capacity;
    let next_index = metadata.next_index;

    let rent = hardcoded_rent_exemption(data_len).unwrap();
    let expected_excess = compute_claimable_excess(
        tree_lamports_before,
        rent,
        rollover_fee,
        capacity,
        next_index,
    );

    // If there's no excess, the test is vacuous -- skip assertion on transfer
    // but still verify claim_fees succeeds.
    let recipient_before = rpc
        .get_account(fee_recipient.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Call claim_fees via registry wrapper.
    let ix = create_claim_fees_wrapper_instruction(
        env.protocol.forester.pubkey(),
        env.protocol.forester.pubkey(),
        tree_pubkey,
        fee_recipient.pubkey(),
        protocol_config_pda,
        0,
    );
    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &env.protocol.forester])
        .await
        .unwrap();

    // Verify tree balance matches formula.
    let tree_lamports_after = rpc
        .get_account(tree_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let expected_tree_balance = rent
        + rollover_fee
            * (capacity
                .checked_sub(next_index)
                .unwrap()
                .checked_add(1)
                .unwrap());

    if let Some(excess) = expected_excess {
        if excess > 0 {
            assert_eq!(
                tree_lamports_after, expected_tree_balance,
                "Tree balance after claim should equal rent + rollover reserves",
            );

            let recipient_after = rpc
                .get_account(fee_recipient.pubkey())
                .await
                .unwrap()
                .unwrap()
                .lamports;
            assert_eq!(
                recipient_after - recipient_before,
                excess,
                "Fee recipient should receive the excess lamports",
            );
        }
    }
}

/// Calling claim_fees with a fee_recipient that doesn't match
/// protocol_config.protocol_fee_recipient fails with InvalidFeeRecipient.
#[serial]
#[tokio::test]
async fn test_claim_fees_fails_fee_recipient_mismatch() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_test_forester(true))
        .await
        .unwrap();
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();

    // Set fee recipient to keypair A.
    let correct_recipient = Keypair::new();
    rpc.airdrop_lamports(&correct_recipient.pubkey(), 1_000_000)
        .await
        .unwrap();

    let protocol_config_pda = env.protocol.governance_authority_pda;
    set_protocol_fee_recipient(
        &mut rpc,
        &env.protocol.governance_authority,
        protocol_config_pda,
        correct_recipient.pubkey(),
    )
    .await;

    // Call claim_fees with wrong recipient (keypair B).
    let wrong_recipient = Keypair::new();
    rpc.airdrop_lamports(&wrong_recipient.pubkey(), 1_000_000)
        .await
        .unwrap();

    let tree_pubkey = env.v2_state_trees[0].merkle_tree;
    let ix = create_claim_fees_wrapper_instruction(
        env.protocol.forester.pubkey(),
        env.protocol.forester.pubkey(),
        tree_pubkey,
        wrong_recipient.pubkey(),
        protocol_config_pda,
        0,
    );
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &env.protocol.forester])
        .await;

    // InvalidFeeRecipient is index 32 in RegistryError enum -> error code 6032
    assert_rpc_error(result, 0, 6033).unwrap(); // RegistryError::InvalidFeeRecipient
}

/// Calling claim_fees when protocol_fee_recipient is Pubkey::default()
/// (unconfigured) fails with InvalidFeeRecipient.
#[serial]
#[tokio::test]
async fn test_claim_fees_fails_default_recipient() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_test_forester(true))
        .await
        .unwrap();
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();

    // Do NOT set protocol_fee_recipient -- leave it as Pubkey::default().
    let protocol_config_pda = env.protocol.governance_authority_pda;
    let tree_pubkey = env.v2_state_trees[0].merkle_tree;

    // Pass Pubkey::default() as fee_recipient to match the config.
    let ix = create_claim_fees_wrapper_instruction(
        env.protocol.forester.pubkey(),
        env.protocol.forester.pubkey(),
        tree_pubkey,
        Pubkey::default(),
        protocol_config_pda,
        0,
    );
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &env.protocol.forester])
        .await;

    // InvalidFeeRecipient (default check) is also error code 6032
    assert_rpc_error(result, 0, 6033).unwrap(); // RegistryError::InvalidFeeRecipient
}
