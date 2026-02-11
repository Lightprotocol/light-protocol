//#![cfg(feature = "test-sbf")]

use anchor_lang::InstructionData;
/// Test input range for multi-input tests
const TEST_INPUT_RANGE: [usize; 4] = [1, 2, 3, 4];

use light_compressed_token_sdk::compressed_token::{
    create_compressed_mint::find_mint_address, decompress_full::DecompressFullAccounts,
};
use light_program_test::{Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::{
    actions::{legacy::instructions::mint_action::NewMint, mint_action_comprehensive},
    airdrop_lamports,
};
use light_token_interface::instructions::mint_action::Recipient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

/// Test context containing all the common test data
struct TestContext {
    payer: Keypair,
    owner: Keypair,
    #[allow(dead_code)]
    mint_seed: Keypair,
    mint_pubkey: Pubkey,
    destination_accounts: Vec<Pubkey>,
    compressed_amount_per_account: u64,
    total_compressed_amount: u64,
}

/// Setup function for decompress_full tests
/// Creates compressed tokens (source) and empty decompressed accounts (destination)
async fn setup_decompress_full_test(num_inputs: usize) -> (LightProgramTest, TestContext) {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    let mint_seed = Keypair::new();
    let mint_pubkey = find_mint_address(&mint_seed.pubkey()).0;
    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    let owner = Keypair::new();
    airdrop_lamports(&mut rpc, &owner.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    use light_test_utils::actions::legacy::instructions::mint_action::DecompressMintParams;
    use light_token::instruction::{
        derive_token_ata, CompressibleParams, CreateAssociatedTokenAccount,
    };

    let total_compressed_amount = 1000;
    let compressed_amount_per_account = total_compressed_amount / num_inputs as u64;

    let compressed_recipients: Vec<Recipient> = (0..num_inputs)
        .map(|_| Recipient::new(owner.pubkey(), compressed_amount_per_account))
        .collect();

    println!(
        "Minting {} tokens to {} compressed accounts ({} per account) for owner",
        total_compressed_amount, num_inputs, compressed_amount_per_account
    );

    // First create AND decompress the mint (CToken ATA creation requires mint to exist on-chain)
    // Also mint compressed tokens in the same call
    mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &payer,
        &payer,
        Some(DecompressMintParams::default()), // decompress mint so it exists on-chain
        false,                                 // compress_and_close_mint
        compressed_recipients,
        Vec::new(),
        None,
        None,
        Some(NewMint {
            decimals,
            mint_authority,
            supply: 0,
            freeze_authority: None,
            metadata: None,
            version: 3,
        }),
    )
    .await
    .unwrap();

    // Now create destination ATAs - mint exists on-chain
    let mut destination_accounts = Vec::with_capacity(num_inputs);

    for i in 0..num_inputs {
        let destination_owner = if i == 0 {
            owner.pubkey()
        } else {
            let additional_owner = Keypair::new();
            airdrop_lamports(&mut rpc, &additional_owner.pubkey(), 10_000_000_000)
                .await
                .unwrap();
            additional_owner.pubkey()
        };

        let destination_account = derive_token_ata(&destination_owner, &mint_pubkey);

        let compressible_params = CompressibleParams {
            compressible_config: rpc
                .test_accounts
                .funding_pool_config
                .compressible_config_pda,
            rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
            pre_pay_num_epochs: 0,
            lamports_per_write: None,
            compress_to_account_pubkey: None,
            token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            compression_only: true,
        };

        let create_token_account_ix =
            CreateAssociatedTokenAccount::new(payer.pubkey(), destination_owner, mint_pubkey)
                .with_compressible(compressible_params)
                .instruction()
                .unwrap();

        rpc.create_and_send_transaction(&[create_token_account_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        destination_accounts.push(destination_account);
    }

    (
        rpc,
        TestContext {
            payer,
            owner,
            mint_seed,
            mint_pubkey,
            destination_accounts,
            compressed_amount_per_account,
            total_compressed_amount,
        },
    )
}

/// Test the decompress_full_cpi instruction
/// This test verifies that DecompressFull mode works correctly through CPI
/// Moving tokens from compressed state to decompressed ctoken account
#[tokio::test]
async fn test_decompress_full_cpi() {
    for num_inputs in TEST_INPUT_RANGE {
        println!("Testing decompress_full_cpi with {} inputs", num_inputs);
        let (mut rpc, ctx) = setup_decompress_full_test(num_inputs).await;
        let payer_pubkey = ctx.payer.pubkey();

        let compressed_accounts = rpc
            .get_compressed_token_accounts_by_owner(&ctx.owner.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        assert_eq!(
            compressed_accounts.len(),
            num_inputs,
            "Should have {} compressed accounts",
            num_inputs
        );

        for compressed_account in &compressed_accounts {
            assert_eq!(
                compressed_account.token.amount,
                ctx.compressed_amount_per_account
            );
            assert_eq!(compressed_account.token.mint, ctx.mint_pubkey);
        }

        for destination_account in &ctx.destination_accounts {
            let dest_account = rpc
                .get_account(*destination_account)
                .await
                .unwrap()
                .unwrap();
            use light_token_interface::state::Token;
            use light_zero_copy::traits::ZeroCopyAt;
            let (dest_token, _) = Token::zero_copy_at(&dest_account.data).unwrap();
            assert_eq!(
                u64::from(dest_token.amount),
                0,
                "Destination should be empty initially"
            );
        }

        let mut remaining_accounts = PackedAccounts::default();
        let compressed_hashes: Vec<_> = compressed_accounts
            .iter()
            .map(|acc| acc.account.hash)
            .collect();
        let rpc_result = rpc
            .get_validity_proof(compressed_hashes, vec![], None)
            .await
            .unwrap()
            .value;

        let packed_tree_info = rpc_result.pack_tree_infos(&mut remaining_accounts);
        let config = DecompressFullAccounts::new(None);
        remaining_accounts
            .add_custom_system_accounts(config)
            .unwrap();

        let token_data: Vec<_> = compressed_accounts
            .iter()
            .map(|acc| acc.token.clone())
            .collect();

        let versions: Vec<_> = compressed_accounts
            .iter()
            .map(|acc| {
                let discriminator = acc.account.data.as_ref().unwrap().discriminator;
                light_token_interface::state::TokenDataVersion::from_discriminator(discriminator)
                    .unwrap() as u8
            })
            .collect();

        let indices: Vec<_> = token_data
            .iter()
            .zip(
                packed_tree_info
                    .state_trees
                    .as_ref()
                    .unwrap()
                    .packed_tree_infos
                    .iter(),
            )
            .zip(ctx.destination_accounts.iter())
            .zip(versions.iter())
            .map(|(((token, tree_info), &dest_pubkey), &version)| {
                light_compressed_token_sdk::compressed_token::decompress_full::pack_for_decompress_full(
                    token,
                    tree_info,
                    dest_pubkey,
                    &mut remaining_accounts,
                    None, // No TLV extensions
                    version,
                )
            })
            .collect();

        let validity_proof = rpc_result.proof;
        let (account_metas, _, _) = remaining_accounts.to_account_metas();
        let instruction_data = sdk_token_test::instruction::DecompressFullCpi {
            indices,
            validity_proof,
        };

        let instruction = Instruction {
            program_id: sdk_token_test::ID,
            accounts: [vec![AccountMeta::new(payer_pubkey, true)], account_metas].concat(),
            data: instruction_data.data(),
        };

        rpc.create_and_send_transaction(&[instruction], &payer_pubkey, &[&ctx.payer, &ctx.owner])
            .await
            .unwrap();

        let remaining_compressed = rpc
            .get_compressed_token_accounts_by_owner(&ctx.owner.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        assert_eq!(
            remaining_compressed.len(),
            0,
            "All compressed accounts should be consumed"
        );

        for destination_account in &ctx.destination_accounts {
            let dest_account_after = rpc
                .get_account(*destination_account)
                .await
                .unwrap()
                .unwrap();
            use light_token_interface::state::Token;
            use light_zero_copy::traits::ZeroCopyAt;
            let (dest_token_after, _) = Token::zero_copy_at(&dest_account_after.data).unwrap();
            assert_eq!(
                u64::from(dest_token_after.amount),
                ctx.compressed_amount_per_account,
                "Each destination should have its decompressed amount"
            );
        }

        println!("Successfully decompressed {} inputs", num_inputs);
    }
}

/// Test decompress_full with the CPI context instruction variant
///
/// NOTE: After the mint validation change, this test no longer uses CPI context because:
/// 1. CToken ATAs require an on-chain (decompressed) mint
/// 2. MintWithContext requires a compressed mint
/// 3. The program ties CPI context to minting (with_cpi_context = params.is_some())
///
/// Since these constraints are mutually exclusive, we can only test the instruction
/// variant without actually using CPI context. The core DecompressFull functionality
/// is tested in test_decompress_full_cpi.
#[tokio::test]
async fn test_decompress_full_cpi_with_context() {
    for num_inputs in TEST_INPUT_RANGE {
        println!(
            "Testing decompress_full_cpi_with_context with {} inputs",
            num_inputs
        );
        let (mut rpc, ctx) = setup_decompress_full_test(num_inputs).await;
        let payer_pubkey = ctx.payer.pubkey();

        let initial_compressed_accounts = rpc
            .get_compressed_token_accounts_by_owner(&ctx.owner.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        assert_eq!(
            initial_compressed_accounts.len(),
            num_inputs,
            "Should have {} compressed accounts initially",
            num_inputs
        );

        for destination_account in &ctx.destination_accounts {
            let dest_account_before = rpc
                .get_account(*destination_account)
                .await
                .unwrap()
                .unwrap();
            use light_token_interface::state::Token;
            use light_zero_copy::traits::ZeroCopyAt;
            let (dest_token_before, _) = Token::zero_copy_at(&dest_account_before.data).unwrap();
            assert_eq!(
                u64::from(dest_token_before.amount),
                0,
                "Destination should be empty initially"
            );
        }

        let mut remaining_accounts = PackedAccounts::default();

        let compressed_hashes: Vec<_> = initial_compressed_accounts
            .iter()
            .map(|acc| acc.account.hash)
            .collect();
        let rpc_result = rpc
            .get_validity_proof(compressed_hashes, vec![], None)
            .await
            .unwrap()
            .value;

        // Add tree accounts first, then custom system accounts (no CPI context since params is None)
        let packed_tree_info = rpc_result.pack_tree_infos(&mut remaining_accounts);
        let config = DecompressFullAccounts::new(None);
        remaining_accounts
            .add_custom_system_accounts(config)
            .unwrap();

        let token_data: Vec<_> = initial_compressed_accounts
            .iter()
            .map(|acc| acc.token.clone())
            .collect();

        let versions: Vec<_> = initial_compressed_accounts
            .iter()
            .map(|acc| {
                let discriminator = acc.account.data.as_ref().unwrap().discriminator;
                light_token_interface::state::TokenDataVersion::from_discriminator(discriminator)
                    .unwrap() as u8
            })
            .collect();

        let indices: Vec<_> = token_data
            .iter()
            .zip(
                packed_tree_info
                    .state_trees
                    .as_ref()
                    .unwrap()
                    .packed_tree_infos
                    .iter(),
            )
            .zip(ctx.destination_accounts.iter())
            .zip(versions.iter())
            .map(|(((token, tree_info), &dest_pubkey), &version)| {
                light_compressed_token_sdk::compressed_token::decompress_full::pack_for_decompress_full(
                    token,
                    tree_info,
                    dest_pubkey,
                    &mut remaining_accounts,
                    None, // No TLV extensions
                    version,
                )
            })
            .collect();

        let validity_proof = rpc_result.proof;

        let (account_metas, _, _) = remaining_accounts.to_account_metas();

        let instruction_data = sdk_token_test::instruction::DecompressFullCpiWithCpiContext {
            indices,
            validity_proof,
            params: None,
        };

        let instruction = Instruction {
            program_id: sdk_token_test::ID,
            accounts: [vec![AccountMeta::new(payer_pubkey, true)], account_metas].concat(),
            data: instruction_data.data(),
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer_pubkey),
            &[&ctx.payer, &ctx.owner],
            rpc.get_latest_blockhash().await.unwrap().0,
        );

        rpc.process_transaction(transaction).await.unwrap();

        // All compressed accounts should be consumed (decompressed)
        let final_compressed_accounts = rpc
            .get_compressed_token_accounts_by_owner(&ctx.owner.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        assert_eq!(
            final_compressed_accounts.len(),
            0,
            "All compressed accounts should be consumed"
        );

        for destination_account in &ctx.destination_accounts {
            let dest_account_after = rpc
                .get_account(*destination_account)
                .await
                .unwrap()
                .unwrap();
            use light_token_interface::state::Token;
            use light_zero_copy::traits::ZeroCopyAt;
            let (dest_token_after, _) = Token::zero_copy_at(&dest_account_after.data).unwrap();
            assert_eq!(
                u64::from(dest_token_after.amount),
                ctx.compressed_amount_per_account,
                "Each destination should have received its decompressed amount"
            );
        }

        println!(
            "DecompressFull CPI (context variant) test passed with {} inputs!",
            num_inputs
        );
        println!(
            "  - {} tokens decompressed to {} destinations ({} each)",
            ctx.total_compressed_amount, num_inputs, ctx.compressed_amount_per_account
        );
    }
}
