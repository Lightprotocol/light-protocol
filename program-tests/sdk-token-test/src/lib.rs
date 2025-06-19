#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use light_compressed_token_sdk::instructions::Recipient;
use light_compressed_token_sdk::{TokenAccountMeta, ValidityProof};

mod process_compress_tokens;
mod process_transfer_tokens;
mod process_decompress_tokens;
mod process_batch_compress_tokens;

use process_compress_tokens::process_compress_tokens;
use process_transfer_tokens::process_transfer_tokens;
use process_decompress_tokens::process_decompress_tokens;
use process_batch_compress_tokens::process_batch_compress_tokens;

declare_id!("5p1t1GAaKtK1FKCh5Hd2Gu8JCu3eREhJm4Q2qYfTEPYK");

#[program]
pub mod sdk_token_test {
    use super::*;

    pub fn compress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        output_tree_index: u8,
        recipient: Pubkey,
        mint: Pubkey,
        amount: u64,
    ) -> Result<()> {
        process_compress_tokens(ctx, output_tree_index, recipient, mint, amount)
    }

    pub fn transfer_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        validity_proof: ValidityProof,
        token_metas: Vec<TokenAccountMeta>,
        output_tree_index: u8,
        mint: Pubkey,
        recipient: Pubkey,
    ) -> Result<()> {
        process_transfer_tokens(ctx, validity_proof, token_metas, output_tree_index, mint, recipient)
    }

    pub fn decompress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        validity_proof: ValidityProof,
        token_data: Vec<TokenAccountMeta>,
        output_tree_index: u8,
        mint: Pubkey,
    ) -> Result<()> {
        process_decompress_tokens(ctx, validity_proof, token_data, output_tree_index, mint)
    }

    pub fn batch_compress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        recipients: Vec<Recipient>,
        token_pool_index: u8,
        token_pool_bump: u8,
    ) -> Result<()> {
        process_batch_compress_tokens(ctx, recipients, token_pool_index, token_pool_bump)
    }
}

#[derive(Accounts)]
pub struct Generic<'info> {
    // fee payer and authority are the same
    #[account(mut)]
    pub signer: Signer<'info>,
}
