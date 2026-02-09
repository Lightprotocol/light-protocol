// Shared test utilities for sdk-light-token-test

use light_client::{indexer::Indexer, rpc::Rpc};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// Setup helper: Creates a compressed mint directly using the ctoken SDK (not via wrapper program)
/// Optionally creates ATAs and mints tokens for each recipient.
/// Note: This decompresses the mint first, then uses MintTo to mint to ctoken accounts.
/// Returns (mint_pda, compression_address, ata_pubkeys, mint_seed_keypair)
#[allow(unused)]
pub async fn setup_create_mint(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    decimals: u8,
    recipients: Vec<(u64, Pubkey)>,
) -> (Pubkey, [u8; 32], Vec<Pubkey>, Keypair) {
    use light_token::instruction::{
        CreateAssociatedTokenAccount, CreateMint, CreateMintParams, MintTo,
    };

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_token::instruction::derive_mint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let (mint, bump) = light_token::instruction::find_mint_address(&mint_seed.pubkey());

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
        freeze_authority: None,
        extensions: None,
        rent_payment: 16,
        write_top_up: 766,
    };

    // Create instruction directly using SDK
    let create_mint_builder = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_mint_builder.instruction().unwrap();

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

    // If no recipients, return early
    if recipients.is_empty() {
        return (mint, compression_address, vec![], mint_seed);
    }

    // Create ATAs for each recipient
    use light_token::instruction::derive_token_ata;

    let mut ata_pubkeys = Vec::with_capacity(recipients.len());

    for (_amount, owner) in &recipients {
        let ata_address = derive_token_ata(owner, &mint);
        ata_pubkeys.push(ata_address);

        let create_ata = CreateAssociatedTokenAccount::new(payer.pubkey(), *owner, mint);
        let ata_instruction = create_ata.instruction().unwrap();

        rpc.create_and_send_transaction(&[ata_instruction], &payer.pubkey(), &[payer])
            .await
            .unwrap();
    }

    // Mint tokens to recipients with amount > 0
    let recipients_with_amount: Vec<_> = recipients
        .iter()
        .enumerate()
        .filter(|(_, (amount, _))| *amount > 0)
        .collect();

    // Mint to each recipient using the decompressed Mint (CreateMint already decompresses)
    for (idx, (amount, _)) in &recipients_with_amount {
        let mint_instruction = MintTo {
            mint,
            destination: ata_pubkeys[*idx],
            amount: *amount,
            authority: mint_authority,
            max_top_up: None,
            fee_payer: None,
        }
        .instruction()
        .unwrap();

        rpc.create_and_send_transaction(&[mint_instruction], &payer.pubkey(), &[payer])
            .await
            .unwrap();
    }

    (mint, compression_address, ata_pubkeys, mint_seed)
}

/// Same as setup_create_mint but with optional freeze_authority
/// Returns (mint_pda, compression_address, ata_pubkeys)
#[allow(unused)]
pub async fn setup_create_mint_with_freeze_authority(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    decimals: u8,
    recipients: Vec<(u64, Pubkey)>,
) -> (Pubkey, [u8; 32], Vec<Pubkey>) {
    use light_token::instruction::{
        CreateAssociatedTokenAccount, CreateMint, CreateMintParams, MintTo,
    };

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_token::instruction::derive_mint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let (mint, bump) = light_token::instruction::find_mint_address(&mint_seed.pubkey());

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
    let create_mint_builder = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_mint_builder.instruction().unwrap();

    // Send transaction (CreateMint now creates both compressed mint AND Mint Solana account)
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    // If no recipients, return early
    if recipients.is_empty() {
        return (mint, compression_address, vec![]);
    }

    // Create ATAs for each recipient
    use light_token::instruction::derive_token_ata;

    let mut ata_pubkeys = Vec::with_capacity(recipients.len());

    for (_amount, owner) in &recipients {
        let ata_address = derive_token_ata(owner, &mint);
        ata_pubkeys.push(ata_address);

        let create_ata = CreateAssociatedTokenAccount::new(payer.pubkey(), *owner, mint);
        let ata_instruction = create_ata.instruction().unwrap();

        rpc.create_and_send_transaction(&[ata_instruction], &payer.pubkey(), &[payer])
            .await
            .unwrap();
    }

    // After decompression, use MintTo (simple 3-account instruction)
    let recipients_with_amount: Vec<_> = recipients
        .iter()
        .enumerate()
        .filter(|(_, (amount, _))| *amount > 0)
        .collect();

    if !recipients_with_amount.is_empty() {
        for (idx, (amount, _)) in &recipients_with_amount {
            let mint_instruction = MintTo {
                mint,
                destination: ata_pubkeys[*idx],
                amount: *amount,
                authority: mint_authority,
                max_top_up: None,
                fee_payer: None,
            }
            .instruction()
            .unwrap();

            rpc.create_and_send_transaction(&[mint_instruction], &payer.pubkey(), &[payer])
                .await
                .unwrap();
        }
    }

    (mint, compression_address, ata_pubkeys)
}

