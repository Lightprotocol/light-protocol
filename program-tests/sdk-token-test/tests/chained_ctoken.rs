use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_client::indexer::Indexer;
use light_compressed_token_sdk::{
    instructions::{create_compressed_mint::find_spl_mint_address, derive_compressed_mint_address},
    CPI_AUTHORITY_PDA,
};

use light_ctoken_types::{
    instructions::{
        extensions::token_metadata::TokenMetadataInstructionData, mint_to_compressed::Recipient,
        update_compressed_mint::CompressedMintAuthorityType,
    },
    state::extensions::{AdditionalMetadata, Metadata},
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_program_test::{LightProgramTest, ProgramTestConfig, Rpc, RpcError};

use light_compressed_account::{address::derive_address, hash_to_bn254_field_size_be};
use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
use sdk_token_test::{
    processor::{
        CreateCompressedMintInstructionData, MintToCompressedInstructionData,
        UpdateCompressedMintInstructionDataCpi,
    },
    ID,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_ctoken_minter() {
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
        metadata: Metadata {
            name: token_name.clone().into_bytes(),
            symbol: token_symbol.clone().into_bytes(),
            uri: token_uri.clone().into_bytes(),
        },
        additional_metadata: Some(additional_metadata),
        version: 1, // Poseidon hash version
    };

    // Create the compressed mint (with chained operations including update mint)
    let compressed_mint_address = create_mint(
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
        .value;

    // Verify the chained CPI operations worked correctly
    println!("🧪 Verifying chained CPI results...");

    // 1. Verify compressed mint was created and mint authority was revoked
    let compressed_mint = light_ctoken_types::state::CompressedMint::deserialize(
        &mut &mint_account.data.as_ref().unwrap().data[..],
    )
    .unwrap();

    println!("✅ Compressed mint created:");
    println!("   - SPL mint: {:?}", compressed_mint.spl_mint);
    println!("   - Decimals: {}", compressed_mint.decimals);
    println!("   - Supply: {}", compressed_mint.supply);
    println!("   - Mint authority: {:?}", compressed_mint.mint_authority);
    println!(
        "   - Freeze authority: {:?}",
        compressed_mint.freeze_authority
    );

    // Assert mint authority was revoked (should be None after update)
    assert_eq!(
        compressed_mint.mint_authority, None,
        "Mint authority should be revoked (None)"
    );
    assert_eq!(
        compressed_mint.supply, 1000u64,
        "Supply should be 1000 after minting"
    );
    assert_eq!(compressed_mint.decimals, decimals, "Decimals should match");

    // 2. Verify tokens were minted to the payer
    let token_accounts = rpc
        .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    println!("✅ Tokens minted:");
    println!("   - Token accounts found: {}", token_accounts.len());
    assert!(
        !token_accounts.is_empty(),
        "Should have minted tokens to payer"
    );

    let token_account = &token_accounts[0];
    println!("   - Token amount: {}", token_account.token.amount);
    println!("   - Token mint: {:?}", token_account.token.mint);
    assert_eq!(
        token_account.token.amount, 1000u64,
        "Token amount should be 1000"
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
) -> Result<[u8; 32], RpcError> {
    // Get address tree and output queue from RPC
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let tree_info = rpc.get_random_state_tree_info()?;

    // Derive compressed mint address using utility function
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint bump for the instruction
    let (spl_mint, mint_bump) = find_spl_mint_address(&mint_seed.pubkey());
    let pda_address_seed = hash_to_bn254_field_size_be(
        [b"escrow", payer.pubkey().to_bytes().as_ref()]
            .concat()
            .as_slice(),
    );
    println!("spl_mint: {:?}", spl_mint);
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
    let config = SystemAccountMetaConfig {
        cpi_context: tree_info.cpi_context,
        self_program: ID,
        ..Default::default()
    };
    packed_accounts.add_system_accounts_small(config).unwrap();
    rpc_result.pack_tree_infos(&mut packed_accounts);
    // Create instruction data for the ctoken-minter program
    let inputs = CreateCompressedMintInstructionData {
        decimals,
        freeze_authority,
        mint_bump,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        version: 1,
        metadata,
        compressed_mint_address,
    };

    // Create mint_to_compressed instruction data
    let mint_inputs = MintToCompressedInstructionData {
        recipients: vec![Recipient {
            recipient: payer.pubkey().into(),
            amount: 1000u64, // Mint 1000 tokens
        }],
        lamports: None,
        version: 2,
    };

    // Create update_compressed_mint instruction data (revoke mint authority)
    let update_mint_inputs = UpdateCompressedMintInstructionDataCpi {
        authority_type: CompressedMintAuthorityType::MintTokens,
        new_authority: None, // Revoke mint authority (set to None)
        mint_authority: Some(mint_authority.pubkey()), // Current mint authority needed for validation
    };
    // Create Anchor accounts struct
    let accounts = sdk_token_test::accounts::CreateCompressedMint {
        payer: payer.pubkey(),
        mint_authority: mint_authority.pubkey(),
        mint_seed: mint_seed.pubkey(),
        ctoken_program: Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
        ctoken_cpi_authority: Pubkey::new_from_array(CPI_AUTHORITY_PDA),
    };

    // Create PDA parameters
    let pda_amount = 100u64;

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

    // Create the instruction
    let instruction_data = sdk_token_test::instruction::ChainedCtoken {
        inputs,
        mint_inputs,
        update_mint_inputs,
        pda_proof: rpc_result.proof,
        output_tree_index,
        amount: pda_amount,
        address: pda_address,
        new_address_params: pda_new_address_params,
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

    // Return the compressed mint address
    Ok(compressed_mint_address)
}
