mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{get_create_accounts_proof, instructions, CreateAccountsProofInput};
use light_compressible::rent::{get_last_funded_epoch, SLOTS_PER_EPOCH};
use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    Indexer, Rpc,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Extra lamports to airdrop to each PDA after creation.
/// The D9 init flow only funds the PDA with rent-exemption lamports.
/// We airdrop additional lamports so the account has a non-trivial rent budget
/// and is NOT immediately compressible.
///
/// 50_000 extra lamports gives ~195 epochs of rent for a 72-byte account
/// (rent_per_epoch = 128 + 72 = 200, available = 50_000 - 11_000 = 39_000).
const EXTRA_RENT_LAMPORTS: u64 = 50_000;

/// Helper: create a D9InstrSinglePubkey PDA, fund it with extra rent lamports,
/// and return (pda, owner).
async fn create_funded_d9_pda(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: &Pubkey,
    rent_sponsor: &Pubkey,
) -> (Pubkey, Pubkey) {
    use csdk_anchor_full_derived_test::D9SinglePubkeyParams;

    let owner = Keypair::new().pubkey();
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"instr_single", owner.as_ref()], program_id);

    let proof_result = get_create_accounts_proof(
        rpc,
        program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D9InstrSinglePubkey {
        fee_payer: payer.pubkey(),
        compression_config: *config_pda,
        pda_rent_sponsor: *rent_sponsor,
        d9_instr_single_pubkey_record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D9InstrSinglePubkey {
        params: D9SinglePubkeyParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
        .expect("D9InstrSinglePubkey should succeed");

    // Fund the PDA with extra lamports so it has a non-trivial rent budget.
    // Without this, the PDA has only rent-exemption lamports and is
    // immediately compressible (available_balance = 0).
    rpc.airdrop_lamports(&record_pda, EXTRA_RENT_LAMPORTS)
        .await
        .expect("Airdrop rent lamports to PDA should succeed");

    (record_pda, owner)
}

/// Compute the boundary slot at which the PDA becomes compressible.
///
/// Returns the first slot of the epoch after which the account's rent balance is exhausted.
/// At the last slot of the last funded epoch, the account is NOT compressible.
/// At the first slot of the next epoch, the account IS compressible.
async fn compute_boundary_slot(rpc: &mut LightProgramTest, pda: &Pubkey) -> u64 {
    let account = rpc
        .get_account(*pda)
        .await
        .unwrap()
        .expect("PDA should exist");

    let record: csdk_anchor_full_derived_test::SinglePubkeyRecord =
        borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();

    let ci = &record.compression_info;
    let rent_config = &ci.rent_config;
    let data_len = account.data.len() as u64;
    let lamports = account.lamports;

    let rent_exemption = rpc
        .get_minimum_balance_for_rent_exemption(account.data.len())
        .await
        .unwrap();

    let last_funded_epoch = get_last_funded_epoch(
        data_len,
        lamports,
        ci.last_claimed_slot,
        rent_config,
        rent_exemption,
    );

    println!(
        "compute_boundary: data_len={}, lamports={}, rent_exemption={}, \
         last_claimed_slot={}, last_funded_epoch={}",
        data_len, lamports, rent_exemption, ci.last_claimed_slot, last_funded_epoch
    );

    (last_funded_epoch + 1) * SLOTS_PER_EPOCH
}

/// Test A: Exact compressibility boundary.
///
/// Creates a PDA funded with enough rent for several epochs and verifies that:
/// - Just before the boundary: compression is a no-op (PDA remains on-chain)
/// - At the boundary: compression proceeds (PDA is closed and compressed)
#[tokio::test]
async fn test_compressibility_boundary() {
    let shared::SharedTestContext {
        mut rpc,
        payer,
        config_pda,
        rent_sponsor,
        program_id,
    } = shared::SharedTestContext::new().await;

    // Create a funded PDA (has extra lamports for rent)
    let (pda, _owner) =
        create_funded_d9_pda(&mut rpc, &payer, &program_id, &config_pda, &rent_sponsor).await;

    shared::assert_onchain_exists(&mut rpc, &pda, "Record").await;

    // Compute the exact compressibility boundary
    let boundary_slot = compute_boundary_slot(&mut rpc, &pda).await;
    let current_slot = rpc.get_slot().await.unwrap();
    println!(
        "boundary_slot = {}, current_slot = {}",
        boundary_slot, current_slot
    );
    assert!(
        boundary_slot > current_slot,
        "boundary_slot ({}) should be in the future (current: {})",
        boundary_slot,
        current_slot
    );

    // Warp to the last slot of the last funded epoch (without auto-compress)
    rpc.warp_to_slot(boundary_slot - 1).unwrap();

    // Manually attempt compression -- should be a no-op (PDA not yet compressible)
    light_program_test::compressible::auto_compress_program_pdas(&mut rpc, program_id)
        .await
        .expect("auto_compress should succeed (no-op)");

    // PDA should still be on-chain
    shared::assert_onchain_exists(&mut rpc, &pda, "Record (before boundary)").await;

    // Warp to the first slot of the next epoch (the boundary)
    rpc.warp_to_slot(boundary_slot).unwrap();

    // Manually attempt compression -- PDA should now be compressible
    light_program_test::compressible::auto_compress_program_pdas(&mut rpc, program_id)
        .await
        .expect("auto_compress should succeed (compress)");

    // PDA should be closed
    shared::assert_onchain_closed(&mut rpc, &pda, "Record (at boundary)").await;

    // Compressed account should exist with data
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_address = light_compressed_account::address::derive_address(
        &pda.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    shared::assert_compressed_exists_with_data(&mut rpc, compressed_address, "Record").await;
}

/// Test B: Batch abort when one account is not compressible.
///
/// Creates two PDAs, warps so both become compressible, then airdrops extra
/// lamports to one PDA making it non-compressible again. Sends a single
/// compress instruction covering both PDAs. The entire batch should be
/// skipped because the validity proof covers all accounts.
#[tokio::test]
async fn test_batch_abort_non_compressible() {
    let shared::SharedTestContext {
        mut rpc,
        payer,
        config_pda,
        rent_sponsor,
        program_id,
    } = shared::SharedTestContext::new().await;

    // Create two funded PDAs
    let (pda_1, _owner_1) =
        create_funded_d9_pda(&mut rpc, &payer, &program_id, &config_pda, &rent_sponsor).await;
    let (pda_2, _owner_2) =
        create_funded_d9_pda(&mut rpc, &payer, &program_id, &config_pda, &rent_sponsor).await;

    shared::assert_onchain_exists(&mut rpc, &pda_1, "PDA 1").await;
    shared::assert_onchain_exists(&mut rpc, &pda_2, "PDA 2").await;

    // Compute boundaries for both PDAs
    let boundary_1 = compute_boundary_slot(&mut rpc, &pda_1).await;
    let boundary_2 = compute_boundary_slot(&mut rpc, &pda_2).await;

    // Warp past both boundaries so both are compressible (no auto-compress)
    let past_both = boundary_1.max(boundary_2) + SLOTS_PER_EPOCH;
    rpc.warp_to_slot(past_both).unwrap();

    // Airdrop a large amount to PDA 1, making it non-compressible again.
    // This gives it enough lamports to cover rent for many more epochs.
    rpc.airdrop_lamports(&pda_1, 1_000_000)
        .await
        .expect("Airdrop to PDA 1 should succeed");

    // Verify PDA 1 is indeed not compressible now
    {
        let account = rpc.get_account(pda_1).await.unwrap().unwrap();
        let record: csdk_anchor_full_derived_test::SinglePubkeyRecord =
            borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
        let ci = &record.compression_info;
        let rent_config = &ci.rent_config;
        let data_len = account.data.len() as u64;
        let rent_exemption = rpc
            .get_minimum_balance_for_rent_exemption(account.data.len())
            .await
            .unwrap();
        let current_slot = rpc.get_slot().await.unwrap();

        let state = light_compressible::rent::AccountRentState {
            num_bytes: data_len,
            current_slot,
            current_lamports: account.lamports,
            last_claimed_slot: ci.last_claimed_slot,
        };
        assert!(
            state.is_compressible(rent_config, rent_exemption).is_none(),
            "PDA 1 should NOT be compressible after airdrop"
        );
    }

    // Build a manual compress instruction with BOTH PDAs in one batch.
    // This requires getting validity proofs for both compressed placeholders.
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let addr_1 = light_compressed_account::address::derive_address(
        &pda_1.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );
    let addr_2 = light_compressed_account::address::derive_address(
        &pda_2.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    // Get compressed account hashes for both
    let cacc_1 = rpc
        .get_compressed_account(addr_1, None)
        .await
        .unwrap()
        .value
        .expect("Compressed placeholder for PDA 1 should exist");
    let cacc_2 = rpc
        .get_compressed_account(addr_2, None)
        .await
        .unwrap()
        .value
        .expect("Compressed placeholder for PDA 2 should exist");

    // Get validity proof covering both accounts
    let proof = rpc
        .get_validity_proof(vec![cacc_1.hash, cacc_2.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Build program account metas (fee_payer, config, rent_sponsor, compression_authority)
    let (config_bytes, _) = light_account::LightConfig::derive_pda_bytes::<
        light_account::AccountInfo<'_>,
    >(&program_id.to_bytes(), 0);
    let light_config_pda = Pubkey::from(config_bytes);
    let program_metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(light_config_pda, false),
        AccountMeta::new(rent_sponsor, false),
        AccountMeta::new_readonly(payer.pubkey(), false),
    ];

    // Build the batch compress instruction
    let ix = instructions::build_compress_accounts_idempotent(
        &program_id,
        &instructions::COMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        &[pda_1, pda_2],
        &program_metas,
        proof,
    )
    .expect("build_compress_accounts_idempotent should succeed");

    // Send the instruction -- should succeed (Ok), not error
    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Batch compress should succeed (no-op due to non-compressible account)");

    // BOTH PDAs should still be on-chain (batch was aborted)
    shared::assert_onchain_exists(&mut rpc, &pda_1, "PDA 1 (after batch abort)").await;
    shared::assert_onchain_exists(&mut rpc, &pda_2, "PDA 2 (after batch abort)").await;

    // Verify that PDA 2 (the compressible one) can still be compressed individually
    // via auto_compress, which processes one PDA at a time.
    light_program_test::compressible::auto_compress_program_pdas(&mut rpc, program_id)
        .await
        .expect("auto_compress should succeed");

    // PDA 2 should now be closed (compressed individually)
    shared::assert_onchain_closed(&mut rpc, &pda_2, "PDA 2 (after individual compress)").await;

    // PDA 1 should still be on-chain (still non-compressible)
    shared::assert_onchain_exists(&mut rpc, &pda_1, "PDA 1 (still on-chain)").await;

    // Verify PDA 2's compressed data exists
    shared::assert_compressed_exists_with_data(&mut rpc, addr_2, "PDA 2 compressed").await;
}
