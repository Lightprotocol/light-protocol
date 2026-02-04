use anchor_lang::prelude::*;
use light_token::{
    instruction::{create_mints, CreateMintsParams as SdkCreateMintsParams, SingleMintParams},
    CompressedProof,
};

/// Parameters for a single mint within a batch creation.
/// Does not include proof since proof is shared across all mints.
/// `mint` and `compression_address` are derived internally from `mint_seed_pubkey`.
#[derive(Clone, AnchorSerialize, AnchorDeserialize, Debug)]
pub struct MintParams {
    pub decimals: u8,
    pub mint_authority: Pubkey,
    /// Optional mint bump. If `None`, derived from `find_mint_address(mint_seed_pubkey)`.
    pub mint_bump: Option<u8>,
    pub freeze_authority: Option<Pubkey>,
    pub mint_seed_pubkey: Pubkey,
}

/// Parameters for creating one or more compressed mints with decompression.
///
/// Creates N compressed mints and decompresses all to Solana Mint accounts.
/// Uses CPI context pattern when N > 1 for efficiency.
///
/// Flow:
/// - N=1: Single CPI (create + decompress)
/// - N>1: 2N-1 CPIs (N-1 writes + 1 execute with decompress + N-1 decompress)
#[derive(Clone, AnchorSerialize, AnchorDeserialize, Debug)]
pub struct CreateMintsParams {
    /// Parameters for each mint to create
    pub mints: Vec<MintParams>,
    /// Single proof covering all new addresses
    pub proof: CompressedProof,
    /// Address merkle tree root index (shared across all mints in batch)
    pub address_merkle_tree_root_index: u16,
}

impl CreateMintsParams {
    pub fn new(mints: Vec<MintParams>, proof: CompressedProof, root_index: u16) -> Self {
        Self {
            mints,
            proof,
            address_merkle_tree_root_index: root_index,
        }
    }
}

/// Anchor instruction wrapper for create_mints.
pub fn process_create_mints<'a, 'info>(
    ctx: Context<'a, '_, 'info, 'info, crate::Generic<'info>>,
    params: CreateMintsParams,
) -> Result<()> {
    // Convert anchor types to SDK types
    let sdk_mints: Vec<SingleMintParams<'_>> = params
        .mints
        .iter()
        .map(|m| SingleMintParams {
            decimals: m.decimals,
            mint_authority: solana_pubkey::Pubkey::new_from_array(m.mint_authority.to_bytes()),
            mint_bump: m.mint_bump,
            freeze_authority: m
                .freeze_authority
                .map(|a| solana_pubkey::Pubkey::new_from_array(a.to_bytes())),
            mint_seed_pubkey: solana_pubkey::Pubkey::new_from_array(m.mint_seed_pubkey.to_bytes()),
            authority_seeds: None,
            mint_signer_seeds: None,
            token_metadata: None,
        })
        .collect();

    let sdk_params = SdkCreateMintsParams::new(
        &sdk_mints,
        params.proof,
        params.address_merkle_tree_root_index,
    );

    let payer = ctx.accounts.signer.to_account_info();
    create_mints(&payer, ctx.remaining_accounts, sdk_params)
        .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;

    Ok(())
}
