use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_client::indexer::Indexer;
use light_compressed_account::{address::derive_address, hash_to_bn254_field_size_be};
use light_compressed_token_sdk::{
    instructions::{
        create_compressed_mint::find_spl_mint_address, derive_compressed_mint_address,
        mint_action::MintToRecipient,
    },
    CPI_AUTHORITY_PDA,
};
use light_ctoken_types::{
    instructions::{
        extensions::token_metadata::TokenMetadataInstructionData,
        mint_action::{CompressedMintInstructionData, CompressedMintWithContext},
    },
    state::{extensions::AdditionalMetadata, CompressedMintMetadata},
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_program_test::{LightProgramTest, ProgramTestConfig, Rpc, RpcError};
use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
use sdk_token_test::{ChainedCtokenInstructionData, PdaCreationData, ID};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_ctoken_pda() {
    // Initialize test environment
    let config = ProgramTestConfig::new_v2(false, Some(vec![("sdk_token_test", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Test parameters
    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new();
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = mint_authority; // Same as mint authority for this example
    let mint_seed = Keypair::new();

    // Token metadata
    let token_name = "Test Compressed Token".to_string();
    let token_symbol = "TCT".to_string();
    let token_uri = "https://example.com/test-token.json".to_string();

    // Create token metadata extension
    let additional_metadata = vec![
        AdditionalMetadata {
            key: b"created_by".to_vec(),
            value: b"ctoken-minter".to_vec(),
        },
        AdditionalMetadata {
            key: b"example".to_vec(),
            value: b"program-examples".to_vec(),
        },
    ];

    let token_metadata = TokenMetadataInstructionData {
        update_authority: Some(mint_authority.into()),
        name: token_name.clone().into_bytes(),
        symbol: token_symbol.clone().into_bytes(),
        uri: token_uri.clone().into_bytes(),
        additional_metadata: Some(additional_metadata),
    };

    // Create the compressed mint (with chained operations including update mint)
    let (compressed_mint_address, _spl_mint) = create_mint(
        &mut rpc,
        &mint_seed,
        decimals,
        &mint_authority_keypair,
        Some(freeze_authority),
        Some(token_metadata),
        &payer,
    )
    .await
    .unwrap();
    let all_accounts = rpc
        .get_compressed_accounts_by_owner(&sdk_token_test::ID, None, None)
        .await
        .unwrap()
        .value;
    println!("All accounts: {:?}", all_accounts);

    let mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value
        .ok_or("Mint account not found")
        .unwrap();

    // Verify the chained CPI operations worked correctly
    println!("🧪 Verifying chained CPI results...");

    // 1. Verify compressed mint was created and mint authority was revoked
    let compressed_mint = light_ctoken_types::state::CompressedMint::deserialize(
        &mut &mint_account.data.as_ref().unwrap().data[..],
    )
    .unwrap();

    println!("✅ Compressed mint created:");
    println!("   - SPL mint: {:?}", compressed_mint.metadata.mint);
    println!("   - Decimals: {}", compressed_mint.base.decimals);
    println!("   - Supply: {}", compressed_mint.base.supply);
    println!(
        "   - Mint authority: {:?}",
        compressed_mint.base.mint_authority
    );
    println!(
        "   - Freeze authority: {:?}",
        compressed_mint.base.freeze_authority
    );

    // Assert mint authority was revoked (should be None after update)
    assert_eq!(
        compressed_mint.base.mint_authority, None,
        "Mint authority should be revoked (None)"
    );
    assert_eq!(
        compressed_mint.base.supply, 1000u64,
        "Supply should be 1000 after minting"
    );
    assert_eq!(
        compressed_mint.base.decimals, decimals,
        "Decimals should match"
    );

    println!("🎉 All chained CPI operations completed successfully!");
    println!("   1. ✅ Created compressed mint with mint authority");
    println!("   2. ✅ Minted 1000 tokens to payer");
    println!("   3. ✅ Revoked mint authority (set to None)");
    println!("   4. ✅ Created escrow PDA");
}

pub async fn create_mint<R: Rpc + Indexer>(
    rpc: &mut R,
    mint_seed: &Keypair,
    decimals: u8,
    mint_authority: &Keypair,
    freeze_authority: Option<Pubkey>,
    metadata: Option<TokenMetadataInstructionData>,
    payer: &Keypair,
) -> Result<([u8; 32], Pubkey), RpcError> {
    // Get address tree and output queue from RPC
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let tree_info = rpc.get_random_state_tree_info()?;

    // Derive compressed mint address using utility function
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint bump for the instruction
    let (mint, mint_bump) = find_spl_mint_address(&mint_seed.pubkey());

    let pda_address_seed = hash_to_bn254_field_size_be(
        [b"escrow", payer.pubkey().to_bytes().as_ref()]
            .concat()
            .as_slice(),
    );
    println!("mint: {:?}", mint);
    let pda_address = derive_address(
        &pda_address_seed,
        &address_tree_pubkey.to_bytes(),
        &ID.to_bytes(),
    );
    // Get validity proof for address creation
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![
                light_client::indexer::AddressWithTree {
                    address: compressed_mint_address,
                    tree: address_tree_pubkey,
                },
                light_client::indexer::AddressWithTree {
                    address: pda_address, // is first, because we execute the cpi context with this ix
                    tree: address_tree_pubkey,
                },
            ],
            None,
        )
        .await?
        .value;
    let mut packed_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new_with_cpi_context(ID, tree_info.cpi_context.unwrap());
    packed_accounts.add_system_accounts_v2(config).unwrap();
    // packed_accounts.insert_or_get(tree_info.get_output_pubkey()?);
    rpc_result.pack_tree_infos(&mut packed_accounts);

    // Create PDA parameters
    let pda_amount = 100u64;

    // Create consolidated instruction data using new optimized structure
    let compressed_mint_with_context = CompressedMintWithContext {
        leaf_index: 0,
        prove_by_index: false,
        root_index: rpc_result.addresses[0].root_index,
        address: compressed_mint_address,
        mint: CompressedMintInstructionData {
            supply: 0,
            decimals,
            metadata: CompressedMintMetadata {
                version: 3,
                mint: mint.into(),
                spl_mint_initialized: false,
            },
            mint_authority: Some(mint_authority.pubkey().into()),
            freeze_authority: freeze_authority.map(|fa| fa.into()),
            extensions: metadata.map(|m| vec![light_ctoken_types::instructions::extensions::ExtensionInstructionData::TokenMetadata(m)]),
        },
    };

    let token_recipients = vec![MintToRecipient {
        recipient: payer.pubkey(),
        amount: 1000u64, // Mint 1000 tokens
    }];

    let pda_creation = PdaCreationData {
        amount: pda_amount,
        address: pda_address,
        proof: rpc_result.proof,
    };
    // Create Anchor accounts struct
    let accounts = sdk_token_test::accounts::CTokenPda {
        payer: payer.pubkey(),
        mint_authority: mint_authority.pubkey(),
        mint_seed: mint_seed.pubkey(),
        ctoken_program: Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
        ctoken_cpi_authority: Pubkey::new_from_array(CPI_AUTHORITY_PDA),
    };

    let pda_new_address_params = light_sdk::address::NewAddressParamsAssignedPacked {
        seed: pda_address_seed,
        address_queue_account_index: 0,
        address_merkle_tree_account_index: 0,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        assigned_account_index: 0,
        assigned_to_account: true,
    };
    let output_tree_index = packed_accounts.insert_or_get(tree_info.get_output_pubkey().unwrap());
    let tree_index = packed_accounts.insert_or_get(tree_info.tree);
    assert_eq!(output_tree_index, 1);
    assert_eq!(tree_index, 2);
    let remaining_accounts = packed_accounts.to_account_metas().0;

    // Create the consolidated instruction data
    let instruction_data = sdk_token_test::instruction::CtokenPda {
        input: ChainedCtokenInstructionData {
            compressed_mint_with_context,
            mint_bump,
            token_recipients,
            final_mint_authority: None, // Revoke mint authority (set to None)
            pda_creation,
            output_tree_index,
            new_address_params: pda_new_address_params,
        },
    };
    let ix = solana_sdk::instruction::Instruction {
        program_id: ID,
        accounts: [accounts.to_account_metas(None), remaining_accounts].concat(),
        data: instruction_data.data(),
    };
    println!("ix {:?}", ix);
    // Determine signers (deduplicate if mint_signer and payer are the same)
    let mut signers = vec![payer, mint_authority];
    if mint_seed.pubkey() != payer.pubkey() {
        signers.push(mint_seed);
    }

    // TODO: pass indices for address tree and output queue so that we can define them in the cpi context invocation
    // Send the transaction
    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
        .await?;

    // Return the compressed mint address, token account, and SPL mint
    Ok((compressed_mint_address, mint))
}
