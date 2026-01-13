// Shared test utilities for sdk-light-token-test

use borsh::BorshDeserialize;
use light_client::{indexer::Indexer, rpc::Rpc};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// Setup helper: Creates a compressed mint directly using the ctoken SDK (not via wrapper program)
/// Optionally creates ATAs and mints tokens for each recipient.
/// Returns (mint_pda, compression_address, ata_pubkeys, mint_seed_keypair)
#[allow(unused)]
pub async fn setup_create_compressed_mint(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    decimals: u8,
    recipients: Vec<(u64, Pubkey)>,
) -> (Pubkey, [u8; 32], Vec<Pubkey>, Keypair) {
    use light_token_sdk::token::{
        CreateAssociatedTokenAccount, CreateCMint, CreateCMintParams, MintTo, MintToParams,
    };

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_token_sdk::token::derive_cmint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let mint = light_token_sdk::token::find_cmint_address(&mint_seed.pubkey()).0;

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
        freeze_authority: None,
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

    // If no recipients, return early
    if recipients.is_empty() {
        return (mint, compression_address, vec![], mint_seed);
    }

    // Create ATAs for each recipient
    use light_token_sdk::token::derive_token_ata;

    let mut ata_pubkeys = Vec::with_capacity(recipients.len());

    for (_amount, owner) in &recipients {
        let (ata_address, _bump) = derive_token_ata(owner, &mint);
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

    if !recipients_with_amount.is_empty() {
        // Get the compressed mint account for minting
        let compressed_mint_account = rpc
            .get_compressed_account(compression_address, None)
            .await
            .unwrap()
            .value
            .expect("Compressed mint should exist");

        use light_token_interface::state::CompressedMint;
        let compressed_mint =
            CompressedMint::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
                .unwrap();

        // Get validity proof for the mint operation
        let rpc_result = rpc
            .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
            .await
            .unwrap()
            .value;

        // Build CompressedMintWithContext
        use light_token_interface::instructions::mint_action::CompressedMintWithContext;
        let compressed_mint_with_context = CompressedMintWithContext {
            address: compression_address,
            leaf_index: compressed_mint_account.leaf_index,
            prove_by_index: true,
            root_index: rpc_result.accounts[0]
                .root_index
                .root_index()
                .unwrap_or_default(),
            mint: Some(compressed_mint.try_into().unwrap()),
        };

        // Build mint params with first recipient
        let (first_idx, (first_amount, _)) = recipients_with_amount[0];
        let mut mint_params = MintToParams::new(
            compressed_mint_with_context,
            *first_amount,
            mint_authority,
            rpc_result.proof,
        );
        // Override the account_index for the first action
        mint_params.mint_to_actions[0].account_index = first_idx as u8;

        // Add remaining recipients
        for (idx, (amount, _)) in recipients_with_amount.iter().skip(1) {
            mint_params = mint_params.add_mint_to_action(*idx as u8, *amount);
        }

        // Build MintToToken instruction
        let mint_to_ctoken = MintTo::new(
            mint_params,
            payer.pubkey(),
            compressed_mint_account.tree_info.tree,
            compressed_mint_account.tree_info.queue,
            compressed_mint_account.tree_info.queue,
            ata_pubkeys.clone(),
        );
        let mint_instruction = mint_to_ctoken.instruction().unwrap();

        rpc.create_and_send_transaction(&[mint_instruction], &payer.pubkey(), &[payer])
            .await
            .unwrap();
    }

    (mint, compression_address, ata_pubkeys, mint_seed)
}

/// Same as setup_create_compressed_mint but with optional freeze_authority
/// Returns (mint_pda, compression_address, ata_pubkeys)
#[allow(unused)]
pub async fn setup_create_compressed_mint_with_freeze_authority(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    decimals: u8,
    recipients: Vec<(u64, Pubkey)>,
) -> (Pubkey, [u8; 32], Vec<Pubkey>) {
    use light_token_sdk::token::{
        CreateAssociatedTokenAccount, CreateCMint, CreateCMintParams, MintTo, MintToParams,
    };

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_token_sdk::token::derive_cmint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let mint = light_token_sdk::token::find_cmint_address(&mint_seed.pubkey()).0;

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

    // Verify the compressed mint was created and get it for decompression
    let compressed_mint_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value
        .expect("Compressed mint should exist after setup");

    // Decompress the mint to create an on-chain CMint account
    // This is required for freeze/thaw operations which need to read the mint
    {
        use light_token_interface::{
            instructions::mint_action::CompressedMintWithContext, state::CompressedMint,
        };
        use light_token_sdk::token::DecompressCMint;

        let compressed_mint = CompressedMint::deserialize(
            &mut compressed_mint_account
                .data
                .as_ref()
                .unwrap()
                .data
                .as_slice(),
        )
        .unwrap();

        // Get validity proof for the decompress operation
        let rpc_result = rpc
            .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
            .await
            .unwrap()
            .value;

        let compressed_mint_with_context = CompressedMintWithContext {
            address: compression_address,
            leaf_index: compressed_mint_account.leaf_index,
            prove_by_index: true,
            root_index: rpc_result.accounts[0]
                .root_index
                .root_index()
                .unwrap_or_default(),
            mint: Some(compressed_mint.try_into().unwrap()),
        };

        let decompress_ix = DecompressCMint {
            mint_seed_pubkey: mint_seed.pubkey(),
            payer: payer.pubkey(),
            authority: mint_authority,
            state_tree: compressed_mint_account.tree_info.tree,
            input_queue: compressed_mint_account.tree_info.queue,
            output_queue,
            compressed_mint_with_context,
            proof: rpc_result.proof,
            rent_payment: 16,  // ~24 hours rent (16 epochs * 1.5h per epoch)
            write_top_up: 766, // ~3 hours per write
        }
        .instruction()
        .unwrap();

        rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[payer])
            .await
            .unwrap();
    }

    // If no recipients, return early
    if recipients.is_empty() {
        return (mint, compression_address, vec![]);
    }

    // Create ATAs for each recipient
    use light_token_sdk::token::derive_token_ata;

    let mut ata_pubkeys = Vec::with_capacity(recipients.len());

    for (_amount, owner) in &recipients {
        let (ata_address, _bump) = derive_token_ata(owner, &mint);
        ata_pubkeys.push(ata_address);

        let create_ata = CreateAssociatedTokenAccount::new(payer.pubkey(), *owner, mint);
        let ata_instruction = create_ata.instruction().unwrap();

        rpc.create_and_send_transaction(&[ata_instruction], &payer.pubkey(), &[payer])
            .await
            .unwrap();
    }

    // After decompression, use TokenMintTo (simple 3-account instruction)
    // instead of MintToToken (which uses compressed mint)
    let recipients_with_amount: Vec<_> = recipients
        .iter()
        .enumerate()
        .filter(|(_, (amount, _))| *amount > 0)
        .collect();

    if !recipients_with_amount.is_empty() {
        use light_token_sdk::token::TokenMintTo;

        for (idx, (amount, _)) in &recipients_with_amount {
            let mint_instruction = TokenMintTo {
                cmint: mint,
                destination: ata_pubkeys[*idx],
                amount: *amount,
                authority: mint_authority,
                max_top_up: None,
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

/// Same as setup_create_compressed_mint but with compression_only flag set
#[allow(unused)]
pub async fn setup_create_compressed_mint_with_compression_only(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    decimals: u8,
    recipients: Vec<(u64, Pubkey)>,
    compression_only: bool,
) -> (Pubkey, [u8; 32], Vec<Pubkey>) {
    use light_token_sdk::token::{
        CompressibleParams, CreateAssociatedTokenAccount, CreateCMint, CreateCMintParams,
        MintToToken, MintToTokenParams,
    };

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_token_sdk::token::derive_cmint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let mint = light_token_sdk::token::find_cmint_address(&mint_seed.pubkey()).0;

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
        freeze_authority: None,
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

    // If no recipients, return early
    if recipients.is_empty() {
        return (mint, compression_address, vec![]);
    }

    // Create ATAs for each recipient with custom compression_only setting
    use light_token_sdk::token::derive_token_ata;

    let mut ata_pubkeys = Vec::with_capacity(recipients.len());

    // Build custom CompressibleParams with compression_only flag
    let compressible_params = CompressibleParams {
        compression_only,
        ..Default::default()
    };

    for (_amount, owner) in &recipients {
        let (ata_address, _bump) = derive_token_ata(owner, &mint);
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

    if !recipients_with_amount.is_empty() {
        // Get the compressed mint account for minting
        let compressed_mint_account = rpc
            .get_compressed_account(compression_address, None)
            .await
            .unwrap()
            .value
            .expect("Compressed mint should exist");

        use light_token_interface::state::CompressedMint;
        let compressed_mint =
            CompressedMint::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
                .unwrap();

        // Get validity proof for the mint operation
        let rpc_result = rpc
            .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
            .await
            .unwrap()
            .value;

        // Build CompressedMintWithContext
        use light_token_interface::instructions::mint_action::CompressedMintWithContext;
        let compressed_mint_with_context = CompressedMintWithContext {
            address: compression_address,
            leaf_index: compressed_mint_account.leaf_index,
            prove_by_index: true,
            root_index: rpc_result.accounts[0]
                .root_index
                .root_index()
                .unwrap_or_default(),
            mint: Some(compressed_mint.try_into().unwrap()),
        };

        // Build mint params with first recipient
        let (first_idx, (first_amount, _)) = recipients_with_amount[0];
        let mut mint_params = MintToParams::new(
            compressed_mint_with_context,
            *first_amount,
            mint_authority,
            rpc_result.proof,
        );
        // Override the account_index for the first action
        mint_params.mint_to_actions[0].account_index = first_idx as u8;

        // Add remaining recipients
        for (idx, (amount, _)) in recipients_with_amount.iter().skip(1) {
            mint_params = mint_params.add_mint_to_action(*idx as u8, *amount);
        }

        // Build MintToToken instruction
        let mint_to_ctoken = MintTo::new(
            mint_params,
            payer.pubkey(),
            compressed_mint_account.tree_info.tree,
            compressed_mint_account.tree_info.queue,
            compressed_mint_account.tree_info.queue,
            ata_pubkeys.clone(),
        );
        let mint_instruction = mint_to_ctoken.instruction().unwrap();

        rpc.create_and_send_transaction(&[mint_instruction], &payer.pubkey(), &[payer])
            .await
            .unwrap();
    }

    (mint, compression_address, ata_pubkeys)
}
