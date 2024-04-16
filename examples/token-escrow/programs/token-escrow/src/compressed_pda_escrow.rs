use anchor_lang::prelude::*;
use light_hasher::{errors::HasherError, DataHasher, Hasher, Poseidon};
use psp_compressed_pda::{
    compressed_account::{derive_address, CompressedAccount, CompressedAccountData},
    utils::CompressedProof,
    InstructionDataTransfer, NewAddressParamsPacked,
};
use psp_compressed_token::{
    CompressedTokenInstructionDataTransfer, InputTokenDataWithContext, TokenTransferOutputData,
};

use crate::{create_change_output_compressed_token_account, EscrowTimeLock};

/// create compressed pda data
/// transfer tokens
/// execute complete transaction
pub fn process_escrow_compressed_tokens_with_compressed_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    lock_up_time: u64,
    escrow_amount: u64,
    proof: Option<CompressedProof>,
    root_indices: Vec<u16>,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_state_merkle_tree_account_indices: Vec<u8>,
    pubkey_array: Vec<Pubkey>,
    new_address_params: NewAddressParamsPacked,
) -> Result<()> {
    let compressed_pda = create_compressed_pda_data(lock_up_time, &ctx, &new_address_params)?;
    let escrow_token_data = TokenTransferOutputData {
        amount: escrow_amount,
        owner: ctx.accounts.cpi_signer.key(),
        lamports: None,
    };
    let change_token_data = create_change_output_compressed_token_account(
        &input_token_data_with_context,
        &[escrow_token_data],
        &ctx.accounts.signer.key(),
    );
    let output_compressed_accounts = vec![escrow_token_data, change_token_data];

    cpi_compressed_token_transfer_pda(
        &ctx,
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
        output_state_merkle_tree_account_indices,
        pubkey_array,
    )?;

    cpi_compressed_pda_transfer(
        &ctx,
        proof,
        root_indices,
        new_address_params,
        compressed_pda,
    )?;
    Ok(())
}

fn cpi_compressed_pda_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    proof: Option<CompressedProof>,
    root_indices: Vec<u16>,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: CompressedAccount,
) -> Result<()> {
    let inputs_struct = InstructionDataTransfer {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: vec![compressed_pda],
        input_root_indices: root_indices,
        output_state_merkle_tree_account_indices: vec![0],
        proof,
        new_address_params: vec![new_address_params],
        compression_lamports: None,
        is_compress: false,
    };

    let mut inputs = Vec::new();
    InstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = psp_compressed_pda::cpi::accounts::TransferInstruction {
        signer: ctx.accounts.signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        psp_account_compression_authority: ctx
            .accounts
            .account_compression_authority
            .to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        cpi_signature_account: None,
        invoking_program: Some(ctx.accounts.self_program.to_account_info()),
        compressed_sol_pda: None,
        compression_recipient: None,
        system_program: None,
    };
    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_pda_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    psp_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs)?;
    Ok(())
}

fn create_compressed_pda_data<'info>(
    lock_up_time: u64,
    ctx: &Context<'_, '_, '_, '_, EscrowCompressedTokensWithCompressedPda<'info>>,
    new_address_params: &NewAddressParamsPacked,
) -> Result<CompressedAccount> {
    let current_slot = Clock::get()?.slot;
    let timelock_compressed_pda = EscrowTimeLock {
        slot: current_slot.checked_add(lock_up_time).unwrap(),
    };
    let compressed_account_data = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: timelock_compressed_pda.try_to_vec().unwrap(),
        data_hash: timelock_compressed_pda.hash().map_err(ProgramError::from)?,
    };
    let derive_address = derive_address(&ctx.remaining_accounts[0].key(), &new_address_params.seed)
        .map_err(|_| ProgramError::InvalidArgument)?;
    Ok(CompressedAccount {
        owner: crate::ID,
        lamports: 0,
        address: Some(derive_address),
        data: Some(compressed_account_data),
    })
}

impl light_hasher::DataHasher for EscrowTimeLock {
    fn hash(&self) -> std::result::Result<[u8; 32], HasherError> {
        Poseidon::hash(&self.slot.to_le_bytes())
    }
}

#[derive(Accounts)]
pub struct EscrowCompressedTokensWithCompressedPda<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(seeds = [b"escrow".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub cpi_signer: AccountInfo<'info>,
    pub compressed_token_program: Program<'info, psp_compressed_token::program::PspCompressedToken>,
    pub compressed_pda_program: Program<'info, psp_compressed_pda::program::PspCompressedPda>,
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    pub account_compression_authority: AccountInfo<'info>,
    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
    pub registered_program_pda: AccountInfo<'info>,
    pub noop_program: AccountInfo<'info>,
    pub self_program: Program<'info, crate::program::TokenEscrow>,
    pub cpi_signature_account: AccountInfo<'info>,
}

#[inline(never)]
pub fn cpi_compressed_token_transfer_pda<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    // proof: Option<CompressedProof>,
    // root_indices: Vec<u16>,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<TokenTransferOutputData>,
    output_state_merkle_tree_account_indices: Vec<u8>,
    pubkey_array: Vec<Pubkey>,
) -> Result<()> {
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: None,
        root_indices: Vec::new(),
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
        output_state_merkle_tree_account_indices,
        pubkey_array,
        is_compress: false,
        compression_amount: None,
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = psp_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        psp_account_compression_authority: ctx
            .accounts
            .account_compression_authority
            .to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        self_program: ctx.accounts.compressed_token_program.to_account_info(),
        cpi_authority_pda: ctx
            .accounts
            .compressed_token_cpi_authority_pda
            .to_account_info(),
        compressed_pda_program: ctx.accounts.compressed_pda_program.to_account_info(),
        token_pool_pda: None,
        decompress_token_account: None,
        token_program: None,
        cpi_signature_account: Some(ctx.accounts.cpi_signature_account.to_account_info()),
    };

    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    psp_compressed_token::cpi::transfer(cpi_ctx, inputs)?;
    Ok(())
}
