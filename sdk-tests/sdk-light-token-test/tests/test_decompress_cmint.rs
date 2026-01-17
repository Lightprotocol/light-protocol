// Tests for DecompressMint SDK instruction

mod shared;

use borsh::BorshDeserialize;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressible::compression_info::CompressionInfo;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token_interface::{instructions::mint_action::MintWithContext, state::Mint};
use light_token_sdk::token::{find_mint_address, DecompressMint};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// Test decompressing a compressed mint to CMint account
#[tokio::test]
async fn test_decompress_mint() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    // Create a compressed mint (returns mint_seed keypair)
    let (mint_pda, compression_address, _, _mint_seed) =
        shared::setup_create_mint(&mut rpc, &payer, mint_authority, decimals, vec![]).await;

    // Verify CMint account does NOT exist on-chain yet
    let cmint_account_before = rpc.get_account(mint_pda).await.unwrap();
    assert!(
        cmint_account_before.is_none(),
        "CMint should not exist before decompression"
    );

    // Verify compressed mint exists
    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value
        .expect("Compressed mint should exist");

    // Get validity proof for decompression
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Deserialize the compressed mint to build context
    let compressed_mint =
        Mint::deserialize(&mut compressed_account.data.as_ref().unwrap().data.as_slice()).unwrap();

    let compressed_mint_with_context = MintWithContext {
        address: compression_address,
        leaf_index: compressed_account.leaf_index,
        prove_by_index: true,
        root_index: rpc_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        mint: Some(compressed_mint.clone().try_into().unwrap()),
    };

    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Build and execute DecompressMint instruction
    let decompress_ix = DecompressMint {
        payer: payer.pubkey(),
        authority: mint_authority,
        state_tree: compressed_account.tree_info.tree,
        input_queue: compressed_account.tree_info.queue,
        output_queue,
        compressed_mint_with_context,
        proof: rpc_result.proof,
        rent_payment: 16,
        write_top_up: 766,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify CMint account now exists on-chain
    let cmint_account_after = rpc.get_account(mint_pda).await.unwrap();
    assert!(
        cmint_account_after.is_some(),
        "CMint should exist after decompression"
    );

    // Verify CMint state with single assert_eq
    let cmint_account = cmint_account_after.unwrap();
    let cmint = Mint::deserialize(&mut &cmint_account.data[..]).unwrap();

    // Verify compression info is set (non-default) when decompressed
    assert_ne!(
        cmint.compression,
        CompressionInfo::default(),
        "CMint compression info should be set when decompressed"
    );

    // Build expected CMint from original compressed mint, updating fields changed by decompression
    let mut expected_cmint = compressed_mint.clone();
    expected_cmint.metadata.mint_decompressed = true;
    expected_cmint.compression = cmint.compression;

    assert_eq!(cmint, expected_cmint, "CMint should match expected state");
}

/// Test decompressing a compressed mint with freeze_authority
#[tokio::test]
async fn test_decompress_mint_with_freeze_authority() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let mint_authority = payer.pubkey();
    let freeze_authority = Keypair::new();
    let decimals = 6u8;

    // Create a compressed mint with freeze_authority
    let (mint_pda, compression_address, _mint_seed) = setup_create_mint_with_freeze_authority_only(
        &mut rpc,
        &payer,
        mint_authority,
        Some(freeze_authority.pubkey()),
        decimals,
    )
    .await;

    // Verify CMint account does NOT exist on-chain yet
    let cmint_account_before = rpc.get_account(mint_pda).await.unwrap();
    assert!(
        cmint_account_before.is_none(),
        "CMint should not exist before decompression"
    );

    // Get compressed mint account
    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value
        .expect("Compressed mint should exist");

    // Get validity proof for decompression
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Deserialize the compressed mint
    let compressed_mint =
        Mint::deserialize(&mut compressed_account.data.as_ref().unwrap().data.as_slice()).unwrap();

    let compressed_mint_with_context = MintWithContext {
        address: compression_address,
        leaf_index: compressed_account.leaf_index,
        prove_by_index: true,
        root_index: rpc_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        mint: Some(compressed_mint.clone().try_into().unwrap()),
    };

    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Build and execute DecompressMint instruction
    let decompress_ix = DecompressMint {
        payer: payer.pubkey(),
        authority: mint_authority,
        state_tree: compressed_account.tree_info.tree,
        input_queue: compressed_account.tree_info.queue,
        output_queue,
        compressed_mint_with_context,
        proof: rpc_result.proof,
        rent_payment: 16,
        write_top_up: 766,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify CMint state with single assert_eq
    let cmint_account = rpc
        .get_account(mint_pda)
        .await
        .unwrap()
        .expect("CMint should exist after decompression");
    let cmint = Mint::deserialize(&mut &cmint_account.data[..]).unwrap();

    // Verify compression info is set (non-default) when decompressed
    assert_ne!(
        cmint.compression,
        CompressionInfo::default(),
        "CMint compression info should be set when decompressed"
    );

    // Build expected CMint from original compressed mint, updating fields changed by decompression
    let mut expected_cmint = compressed_mint.clone();
    expected_cmint.metadata.mint_decompressed = true;
    expected_cmint.compression = cmint.compression;

    assert_eq!(cmint, expected_cmint, "CMint should match expected state");
}

