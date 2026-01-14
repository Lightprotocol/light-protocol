use anchor_lang::prelude::borsh::BorshDeserialize;
use light_token_interface::{
    instructions::extensions::TokenMetadataInstructionData,
    state::{BaseMint, CompressedMint, CompressedMintMetadata, ExtensionStruct, ACCOUNT_TYPE_MINT},
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
) -> CompressedMint {
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
    let expected_compressed_mint = CompressedMint {
        base: BaseMint {
            mint_authority: Some(mint_authority.into()),
            supply: 0,
            decimals,
            is_initialized: true,
            freeze_authority: Some(freeze_authority.into()),
        },
        metadata: CompressedMintMetadata {
            version: 3,
            mint: spl_mint_pda.into(),
            cmint_decompressed: false,
            compressed_address: compressed_mint_address,
        },
        reserved: [0u8; 17],
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

    // Deserialize and verify the CompressedMint struct matches expected
    let compressed_mint: CompressedMint =
        BorshDeserialize::deserialize(&mut compressed_account_data.data.as_slice()).unwrap();
    println!("Compressed Mint: {:?}", compressed_mint);
    assert_eq!(compressed_mint, expected_compressed_mint);
    if let Some(extensions) = compressed_mint.extensions {
        println!("Compressed Mint extensions: {:?}", extensions);
    }
    assert_sha_account_hash(compressed_mint_account).unwrap();

    expected_compressed_mint
}
