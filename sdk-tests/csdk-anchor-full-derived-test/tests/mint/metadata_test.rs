//! Integration tests for mint with metadata support in #[light_account(init)] macro.

#[path = "../shared.rs"]
mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{
    decompress_mint::decompress_mint, get_create_accounts_proof, AccountInterfaceExt,
    CreateAccountsProofInput,
};
use light_compressible::{rent::SLOTS_PER_EPOCH, DECOMPRESSED_PDA_DISCRIMINATOR};
use light_program_test::{program_test::TestRpc, Indexer, Rpc};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Test creating a mint with metadata and full lifecycle.
/// Phase 1: Create mint on-chain with metadata (name, symbol, uri, update_authority, additional_metadata)
/// Phase 2: Warp slots to trigger auto-compression by forester
/// Phase 3: Decompress mint and verify metadata is preserved
#[tokio::test]
async fn test_create_mint_with_metadata() {
    use csdk_anchor_full_derived_test::instruction_accounts::{
        CreateMintWithMetadataParams, METADATA_MINT_SIGNER_SEED,
    };
    use light_token::instruction::{
        find_mint_address as find_cmint_address, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR,
    };

    let shared::SharedTestContext {
        mut rpc,
        payer,
        config_pda,
        rent_sponsor: _,
        program_id,
    } = shared::SharedTestContext::new().await;

    let authority = Keypair::new();

    // Derive PDA for mint signer
    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[METADATA_MINT_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );

    // Derive mint PDA
    let (cmint_pda, _) = find_cmint_address(&mint_signer_pda);

    // Get proof for the mint
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::mint(mint_signer_pda)],
    )
    .await
    .unwrap();

    // Define metadata
    let name = b"Test Token".to_vec();
    let symbol = b"TEST".to_vec();
    let uri = b"https://example.com/metadata.json".to_vec();
    let additional_metadata = Some(vec![
        light_token::AdditionalMetadata {
            key: b"author".to_vec(),
            value: b"Light Protocol".to_vec(),
        },
        light_token::AdditionalMetadata {
            key: b"version".to_vec(),
            value: b"1.0.0".to_vec(),
        },
    ]);

    let accounts = csdk_anchor_full_derived_test::accounts::CreateMintWithMetadata {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer: mint_signer_pda,
        cmint: cmint_pda,
        compression_config: config_pda,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::CreateMintWithMetadata {
        params: CreateMintWithMetadataParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            mint_signer_bump,
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            additional_metadata: additional_metadata.clone(),
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateMintWithMetadata should succeed");

    // Verify mint exists on-chain
    let cmint_account = rpc
        .get_account(cmint_pda)
        .await
        .unwrap()
        .expect("Mint should exist on-chain");

    // Parse and verify mint data
    use light_token_interface::state::Mint;
    let mint: Mint = borsh::BorshDeserialize::deserialize(&mut &cmint_account.data[..])
        .expect("Failed to deserialize Mint");

    // Full Mint struct assertion after init
    {
        use light_token_interface::state::mint::BaseMint;
        let expected_mint = Mint {
            base: BaseMint {
                mint_authority: Some(payer.pubkey().to_bytes().into()),
                supply: 0,
                decimals: 9,
                is_initialized: true,
                freeze_authority: None,
            },
            metadata: mint.metadata.clone(),
            reserved: mint.reserved,
            account_type: mint.account_type,
            compression: mint.compression,
            extensions: mint.extensions.clone(),
        };
        assert_eq!(mint, expected_mint, "Mint should match expected after init");
    }

    // Verify token metadata extension details
    use light_token_interface::state::extensions::ExtensionStruct;
    let extensions = mint
        .extensions
        .as_ref()
        .expect("Mint should have extensions");
    let token_metadata = extensions
        .iter()
        .find_map(|ext| {
            if let ExtensionStruct::TokenMetadata(tm) = ext {
                Some(tm)
            } else {
                None
            }
        })
        .expect("Mint should have TokenMetadata extension");

    assert_eq!(token_metadata.name, name, "Token name should match");
    assert_eq!(token_metadata.symbol, symbol, "Token symbol should match");
    assert_eq!(token_metadata.uri, uri, "Token URI should match");

    let expected_update_authority: light_compressed_account::Pubkey =
        authority.pubkey().to_bytes().into();
    assert_eq!(
        token_metadata.update_authority, expected_update_authority,
        "Update authority should be authority signer"
    );

    let additional = &token_metadata.additional_metadata;
    assert_eq!(
        additional.len(),
        2,
        "Should have 2 additional metadata entries"
    );
    assert_eq!(additional[0].key, b"author".to_vec());
    assert_eq!(additional[0].value, b"Light Protocol".to_vec());
    assert_eq!(additional[1].key, b"version".to_vec());
    assert_eq!(additional[1].value, b"1.0.0".to_vec());

    // Verify compressed address registered (mints always use MINT_ADDRESS_TREE)
    use light_token_interface::MINT_ADDRESS_TREE;
    let mint_address_tree = solana_pubkey::Pubkey::new_from_array(MINT_ADDRESS_TREE);
    let mint_compressed_address =
        light_compressed_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
            &mint_signer_pda,
            &mint_address_tree,
        );
    let compressed_mint = rpc
        .get_compressed_account(mint_compressed_address, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(
        compressed_mint.address.unwrap(),
        mint_compressed_address,
        "Mint compressed address should be registered"
    );

    // Verify compressed mint account has decompressed PDA format
    let cmint_data = compressed_mint.data.as_ref().unwrap();
    assert_eq!(
        cmint_data.discriminator, DECOMPRESSED_PDA_DISCRIMINATOR,
        "Decompressed PDA should have DECOMPRESSED_PDA_DISCRIMINATOR"
    );
    assert_eq!(
        cmint_data.data,
        cmint_pda.to_bytes().to_vec(),
        "Decompressed PDA data should contain the PDA pubkey"
    );

    // PHASE 2: Warp to trigger auto-compression by forester
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    // After warp: mint should be closed on-chain
    shared::assert_onchain_closed(&mut rpc, &cmint_pda, "cmint").await;

    // Compressed mint should exist with non-empty data (now compressed)
    shared::assert_compressed_exists_with_data(&mut rpc, mint_compressed_address, "cmint").await;

    // PHASE 3: Decompress mint and verify metadata is preserved

    // Fetch mint interface (unified hot/cold handling)
    // Note: pass the mint PDA (cmint_pda), not the mint signer seed
    let mint_interface = rpc
        .get_mint_interface(&cmint_pda)
        .await
        .expect("get_mint_interface should succeed");
    assert!(mint_interface.is_cold(), "Mint should be cold after warp");

    // Create decompression instruction using decompress_mint helper
    let decompress_instructions = decompress_mint(&mint_interface, payer.pubkey(), &rpc)
        .await
        .expect("decompress_mint should succeed");

    // Should have 1 instruction for mint decompression
    assert_eq!(
        decompress_instructions.len(),
        1,
        "Should have 1 instruction for mint decompression"
    );

    // Execute decompression
    rpc.create_and_send_transaction(&decompress_instructions, &payer.pubkey(), &[&payer])
        .await
        .expect("Mint decompression should succeed");

    // Verify mint is back on-chain
    shared::assert_onchain_exists(&mut rpc, &cmint_pda, "cmint").await;

    // Re-parse and verify mint data with metadata preserved
    let cmint_account_after = rpc
        .get_account(cmint_pda)
        .await
        .unwrap()
        .expect("Mint should exist on-chain after decompression");

    let mint_after: Mint = borsh::BorshDeserialize::deserialize(&mut &cmint_account_after.data[..])
        .expect("Failed to deserialize Mint after decompression");

    // Full Mint struct assertion after decompression
    {
        use light_token_interface::state::mint::BaseMint;
        let expected_mint_after = Mint {
            base: BaseMint {
                mint_authority: Some(payer.pubkey().to_bytes().into()),
                supply: 0,
                decimals: 9,
                is_initialized: true,
                freeze_authority: None,
            },
            metadata: mint_after.metadata.clone(),
            reserved: mint_after.reserved,
            account_type: mint_after.account_type,
            compression: mint_after.compression,
            extensions: mint_after.extensions.clone(),
        };
        assert_eq!(
            mint_after, expected_mint_after,
            "Mint should match expected after decompression"
        );
    }

    // Verify token metadata extension preserved after decompression
    let extensions_after = mint_after
        .extensions
        .as_ref()
        .expect("Mint should still have extensions after decompression");

    let token_metadata_after = extensions_after
        .iter()
        .find_map(|ext| {
            if let ExtensionStruct::TokenMetadata(tm) = ext {
                Some(tm)
            } else {
                None
            }
        })
        .expect("Mint should still have TokenMetadata extension after decompression");

    assert_eq!(
        token_metadata_after.name, name,
        "Token name should be preserved"
    );
    assert_eq!(
        token_metadata_after.symbol, symbol,
        "Token symbol should be preserved"
    );
    assert_eq!(
        token_metadata_after.uri, uri,
        "Token URI should be preserved"
    );
    assert_eq!(
        token_metadata_after.update_authority, expected_update_authority,
        "Update authority should be preserved"
    );

    let additional_after = &token_metadata_after.additional_metadata;
    assert_eq!(
        additional_after.len(),
        2,
        "Should still have 2 additional metadata entries"
    );
    assert_eq!(additional_after[0].key, b"author".to_vec());
    assert_eq!(additional_after[0].value, b"Light Protocol".to_vec());
    assert_eq!(additional_after[1].key, b"version".to_vec());
    assert_eq!(additional_after[1].value, b"1.0.0".to_vec());
}
