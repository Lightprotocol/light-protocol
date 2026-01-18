use anchor_lang::InstructionData;
use light_program_test::{AddressWithTree, Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_token_sdk::token::{
    config_pda, derive_mint_compressed_address, find_mint_address, rent_sponsor_pda,
    SystemAccounts, LIGHT_TOKEN_PROGRAM_ID,
};
use sdk_token_test::{CreateMintsParams, MintParams};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_create_single_mint() {
    test_create_mints(1).await;
}

#[tokio::test]
async fn test_create_two_mints() {
    test_create_mints(2).await;
}

#[tokio::test]
async fn test_create_three_mints() {
    test_create_mints(3).await;
}

async fn test_create_mints(n: usize) {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_signers: Vec<Keypair> = (0..n).map(|_| Keypair::new()).collect();

    let address_tree_info = rpc.get_address_tree_v2();
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    let compression_addresses: Vec<[u8; 32]> = mint_signers
        .iter()
        .map(|signer| derive_mint_compressed_address(&signer.pubkey(), &address_tree_info.tree))
        .collect();

    let mint_pdas: Vec<(solana_sdk::pubkey::Pubkey, u8)> = mint_signers
        .iter()
        .map(|signer| find_mint_address(&signer.pubkey()))
        .collect();

    let addresses_with_trees: Vec<AddressWithTree> = compression_addresses
        .iter()
        .map(|addr| AddressWithTree {
            address: *addr,
            tree: address_tree_info.tree,
        })
        .collect();

    let proof_result = rpc
        .get_validity_proof(vec![], addresses_with_trees, None)
        .await
        .unwrap()
        .value;

    let mints: Vec<MintParams> = mint_signers
        .iter()
        .zip(compression_addresses.iter())
        .zip(mint_pdas.iter())
        .enumerate()
        .map(
            |(i, ((signer, compression_address), (mint_pda, bump)))| MintParams {
                decimals: (6 + i) as u8,
                address_merkle_tree_root_index: proof_result.addresses[i].root_index,
                mint_authority: payer.pubkey(),
                compression_address: *compression_address,
                mint: *mint_pda,
                bump: *bump,
                freeze_authority: None,
                mint_seed_pubkey: signer.pubkey(),
            },
        )
        .collect();

    let params = CreateMintsParams::new(mints, proof_result.proof.0.unwrap());

    let system_accounts = SystemAccounts::default();
    let cpi_context_pubkey = state_tree_info
        .cpi_context
        .expect("CPI context account required");

    // Account layout (remaining_accounts):
    // [0]: light_system_program
    // [1..N+1]: mint_signers (SIGNER)
    // [N+1..N+6]: system PDAs (cpi_authority, registered_program, compression_authority, compression_program, system_program)
    // [N+6]: cpi_context_account
    // [N+7]: output_queue
    // [N+8]: address_tree
    // [N+9]: compressible_config
    // [N+10]: rent_sponsor
    // [N+11]: state_merkle_tree
    // [N+12..2N+12]: mint_pdas
    // [2N+12]: compressed_token_program (for CPI)
    let mut accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(system_accounts.light_system_program, false),
    ];

    for signer in &mint_signers {
        accounts.push(AccountMeta::new_readonly(signer.pubkey(), true));
    }

    accounts.extend(vec![
        AccountMeta::new_readonly(system_accounts.cpi_authority_pda, false),
        AccountMeta::new_readonly(system_accounts.registered_program_pda, false),
        AccountMeta::new_readonly(system_accounts.account_compression_authority, false),
        AccountMeta::new_readonly(system_accounts.account_compression_program, false),
        AccountMeta::new_readonly(system_accounts.system_program, false),
        AccountMeta::new(cpi_context_pubkey, false),
        AccountMeta::new(state_tree_info.queue, false),
        AccountMeta::new(address_tree_info.tree, false),
        AccountMeta::new_readonly(config_pda().into(), false),
        AccountMeta::new(rent_sponsor_pda().into(), false),
        AccountMeta::new(state_tree_info.tree, false),
    ]);

    for (mint_pda, _) in &mint_pdas {
        accounts.push(AccountMeta::new(*mint_pda, false));
    }

    // Append compressed token program at the end for CPI
    accounts.push(AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false));

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts,
        data: sdk_token_test::instruction::CreateMints { params }.data(),
    };

    let mut signers: Vec<&Keypair> = vec![&payer];
    signers.extend(mint_signers.iter());

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await
        .unwrap();

    for (i, (mint_pda, _)) in mint_pdas.iter().enumerate() {
        let mint_account = rpc
            .get_account(*mint_pda)
            .await
            .expect("Failed to get mint account")
            .expect(&format!("Mint PDA {} should exist after decompress", i + 1));

        assert!(
            !mint_account.data.is_empty(),
            "Mint {} account should have data",
            i + 1
        );
    }
}
