// Tests for DecompressCMint SDK instruction

mod shared;

use borsh::BorshDeserialize;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_ctoken_interface::instructions::mint_action::CompressedMintWithContext;
use light_ctoken_interface::state::{CompressedMint, ExtensionStruct};
use light_ctoken_sdk::ctoken::{find_cmint_address, DecompressCMint};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// Test decompressing a compressed mint to CMint account
#[tokio::test]
async fn test_decompress_cmint() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    // Create a compressed mint (returns mint_seed keypair)
    let (mint_pda, compression_address, _, mint_seed) =
        shared::setup_create_compressed_mint(&mut rpc, &payer, mint_authority, decimals, vec![])
            .await;

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
        CompressedMint::deserialize(&mut compressed_account.data.as_ref().unwrap().data.as_slice())
            .unwrap();

    let compressed_mint_with_context = CompressedMintWithContext {
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

    // Build and execute DecompressCMint instruction
    let decompress_ix = DecompressCMint {
        mint_seed_pubkey: mint_seed.pubkey(),
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
    let cmint = CompressedMint::deserialize(&mut &cmint_account.data[..]).unwrap();

    // Extract runtime-specific Compressible extension (added during decompression)
    let compressible_ext = cmint
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(info),
                _ => None,
            })
        })
        .expect("CMint should have Compressible extension");

    // Build expected CMint from original compressed mint, updating fields changed by decompression
    let mut expected_cmint = compressed_mint.clone();
    expected_cmint.metadata.cmint_decompressed = true;
    expected_cmint.extensions = Some(vec![ExtensionStruct::Compressible(*compressible_ext)]);

    assert_eq!(cmint, expected_cmint, "CMint should match expected state");
}

/// Test decompressing a compressed mint with freeze_authority
#[tokio::test]
async fn test_decompress_cmint_with_freeze_authority() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let mint_authority = payer.pubkey();
    let freeze_authority = Keypair::new();
    let decimals = 6u8;

    // Create a compressed mint with freeze_authority
    let (mint_pda, compression_address, mint_seed) =
        setup_create_compressed_mint_with_freeze_authority_only(
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
        CompressedMint::deserialize(&mut compressed_account.data.as_ref().unwrap().data.as_slice())
            .unwrap();

    let compressed_mint_with_context = CompressedMintWithContext {
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

    // Build and execute DecompressCMint instruction
    let decompress_ix = DecompressCMint {
        mint_seed_pubkey: mint_seed.pubkey(),
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
    let cmint = CompressedMint::deserialize(&mut &cmint_account.data[..]).unwrap();

    // Extract runtime-specific Compressible extension (added during decompression)
    let compressible_ext = cmint
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(*info),
                _ => None,
            })
        })
        .expect("CMint should have Compressible extension");

    // Build expected CMint from original compressed mint, updating fields changed by decompression
    let mut expected_cmint = compressed_mint.clone();
    expected_cmint.metadata.cmint_decompressed = true;
    expected_cmint.extensions = Some(vec![ExtensionStruct::Compressible(compressible_ext)]);

    assert_eq!(cmint, expected_cmint, "CMint should match expected state");
}

/// Helper function: Creates a compressed mint with optional freeze_authority
/// but does NOT decompress it (unlike setup_create_compressed_mint_with_freeze_authority)
/// Returns (mint_pda, compression_address, mint_seed_keypair)
async fn setup_create_compressed_mint_with_freeze_authority_only(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    decimals: u8,
) -> (Pubkey, [u8; 32], Keypair) {
    use light_ctoken_sdk::ctoken::{CreateCMint, CreateCMintParams};

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_ctoken_sdk::ctoken::derive_cmint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let mint = find_cmint_address(&mint_seed.pubkey()).0;

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
    let params = CreateCMintParams {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint,
        freeze_authority,
        extensions: None,
    };

    // Create instruction directly using SDK
    let create_cmint_builder = CreateCMint::new(
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
async fn test_decompress_cmint_with_token_metadata() {
    use light_ctoken_interface::instructions::extensions::{
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
    let (mint_pda, compression_address, mint_seed) = setup_create_compressed_mint_with_extensions(
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
        CompressedMint::deserialize(&mut compressed_account.data.as_ref().unwrap().data.as_slice())
            .unwrap();

    let compressed_mint_with_context = CompressedMintWithContext {
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

    // Build and execute DecompressCMint instruction
    let decompress_ix = DecompressCMint {
        mint_seed_pubkey: mint_seed.pubkey(),
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
    let cmint = CompressedMint::deserialize(&mut &cmint_account.data[..]).unwrap();

    // Extract runtime-specific Compressible extension (added during decompression)
    let compressible_ext = cmint
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(*info),
                _ => None,
            })
        })
        .expect("CMint should have Compressible extension");

    // Extract the TokenMetadata extension (should be preserved from original)
    let token_metadata_ext = cmint
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::TokenMetadata(tm) => Some(tm.clone()),
                _ => None,
            })
        })
        .expect("CMint should have TokenMetadata extension");

    // Build expected CMint from original compressed mint, updating fields changed by decompression
    let mut expected_cmint = compressed_mint.clone();
    expected_cmint.metadata.cmint_decompressed = true;
    // Extensions should include original TokenMetadata plus new Compressible
    expected_cmint.extensions = Some(vec![
        ExtensionStruct::TokenMetadata(token_metadata_ext),
        ExtensionStruct::Compressible(compressible_ext),
    ]);

    assert_eq!(cmint, expected_cmint, "CMint should match expected state");
}

/// Helper function: Creates a compressed mint with extensions
/// but does NOT decompress it
/// Returns (mint_pda, compression_address, mint_seed_keypair)
async fn setup_create_compressed_mint_with_extensions(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    decimals: u8,
    extensions: Vec<light_ctoken_interface::instructions::extensions::ExtensionInstructionData>,
) -> (Pubkey, [u8; 32], Keypair) {
    use light_ctoken_sdk::ctoken::{CreateCMint, CreateCMintParams};

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_ctoken_sdk::ctoken::derive_cmint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let mint = find_cmint_address(&mint_seed.pubkey()).0;

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
    let params = CreateCMintParams {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint,
        freeze_authority,
        extensions: Some(extensions),
    };

    // Create instruction directly using SDK
    let create_cmint_builder = CreateCMint::new(
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
