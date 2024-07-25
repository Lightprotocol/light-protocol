use crate::create_change_output_compressed_token_account;
use account_compression::{program::AccountCompression, RegisteredProgram};
use anchor_lang::prelude::*;
use light_compressed_token::{
    process_transfer::{
        CompressedTokenInstructionDataTransfer, InputTokenDataWithContext,
        PackedTokenTransferOutputData,
    },
    program::LightCompressedToken,
};
use light_sdk::traits::*;
use light_sdk::LightTraits;
use light_system_program::{
    invoke::processor::CompressedProof, invoke_cpi::account::CpiContextAccount,
    program::LightSystemProgram,
};

#[derive(Accounts, LightTraits)]
pub struct EscrowCompressedTokensWithPda<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    /// CHECK:
    #[authority]
    #[account(seeds = [b"escrow".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub token_owner_pda: AccountInfo<'info>,
    #[self_program]
    pub compressed_token_program: Program<'info, LightCompressedToken>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK:
    pub account_compression_authority: AccountInfo<'info>,
    /// CHECK:
    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
    /// CHECK:
    pub registered_program_pda: Account<'info, RegisteredProgram>,
    /// CHECK:
    pub noop_program: AccountInfo<'info>,
    #[account(init_if_needed, seeds = [b"timelock".as_slice(), signer.key.to_bytes().as_slice()],bump, payer = signer, space = 8 + 8)]
    pub timelock_pda: Account<'info, EscrowTimeLock>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug)]
#[account]
pub struct EscrowTimeLock {
    pub slot: u64,
}

pub fn process_escrow_compressed_tokens_with_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
    lock_up_time: u64,
    escrow_amount: u64,
    proof: CompressedProof,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_state_merkle_tree_account_indices: Vec<u8>,
) -> Result<()> {
    // set timelock
    let current_slot = Clock::get()?.slot;
    ctx.accounts.timelock_pda.slot = current_slot.checked_add(lock_up_time).unwrap();

    let escrow_token_data = PackedTokenTransferOutputData {
        amount: escrow_amount,
        owner: ctx.accounts.token_owner_pda.key(),
        lamports: None,
        merkle_tree_index: output_state_merkle_tree_account_indices[0],
        tlv: None,
    };
    let change_token_data = create_change_output_compressed_token_account(
        &input_token_data_with_context,
        &[escrow_token_data.clone()],
        &ctx.accounts.signer.key(),
        output_state_merkle_tree_account_indices[1],
    );
    let output_compressed_accounts = vec![escrow_token_data, change_token_data];

    cpi_compressed_token_transfer(
        &ctx,
        proof,
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
    )
}

#[inline(never)]
pub fn cpi_compressed_token_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
    proof: CompressedProof,
    mint: Pubkey,
    _signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
) -> Result<()> {
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: Some(proof),
        mint,
        delegated_transfer: None,
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress: false,
        compress_or_decompress_amount: None,
        cpi_context: None,
        lamports_change_account_merkle_tree_index: None,
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        self_program: ctx.accounts.compressed_token_program.to_account_info(),
        cpi_authority_pda: ctx
            .accounts
            .compressed_token_cpi_authority_pda
            .to_account_info(),
        light_system_program: ctx.accounts.light_system_program.to_account_info(),
        token_pool_pda: None,
        compress_or_decompress_token_account: None,
        token_program: None,
        system_program: ctx.accounts.system_program.to_account_info(),
    };

    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    light_compressed_token::cpi::transfer(cpi_ctx, inputs)?;
    Ok(())
}
