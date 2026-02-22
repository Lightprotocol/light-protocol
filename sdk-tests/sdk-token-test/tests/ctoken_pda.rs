use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::indexer::Indexer;
use light_compressed_account::{address::derive_address, hash_to_bn254_field_size_be};
use light_compressed_token_sdk::compressed_token::create_compressed_mint::{
    derive_mint_compressed_address, find_mint_address,
};
use light_program_test::{
    utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig, Rpc, RpcError,
};
use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
use light_token_interface::{
    instructions::{
        extensions::token_metadata::TokenMetadataInstructionData,
        mint_action::{MintInstructionData, MintWithContext, Recipient},
    },
    state::{extensions::AdditionalMetadata, MintMetadata},
    LIGHT_TOKEN_PROGRAM_ID,
};
use light_token_types::CPI_AUTHORITY_PDA;
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

    // create_mint + write_to_cpi_context is rejected because mint creation
    // charges a fee requiring the rent_sponsor account, which is not available
    // in the CPI context write path (error 6035: CpiContextSetNotUsable).
    let result = create_mint(
        &mut rpc,
        &mint_seed,
        decimals,
        &mint_authority_keypair,
        Some(freeze_authority),
        Some(token_metadata),
        &payer,
    )
    .await;

    assert_rpc_error(result, 0, 6035).unwrap();
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
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint bump for the instruction
    let (mint, mint_bump) = find_mint_address(&mint_seed.pubkey());

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
    let compressed_mint_with_context = MintWithContext {
        leaf_index: 0,
        prove_by_index: false,
        root_index: rpc_result.addresses[0].root_index,
        address: compressed_mint_address,
        mint: Some(MintInstructionData {
            supply: 0,
            decimals,
            metadata: MintMetadata {
                version: 3,
                mint: mint.into(),
                mint_decompressed: false,
                mint_signer: mint_seed.pubkey().to_bytes(),
                bump: mint_bump,
            },
            mint_authority: Some(mint_authority.pubkey().into()),
            freeze_authority: freeze_authority.map(|fa| fa.into()),
            extensions: metadata.map(|m| vec![light_token_interface::instructions::extensions::ExtensionInstructionData::TokenMetadata(m)]),
        }),
    };

    let token_recipients = vec![Recipient::new(
        payer.pubkey(),
        1000u64, // Mint 1000 tokens
    )];

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
        light_token_program: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        light_token_cpi_authority: Pubkey::new_from_array(CPI_AUTHORITY_PDA),
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