/// Same as setup_create_mint but with compression_only flag set
#[allow(unused)]
pub async fn setup_create_mint_with_compression_only(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    decimals: u8,
    recipients: Vec<(u64, Pubkey)>,
    compression_only: bool,
) -> (Pubkey, [u8; 32], Vec<Pubkey>) {
    use light_token::instruction::{
        CompressibleParams, CreateAssociatedTokenAccount, CreateMint, CreateMintParams, MintTo,
    };

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_token::instruction::derive_mint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let (mint, bump) = light_token::instruction::find_mint_address(&mint_seed.pubkey());

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
        freeze_authority: None,
        extensions: None,
        rent_payment: 16,
        write_top_up: 766,
    };

    // Create instruction directly using SDK
    let create_mint_builder = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_mint_builder.instruction().unwrap();

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

    // If no recipients, return early
    if recipients.is_empty() {
        return (mint, compression_address, vec![]);
    }

    // Create ATAs for each recipient with custom compression_only setting
    use light_token::instruction::derive_token_ata;

    let mut ata_pubkeys = Vec::with_capacity(recipients.len());

    // Build custom CompressibleParams with compression_only flag
    let compressible_params = CompressibleParams {
        compression_only,
        ..Default::default()
    };

    for (_amount, owner) in &recipients {
        let ata_address = derive_token_ata(owner, &mint);
        ata_pubkeys.push(ata_address);

        let create_ata = CreateAssociatedTokenAccount::new(payer.pubkey(), *owner, mint)
            .with_compressible(compressible_params.clone());
        let ata_instruction = create_ata.instruction().unwrap();

        rpc.create_and_send_transaction(&[ata_instruction], &payer.pubkey(), &[payer])
            .await
            .unwrap();
    }

    // Mint tokens to recipients with amount > 0
    let recipients_with_amount: Vec<_> = recipients
        .iter()
        .enumerate()
        .filter(|(_, (amount, _))| *amount > 0)
        .collect();

    // Mint to each recipient using the decompressed Mint (CreateMint already decompresses)
    for (idx, (amount, _)) in &recipients_with_amount {
        let mint_instruction = MintTo {
            mint,
            destination: ata_pubkeys[*idx],
            amount: *amount,
            authority: mint_authority,
            max_top_up: None,
            fee_payer: None,
        }
        .instruction()
        .unwrap();

        rpc.create_and_send_transaction(&[mint_instruction], &payer.pubkey(), &[payer])
            .await
            .unwrap();
    }

    (mint, compression_address, ata_pubkeys)
}

/// Creates a compressed-only mint (no decompression) using light-token-client.
/// This creates ONLY the compressed mint account, NOT the Mint Solana account.
/// Use this to test the DecompressMint instruction.
/// Returns (mint_pda, compression_address, mint_seed_keypair)
#[allow(unused)]
pub async fn setup_create_compressed_only_mint(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    decimals: u8,
) -> (Pubkey, [u8; 32], Keypair) {
    use light_test_utils::actions::legacy::instructions::mint_action::{
        create_mint_action_instruction, MintActionParams, NewMint,
    };
    use light_token::instruction::{derive_mint_compressed_address, find_mint_address};

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();

    // Derive addresses
    let compression_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree.tree);
    let (mint_pda, _bump) = find_mint_address(&mint_seed.pubkey());

    // Create compressed-only mint using light-token-client
    // By NOT including DecompressMint action, only the compressed mint is created
    let create_ix = create_mint_action_instruction(
        rpc,
        MintActionParams {
            compressed_mint_address: compression_address,
            mint_seed: mint_seed.pubkey(),
            authority: mint_authority,
            payer: payer.pubkey(),
            actions: vec![], // No actions - just create compressed mint
            new_mint: Some(NewMint {
                decimals,
                supply: 0,
                mint_authority,
                freeze_authority: None,
                metadata: None,
                version: 3,
            }),
        },
    )
    .await
    .unwrap();

    // Send transaction - mint_seed must sign as mint_signer
    rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    // Verify compressed mint was created
    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;
    assert!(
        compressed_account.is_some(),
        "Compressed mint should exist after creation"
    );

    // Verify NO Mint Solana account exists
    let mint_account = rpc.get_account(mint_pda).await.unwrap();
    assert!(
        mint_account.is_none(),
        "Mint Solana account should NOT exist for compressed-only mint"
    );

    (mint_pda, compression_address, mint_seed)
}
