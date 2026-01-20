use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use light_token::token::{CompressibleParams, CreateTokenAccount};
use light_token_interface::instructions::extensions::CompressToPubkey;

use crate::Generic;

pub fn process_create_ctoken_with_compress_to_pubkey<'info>(
    ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
    mint: Pubkey,
    token_account_pubkey: Pubkey,
    compressible_config: Pubkey,
    rent_sponsor: Pubkey,
) -> Result<()> {
    let seeds = &[b"compress_target", mint.as_ref()];
    let (_, bump) = Pubkey::find_program_address(seeds, ctx.program_id);

    let compress_to_pubkey = CompressToPubkey {
        bump,
        program_id: ctx.program_id.to_bytes(),
        seeds: vec![b"compress_target".to_vec(), mint.to_bytes().to_vec()],
    };

    let compressible_params = CompressibleParams {
        compressible_config,
        rent_sponsor,
        pre_pay_num_epochs: 2,
        lamports_per_write: None,
        compress_to_account_pubkey: Some(compress_to_pubkey),
        token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compression_only: false,
    };

    let instruction = CreateTokenAccount::new(
        *ctx.accounts.signer.key,
        token_account_pubkey,
        mint,
        *ctx.accounts.signer.key,
    )
    .with_compressible(compressible_params)
    .instruction()?;

    let seeds = [seeds[0], seeds[1], &[bump]];

    invoke_signed(
        &instruction,
        [
            vec![ctx.accounts.signer.to_account_info()],
            ctx.remaining_accounts.to_vec(),
        ]
        .concat()
        .as_slice(),
        &[seeds.as_slice()],
    )?;

    Ok(())
}