/// Helper function: Creates a compressed mint with optional freeze_authority
/// but does NOT decompress it (unlike setup_create_mint_with_freeze_authority)
/// Returns (mint_pda, compression_address, mint_seed_keypair)
async fn setup_create_mint_with_freeze_authority_only(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    decimals: u8,
) -> (Pubkey, [u8; 32], Keypair) {
    use light_token_sdk::token::{CreateMint, CreateMintParams};

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_token_sdk::token::derive_mint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let (mint, bump) = find_mint_address(&mint_seed.pubkey());

    // Get validity proof for the address
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_client::indexer::AddressWithTree {
                address: compression_address,
                tree: address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    // Build params for the SDK
    let params = CreateMintParams {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint,
        bump,
        freeze_authority,
        extensions: None,
        rent_payment: 16,
        write_top_up: 766,
    };

    // Create instruction directly using SDK
    let create_cmint_builder = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_cmint_builder.instruction().unwrap();

    // Send transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    // Verify the compressed mint was created
    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;

    assert!(
        compressed_account.is_some(),
        "Compressed mint should exist after setup"
    );

    (mint, compression_address, mint_seed)
}

/// Test decompressing a compressed mint with TokenMetadata extension
#[tokio::test]
async fn test_decompress_mint_with_token_metadata() {
    use light_token_interface::instructions::extensions::{
        ExtensionInstructionData, TokenMetadataInstructionData,
    };

    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let mint_authority = payer.pubkey();
    let update_authority = Keypair::new();
    let decimals = 9u8;

    // Create TokenMetadata extension
    let token_metadata = TokenMetadataInstructionData {
        update_authority: Some(update_authority.pubkey().to_bytes().into()),
        name: b"Test Token".to_vec(),
        symbol: b"TEST".to_vec(),
        uri: b"https://example.com/token.json".to_vec(),
        additional_metadata: None,
    };
    let extensions = vec![ExtensionInstructionData::TokenMetadata(token_metadata)];

    // Create a compressed mint with TokenMetadata extension
    let (mint_pda, compression_address, _mint_seed) = setup_create_mint_with_extensions(
        &mut rpc,
        &payer,
        mint_authority,
        None,
        decimals,
        extensions,
    )
    .await;

    // Verify CMint account does NOT exist on-chain yet
    let cmint_account_before = rpc.get_account(mint_pda).await.unwrap();
    assert!(
        cmint_account_before.is_none(),
        "CMint should not exist before decompression"
    );

    // Get compressed mint account
    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value
        .expect("Compressed mint should exist");

    // Get validity proof for decompression
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Deserialize the compressed mint
    let compressed_mint =
        Mint::deserialize(&mut compressed_account.data.as_ref().unwrap().data.as_slice()).unwrap();

    let compressed_mint_with_context = MintWithContext {
        address: compression_address,
        leaf_index: compressed_account.leaf_index,
        prove_by_index: true,
        root_index: rpc_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        mint: Some(compressed_mint.clone().try_into().unwrap()),
    };

    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Build and execute DecompressMint instruction
    let decompress_ix = DecompressMint {
        payer: payer.pubkey(),
        authority: mint_authority,
        state_tree: compressed_account.tree_info.tree,
        input_queue: compressed_account.tree_info.queue,
        output_queue,
        compressed_mint_with_context,
        proof: rpc_result.proof,
        rent_payment: 16,
        write_top_up: 766,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify CMint state with single assert_eq
    let cmint_account = rpc
        .get_account(mint_pda)
        .await
        .unwrap()
        .expect("CMint should exist after decompression");
    let cmint = Mint::deserialize(&mut &cmint_account.data[..]).unwrap();

    // Verify compression info is set (non-default) when decompressed
    assert_ne!(
        cmint.compression,
        CompressionInfo::default(),
        "CMint compression info should be set when decompressed"
    );

    // Verify TokenMetadata extension is preserved
    assert!(
        cmint.extensions.is_some(),
        "CMint should have extensions with TokenMetadata"
    );

    // Build expected CMint from original compressed mint, updating fields changed by decompression
    let mut expected_cmint = compressed_mint.clone();
    expected_cmint.metadata.mint_decompressed = true;
    expected_cmint.compression = cmint.compression;
    // Extensions should preserve original TokenMetadata

    assert_eq!(cmint, expected_cmint, "CMint should match expected state");
}

/// Helper function: Creates a compressed mint with extensions
/// but does NOT decompress it
/// Returns (mint_pda, compression_address, mint_seed_keypair)
async fn setup_create_mint_with_extensions(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    decimals: u8,
    extensions: Vec<light_token_interface::instructions::extensions::ExtensionInstructionData>,
) -> (Pubkey, [u8; 32], Keypair) {
    use light_token_sdk::token::{CreateMint, CreateMintParams};

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_token_sdk::token::derive_mint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let (mint, bump) = find_mint_address(&mint_seed.pubkey());

    // Get validity proof for the address
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_client::indexer::AddressWithTree {
                address: compression_address,
                tree: address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    // Build params for the SDK
    let params = CreateMintParams {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint,
        bump,
        freeze_authority,
        extensions: Some(extensions),
        rent_payment: 16,
        write_top_up: 766,
    };

    // Create instruction directly using SDK
    let create_cmint_builder = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_cmint_builder.instruction().unwrap();

    // Send transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    // Verify the compressed mint was created
    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;

    assert!(
        compressed_account.is_some(),
        "Compressed mint should exist after setup"
    );

    (mint, compression_address, mint_seed)
}

/// Test decompressing a compressed mint via CPI with PDA authority using invoke_signed
#[tokio::test]
async fn test_decompress_mint_cpi_invoke_signed() {
    use borsh::BorshSerialize;
    use native_ctoken_examples::{
        CreateCmintData, DecompressCmintData, InstructionType, ID, MINT_AUTHORITY_SEED,
        MINT_SIGNER_SEED,
    };
    use solana_sdk::instruction::{AccountMeta, Instruction};

    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDAs from our wrapper program
    let (mint_signer_pda, _) = Pubkey::find_program_address(&[MINT_SIGNER_SEED], &ID);
    let (pda_mint_authority, _) = Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &ID);

    let decimals = 9u8;
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using the PDA mint_signer
    let compression_address = light_token_sdk::token::derive_mint_compressed_address(
        &mint_signer_pda,
        &address_tree.tree,
    );

    let (mint_pda, mint_bump) = find_mint_address(&mint_signer_pda);

    // Step 1: Create compressed mint with PDA authority using wrapper program (discriminator 14)
    {
        let rpc_result = rpc
            .get_validity_proof(
                vec![],
                vec![light_client::indexer::AddressWithTree {
                    address: compression_address,
                    tree: address_tree.tree,
                }],
                None,
            )
            .await
            .unwrap()
            .value;

        let compressed_token_program_id =
            Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
        let default_pubkeys = light_token_sdk::utils::TokenDefaultAccounts::default();

        let create_cmint_data = CreateCmintData {
            decimals,
            address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
            mint_authority: pda_mint_authority,
            proof: rpc_result.proof.0.unwrap(),
            compression_address,
            mint: mint_pda,
            bump: mint_bump,
            freeze_authority: None,
            extensions: None,
            rent_payment: 16,
            write_top_up: 766,
        };
        // Discriminator 14 = CreateCmintWithPdaAuthority
        let wrapper_instruction_data =
            [vec![14u8], create_cmint_data.try_to_vec().unwrap()].concat();

        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
            AccountMeta::new_readonly(mint_signer_pda, false),
            AccountMeta::new(pda_mint_authority, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
            AccountMeta::new_readonly(default_pubkeys.system_program, false),
            AccountMeta::new(output_queue, false),
            AccountMeta::new(address_tree.tree, false),
        ];

        let create_mint_ix = Instruction {
            program_id: ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };

        rpc.create_and_send_transaction(&[create_mint_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();
    }

    // Verify CMint account does NOT exist on-chain yet
    let cmint_account_before = rpc.get_account(mint_pda).await.unwrap();
    assert!(
        cmint_account_before.is_none(),
        "CMint should not exist before decompression"
    );

    // Step 2: Decompress the mint via wrapper program (PDA authority requires CPI)
    let compressed_mint = {
        let compressed_mint_account = rpc
            .get_compressed_account(compression_address, None)
            .await
            .unwrap()
            .value
            .expect("Compressed mint should exist");

        let compressed_mint = Mint::deserialize(
            &mut compressed_mint_account
                .data
                .as_ref()
                .unwrap()
                .data
                .as_slice(),
        )
        .unwrap();

        let rpc_result = rpc
            .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
            .await
            .unwrap()
            .value;

        let compressed_mint_with_context = MintWithContext {
            address: compression_address,
            leaf_index: compressed_mint_account.leaf_index,
            prove_by_index: true,
            root_index: rpc_result.accounts[0]
                .root_index
                .root_index()
                .unwrap_or_default(),
            mint: Some(compressed_mint.clone().try_into().unwrap()),
        };

        let default_pubkeys = light_token_sdk::utils::TokenDefaultAccounts::default();
        let compressible_config = light_token_sdk::token::config_pda();
        let rent_sponsor = light_token_sdk::token::rent_sponsor_pda();

        let decompress_data = DecompressCmintData {
            compressed_mint_with_context,
            proof: rpc_result.proof,
            rent_payment: 16,
            write_top_up: 766,
        };

        // Discriminator 33 = DecompressCmintInvokeSigned
        let wrapper_instruction_data = [
            vec![InstructionType::DecompressCmintInvokeSigned as u8],
            decompress_data.try_to_vec().unwrap(),
        ]
        .concat();

        // Account order matches process_decompress_mint_invoke_signed:
        // 0: authority (PDA, readonly - program signs)
        // 1: payer (signer, writable)
        // 2: cmint (writable)
        // 3: compressible_config (readonly)
        // 4: rent_sponsor (writable)
        // 5: state_tree (writable)
        // 6: input_queue (writable)
        // 7: output_queue (writable)
        // 8: light_system_program (readonly)
        // 9: cpi_authority_pda (readonly)
        // 10: registered_program_pda (readonly)
        // 11: account_compression_authority (readonly)
        // 12: account_compression_program (readonly)
        // 13: system_program (readonly)
        // 14: light_token_program (readonly) - required for CPI
        let light_token_program_id =
            Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(pda_mint_authority, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(mint_pda, false),
            AccountMeta::new_readonly(compressible_config, false),
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new(compressed_mint_account.tree_info.tree, false),
            AccountMeta::new(compressed_mint_account.tree_info.queue, false),
            AccountMeta::new(output_queue, false),
            AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
            AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
            AccountMeta::new_readonly(default_pubkeys.system_program, false),
            AccountMeta::new_readonly(light_token_program_id, false),
        ];

        let decompress_ix = Instruction {
            program_id: ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };

        rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        compressed_mint
    };

    // Verify CMint state with single assert_eq
    let cmint_account = rpc
        .get_account(mint_pda)
        .await
        .unwrap()
        .expect("CMint should exist after decompression");
    let cmint = Mint::deserialize(&mut &cmint_account.data[..]).unwrap();

    // Verify compression info is set (non-default) when decompressed
    assert_ne!(
        cmint.compression,
        CompressionInfo::default(),
        "CMint compression info should be set when decompressed"
    );

    // Build expected CMint from original compressed mint, updating fields changed by decompression
    let mut expected_cmint = compressed_mint.clone();
    expected_cmint.metadata.mint_decompressed = true;
    expected_cmint.compression = cmint.compression;

    assert_eq!(cmint, expected_cmint, "CMint should match expected state");
}
