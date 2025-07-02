use anchor_lang::prelude::borsh::BorshDeserialize;
use anchor_spl::token_2022::spl_token_2022;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_token::{
    instructions::create_token_pool::find_token_pool_pda_with_index, LIGHT_CPI_SIGNER,
};
use light_compressed_token_sdk::instructions::{
    derive_compressed_mint_address, find_spl_mint_address,
};
use light_ctoken_types::state::CompressedMint;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};

/// Assert that:
/// 1. compressed mint is marked as decompressed and didn't change otherwise
/// 2. spl mint is initialized and equivalent with the compressed mint
/// 3. if supply exists has been minted to the token pool
pub async fn assert_spl_mint<R: Rpc + Indexer>(
    rpc: &mut R,
    seed: Pubkey,
    pre_compressed_mint: &CompressedMint,
) {
    // Derive all necessary addresses from the seed
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address = derive_compressed_mint_address(&seed, &address_tree_pubkey);
    let (spl_mint_pda, _) = find_spl_mint_address(&seed);

    // Get the compressed mint data
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await
        .expect("Failed to get compressed mint account")
        .value;

    let compressed_mint: CompressedMint = BorshDeserialize::deserialize(
        &mut compressed_mint_account
            .data
            .as_ref()
            .expect("Compressed mint should have data")
            .data
            .as_slice(),
    )
    .expect("Failed to deserialize compressed mint");

    let mut expected_compressed_mint = (*pre_compressed_mint).clone();
    expected_compressed_mint.is_decompressed = true;
    assert_eq!(compressed_mint, expected_compressed_mint);

    // 2. Assert SPL mint is initialized and equivalent with compressed mint
    {
        let mint_account_data = rpc
            .get_account(spl_mint_pda)
            .await
            .expect("Failed to get SPL mint account")
            .expect("SPL mint account should exist");

        let actual_spl_mint = spl_token_2022::state::Mint::unpack(&mint_account_data.data)
            .expect("Failed to unpack SPL mint data");

        // Create expected SPL mint struct
        let expected_spl_mint = spl_token_2022::state::Mint {
            mint_authority: actual_spl_mint.mint_authority, // Copy the actual COption value
            supply: compressed_mint.supply,
            decimals: compressed_mint.decimals,
            is_initialized: true,
            freeze_authority: actual_spl_mint.freeze_authority, // Copy the actual COption value
        };

        assert_eq!(actual_spl_mint, expected_spl_mint);
    }
    // 3. If supply > 0, assert token pool has the supply
    if compressed_mint.supply > 0 {
        let (token_pool_pda, _) = find_token_pool_pda_with_index(&spl_mint_pda, 0);
        let token_pool_account_data = rpc
            .get_account(token_pool_pda)
            .await
            .expect("Failed to get token pool account")
            .expect("Token pool account should exist");

        let actual_token_pool =
            spl_token_2022::state::Account::unpack(&token_pool_account_data.data)
                .expect("Failed to unpack token pool data");

        // Create expected token pool struct
        let expected_token_pool = spl_token_2022::state::Account {
            mint: spl_mint_pda,
            owner: LIGHT_CPI_SIGNER.cpi_signer.into(),
            amount: compressed_mint.supply,
            delegate: actual_token_pool.delegate, // Copy the actual COption value
            state: spl_token_2022::state::AccountState::Initialized,
            is_native: actual_token_pool.is_native, // Copy the actual COption value
            delegated_amount: 0,
            close_authority: actual_token_pool.close_authority, // Copy the actual COption value
        };

        assert_eq!(actual_token_pool, expected_token_pool);
    }
}
