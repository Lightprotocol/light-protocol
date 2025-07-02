use anchor_lang::prelude::borsh::BorshDeserialize;
use light_ctoken_types::{
    instructions::extensions::TokenMetadataInstructionData,
    state::{CompressedMint, ExtensionStruct},
};
use light_hasher::Poseidon;
use solana_sdk::pubkey::Pubkey;

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
            light_ctoken_types::state::extensions::TokenMetadata {
                update_authority: meta.update_authority,
                mint: spl_mint_pda.into(),
                metadata: meta.metadata,
                additional_metadata: meta.additional_metadata.unwrap_or_default(),
                version: meta.version,
            },
        )]
    });

    // Create expected compressed mint for comparison
    let expected_compressed_mint = CompressedMint {
        spl_mint: spl_mint_pda.into(),
        supply: 0,
        decimals,
        is_decompressed: false,
        mint_authority: Some(mint_authority.into()),
        freeze_authority: Some(freeze_authority.into()),
        version: 0,
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
        println!(
            "Compressed Mint extension hash: {:?}",
            extensions[0].hash::<Poseidon>()
        );
    }
    expected_compressed_mint
}
