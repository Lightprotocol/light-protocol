use anchor_lang::prelude::borsh::BorshDeserialize;
use light_token_interface::{
    instructions::extensions::TokenMetadataInstructionData,
    state::{BaseMint, ExtensionStruct, Mint, MintMetadata, ACCOUNT_TYPE_MINT},
};
use solana_sdk::pubkey::Pubkey;

use crate::assert_metadata::assert_sha_account_hash;

#[track_caller]
pub fn assert_compressed_mint_account(
    compressed_mint_account: &light_client::indexer::CompressedAccount,
    compressed_mint_address: [u8; 32],
    spl_mint_pda: Pubkey,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Pubkey,
    metadata: Option<TokenMetadataInstructionData>,
) -> Mint {
    // Derive mint_signer from spl_mint_pda by reversing the PDA derivation
    // We need to find the mint_signer and bump used to create spl_mint_pda
    // spl_mint_pda = PDA([COMPRESSED_MINT_SEED, mint_signer], program_id)
    // Since we can't reverse this, we extract it from the actual compressed mint data
    let compressed_account_data = compressed_mint_account.data.clone().unwrap();
    let actual_compressed_mint: Mint =
        BorshDeserialize::deserialize(&mut compressed_account_data.data.as_slice()).unwrap();
    let mint_signer = actual_compressed_mint.metadata.mint_signer;
    let bump = actual_compressed_mint.metadata.bump;

    // Create expected extensions if metadata is provided
    let expected_extensions = metadata.map(|meta| {
        vec![ExtensionStruct::TokenMetadata(
            light_token_interface::state::extensions::TokenMetadata {
                update_authority: meta
                    .update_authority
                    .unwrap_or_else(|| Pubkey::from([0u8; 32]).into()),
                mint: spl_mint_pda.into(),
                name: meta.name,
                symbol: meta.symbol,
                uri: meta.uri,
                additional_metadata: meta.additional_metadata.unwrap_or_default(),
            },
        )]
    });

    // Create expected compressed mint for comparison
    let expected_compressed_mint = Mint {
        base: BaseMint {
            mint_authority: Some(mint_authority.into()),
            supply: 0,
            decimals,
            is_initialized: true,
            freeze_authority: Some(freeze_authority.into()),
        },
        metadata: MintMetadata {
            version: 3,
            mint: spl_mint_pda.into(),
            mint_decompressed: false,
            mint_signer,
            bump,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression: light_compressible::compression_info::CompressionInfo::default(),
        extensions: expected_extensions,
    };

    // Verify the account exists and has correct properties
    assert_eq!(
        compressed_mint_account.address.unwrap(),
        compressed_mint_address
    );
    assert_eq!(compressed_mint_account.owner, light_compressed_token::ID);
    assert_eq!(compressed_mint_account.lamports, 0);

    // Verify the compressed mint data
    let compressed_account_data = compressed_mint_account.data.clone().unwrap();
    assert_eq!(
        compressed_account_data.discriminator,
        light_compressed_token::constants::COMPRESSED_MINT_DISCRIMINATOR
    );

    // Deserialize and verify the Mint struct matches expected
    let compressed_mint: Mint =
        BorshDeserialize::deserialize(&mut compressed_account_data.data.as_slice()).unwrap();
    println!("Compressed Mint: {:?}", compressed_mint);
    assert_eq!(compressed_mint, expected_compressed_mint);
    if let Some(extensions) = compressed_mint.extensions {
        println!("Compressed Mint extensions: {:?}", extensions);
    }
    assert_sha_account_hash(compressed_mint_account).unwrap();

    expected_compressed_mint
}

/// Assert that the mint creation fee (50,000 lamports) was charged.
/// Compares rent_sponsor balance before and after mint creation.
#[track_caller]
pub fn assert_mint_creation_fee(
    rent_sponsor_lamports_before: u64,
    rent_sponsor_lamports_after: u64,
) {
    assert_eq!(
        rent_sponsor_lamports_after,
        rent_sponsor_lamports_before + light_compressed_token::MINT_CREATION_FEE,
        "Rent sponsor should receive {} lamports mint creation fee",
        light_compressed_token::MINT_CREATION_FEE,
    );
}
