use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use light_compressed_token_sdk::instructions::create_token_account::{
    create_compressible_token_account, CreateCompressibleTokenAccount,
};
use light_ctoken_types::instructions::extensions::compressible::CompressToPubkey;

use crate::Generic;

pub fn process_create_ctoken_with_compress_to_pubkey<'info>(
    ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
    mint: Pubkey,
    token_account_pubkey: Pubkey,
    compressible_config: Pubkey,
    rent_sponsor: Pubkey,
) -> Result<()> {
    // Derive the PDA that tokens will compress to
    let seeds = &[b"compress_target", mint.as_ref()];
    let (_, bump) = Pubkey::find_program_address(seeds, ctx.program_id);

    // Build the CompressToPubkey struct
    let compress_to_pubkey = CompressToPubkey {
        bump,
        program_id: ctx.program_id.to_bytes(),
        seeds: vec![b"compress_target".to_vec(), mint.to_bytes().to_vec()],
    };

    // Create the instruction to create a compressible token account
    let create_account_inputs = CreateCompressibleTokenAccount {
        payer: *ctx.accounts.signer.key,
        account_pubkey: token_account_pubkey,
        mint_pubkey: mint,
        owner_pubkey: *ctx.accounts.signer.key, // Owner is the signer
        compressible_config,
        rent_sponsor,
        pre_pay_num_epochs: 2,    // Pre-pay for 2 epochs
        lamports_per_write: None, // No additional top-up
        compress_to_account_pubkey: Some(compress_to_pubkey),
        token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
    };

    let instruction =
        create_compressible_token_account(create_account_inputs).map_err(ProgramError::from)?;

    let seeds = [seeds[0], seeds[1], &[bump]];

    // The instruction expects the accounts in the exact order they were added to remaining_accounts
    // The test already provides all accounts in the correct order
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
