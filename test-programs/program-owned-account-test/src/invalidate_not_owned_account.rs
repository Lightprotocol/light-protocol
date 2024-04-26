use anchor_lang::prelude::*;
use light_compressed_pda::{
    compressed_account::CompressedAccountWithMerkleContext, compressed_cpi::CompressedCpiContext,
    CompressedProof, InstructionDataTransfer,
};

/// create compressed pda data
/// transfer tokens
/// execute complete transaction
pub fn process_invalidate_not_owned_compressed_account<'info>(
    ctx: Context<'_, '_, '_, 'info, InvalidateNotOwnedCompressedAccount<'info>>,
    compressed_account: CompressedAccountWithMerkleContext,
    proof: Option<CompressedProof>,
    root_indices: Vec<u16>,
    bump: u8,
) -> Result<()> {
    let seeds: [&[u8]; 2] = [b"cpi_signer".as_slice(), &[bump]];
    let inputs_struct = InstructionDataTransfer {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: vec![compressed_account],
        output_compressed_accounts: Vec::new(),
        input_root_indices: root_indices,
        output_state_merkle_tree_account_indices: Vec::new(),
        proof,
        new_address_params: Vec::new(),
        compression_lamports: None,
        is_compress: false,
        signer_seeds: Some(seeds.iter().map(|seed| seed.to_vec()).collect()),
    };
    let cpi_context = CompressedCpiContext {
        execute: true,
        cpi_signature_account_index: (ctx.remaining_accounts.len() - 1) as u8,
    };
    let mut inputs = Vec::new();
    InstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_compressed_pda::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.cpi_signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: Some(ctx.accounts.self_program.to_account_info()),
        compressed_sol_pda: None,
        compression_recipient: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_signature_account: None,
    };
    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_pda_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    light_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs, Some(cpi_context))?;
    Ok(())
}

#[derive(Accounts)]
pub struct InvalidateNotOwnedCompressedAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub compressed_pda_program: Program<'info, light_compressed_pda::program::LightCompressedPda>,
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    /// CHECK:
    pub account_compression_authority: AccountInfo<'info>,
    /// CHECK:
    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK:
    pub noop_program: AccountInfo<'info>,
    pub self_program: Program<'info, crate::program::ProgramOwnedAccountTest>,
    /// CHECK:
    pub cpi_signer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
