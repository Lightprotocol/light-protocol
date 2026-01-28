// Shared test utilities for csdk-anchor-full-derived-test

use light_client::{indexer::Indexer, rpc::Rpc};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

/// Asserts that an account exists on-chain.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `pda` - Account pubkey to check
/// * `name` - Human-readable name for error messages
pub async fn assert_onchain_exists(rpc: &mut (impl Rpc + Indexer), pda: &Pubkey, name: &str) {
    assert!(
        rpc.get_account(*pda).await.unwrap().is_some(),
        "{} account ({}) should exist on-chain",
        name,
        pda
    );
}

/// Asserts that an account is closed (does not exist or has 0 lamports).
///
/// # Arguments
/// * `rpc` - RPC client
/// * `pda` - Account pubkey to check
/// * `name` - Human-readable name for error messages
pub async fn assert_onchain_closed(rpc: &mut (impl Rpc + Indexer), pda: &Pubkey, name: &str) {
    let acc = rpc.get_account(*pda).await.unwrap();
    assert!(
        acc.is_none() || acc.unwrap().lamports == 0,
        "{} account ({}) should be closed",
        name,
        pda
    );
}

/// Asserts that a compressed account exists with non-empty data.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `addr` - Compressed account address
/// * `name` - Human-readable name for error messages
pub async fn assert_compressed_exists_with_data(
    rpc: &mut (impl Rpc + Indexer),
    addr: [u8; 32],
    name: &str,
) {
    let acc = rpc
        .get_compressed_account(addr, None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(
        acc.address.unwrap(),
        addr,
        "{} compressed address mismatch",
        name
    );
    assert!(
        !acc.data.as_ref().unwrap().data.is_empty(),
        "{} compressed account should have non-empty data",
        name
    );
}

/// Asserts that a compressed token account exists with expected amount.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `owner` - Token account owner
/// * `expected_amount` - Expected token amount
/// * `name` - Human-readable name for error messages
pub async fn assert_compressed_token_exists(
    rpc: &mut (impl Rpc + Indexer),
    owner: &Pubkey,
    expected_amount: u64,
    name: &str,
) {
    let accs = rpc
        .get_compressed_token_accounts_by_owner(owner, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        !accs.is_empty(),
        "{} compressed token account should exist for owner {}",
        name,
        owner
    );
    assert_eq!(
        accs[0].token.amount, expected_amount,
        "{} token amount mismatch",
        name
    );
}

/// Asserts that the rent sponsor paid for the created accounts.
///
/// Call this after decompression to verify rent sponsor funded the accounts.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `rent_sponsor` - Rent sponsor PDA pubkey
/// * `rent_sponsor_balance_before` - Balance captured BEFORE the transaction
/// * `created_accounts` - Pubkeys of accounts funded by rent sponsor
pub async fn assert_rent_sponsor_paid_for_accounts(
    rpc: &mut (impl Rpc + Indexer),
    rent_sponsor: &Pubkey,
    rent_sponsor_balance_before: u64,
    created_accounts: &[Pubkey],
) {
    // Get rent sponsor balance after
    let rent_sponsor_balance_after = rpc
        .get_account(*rent_sponsor)
        .await
        .expect("get rent sponsor account")
        .map(|a| a.lamports)
        .unwrap_or(0);

    // Calculate total lamports in created accounts
    let mut total_account_lamports = 0u64;
    for account in created_accounts {
        let account_lamports = rpc
            .get_account(*account)
            .await
            .expect("get created account")
            .map(|a| a.lamports)
            .unwrap_or(0);
        total_account_lamports += account_lamports;
    }

    // Assert rent sponsor paid
    let rent_sponsor_paid = rent_sponsor_balance_before.saturating_sub(rent_sponsor_balance_after);

    assert!(
        rent_sponsor_paid >= total_account_lamports,
        "Rent sponsor should have paid at least {} lamports for accounts, but only paid {}. \
         Before: {}, After: {}",
        total_account_lamports,
        rent_sponsor_paid,
        rent_sponsor_balance_before,
        rent_sponsor_balance_after
    );

    println!(
        "Rent sponsor paid {} lamports for {} accounts (total account balance: {})",
        rent_sponsor_paid,
        created_accounts.len(),
        total_account_lamports
    );
}

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
