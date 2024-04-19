use anchor_lang::prelude::*;
use light_hasher::DataHasher;
use psp_compressed_pda::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext,
    },
    compressed_cpi::CompressedCpiContext,
    utils::CompressedProof,
    InstructionDataTransfer,
};
use psp_compressed_token::{
    CompressedTokenInstructionDataTransfer, InputTokenDataWithContext, TokenTransferOutputData,
};

use crate::{
    create_change_output_compressed_token_account, EscrowCompressedTokensWithCompressedPda,
    EscrowError, EscrowTimeLock, PackedInputCompressedPda,
};

pub fn process_withdraw_compressed_tokens_with_compressed_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    withdrawal_amount: u64,
    proof: Option<CompressedProof>,
    root_indices: Vec<u16>,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_state_merkle_tree_account_indices: Vec<u8>,
    pubkey_array: Vec<Pubkey>,
    cpi_context: CompressedCpiContext,
    input_compressed_pda: PackedInputCompressedPda,
    bump: u8,
) -> Result<()> {
    let current_slot = Clock::get()?.slot;
    if current_slot < input_compressed_pda.old_lock_up_time {
        return err!(EscrowError::EscrowLocked);
    }
    let (old_state, new_state) = create_compressed_pda_data_based_on_diff(&input_compressed_pda)?;
    let withdrawal_token_data = TokenTransferOutputData {
        amount: withdrawal_amount,
        owner: ctx.accounts.token_owner_pda.key(),
        lamports: None,
    };
    let escrow_change_token_data = create_change_output_compressed_token_account(
        &input_token_data_with_context,
        &[withdrawal_token_data],
        &ctx.accounts.signer.key(),
    );
    let output_compressed_accounts = vec![withdrawal_token_data, escrow_change_token_data];
    cpi_compressed_token_withdrawal(
        &ctx,
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
        output_state_merkle_tree_account_indices,
        pubkey_array,
        vec![root_indices[1]],
        proof.clone(),
        &cpi_context,
        bump,
    )?;

    cpi_compressed_pda_withdrawal(
        &ctx,
        proof,
        old_state,
        new_state,
        cpi_context,
        vec![root_indices[0]],
        bump,
    )?;
    Ok(())
}

fn create_compressed_pda_data_based_on_diff(
    input_compressed_pda: &PackedInputCompressedPda,
) -> Result<(CompressedAccountWithMerkleContext, CompressedAccount)> {
    let current_slot = Clock::get()?.slot;

    let old_timelock_compressed_pda = EscrowTimeLock {
        slot: input_compressed_pda.old_lock_up_time,
    };
    let old_compressed_account_data = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: old_timelock_compressed_pda.try_to_vec().unwrap(),
        data_hash: old_timelock_compressed_pda
            .hash()
            .map_err(ProgramError::from)?,
    };
    println!(
        "old_compressed_account_data: {:?}",
        old_compressed_account_data.data_hash
    );
    println!(
        "old_compressed_account_data: {:?}",
        old_compressed_account_data.data_hash
    );
    let old_compressed_account = CompressedAccount {
        owner: crate::ID,
        lamports: 0,
        address: Some(input_compressed_pda.address),
        data: Some(old_compressed_account_data),
    };
    let old_compressed_account_with_context = CompressedAccountWithMerkleContext {
        compressed_account: old_compressed_account,
        merkle_tree_pubkey_index: input_compressed_pda.merkle_context.merkle_tree_pubkey_index,
        nullifier_queue_pubkey_index: input_compressed_pda
            .merkle_context
            .nullifier_queue_pubkey_index,
        leaf_index: input_compressed_pda.merkle_context.leaf_index,
    };
    let new_timelock_compressed_pda = EscrowTimeLock {
        slot: current_slot
            .checked_add(input_compressed_pda.new_lock_up_time)
            .unwrap(),
    };
    let new_compressed_account_data = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: new_timelock_compressed_pda.try_to_vec().unwrap(),
        data_hash: new_timelock_compressed_pda
            .hash()
            .map_err(ProgramError::from)?,
    };
    let new_state = CompressedAccount {
        owner: crate::ID,
        lamports: 0,
        address: Some(input_compressed_pda.address),
        data: Some(new_compressed_account_data),
    };
    Ok((old_compressed_account_with_context, new_state))
}

fn cpi_compressed_pda_withdrawal<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    proof: Option<CompressedProof>,
    old_state: CompressedAccountWithMerkleContext,
    compressed_pda: CompressedAccount,
    cpi_context: CompressedCpiContext,
    root_indices: Vec<u16>,
    bump: u8,
) -> Result<()> {
    let bump = &[bump];
    let signer_bytes = ctx.accounts.signer.key.to_bytes();
    let seeds: [&[u8]; 3] = [b"escrow".as_slice(), signer_bytes.as_slice(), bump];
    let inputs_struct = InstructionDataTransfer {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: vec![old_state],
        output_compressed_accounts: vec![compressed_pda],
        input_root_indices: root_indices,
        output_state_merkle_tree_account_indices: vec![0],
        proof,
        new_address_params: Vec::new(),
        compression_lamports: None,
        is_compress: false,
        signer_seeds: Some(seeds.iter().map(|seed| seed.to_vec()).collect()),
    };

    let mut inputs = Vec::new();
    InstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = psp_compressed_pda::cpi::accounts::TransferInstruction {
        signer: ctx.accounts.token_owner_pda.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        psp_account_compression_authority: ctx
            .accounts
            .account_compression_authority
            .to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: Some(ctx.accounts.self_program.to_account_info()),
        compressed_sol_pda: None,
        compression_recipient: None,
        system_program: None,
        cpi_signature_account: Some(
            ctx.remaining_accounts[cpi_context.cpi_signature_account_index as usize]
                .to_account_info(),
        ),
    };
    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_pda_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    psp_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs, Some(cpi_context))?;
    Ok(())
}

#[inline(never)]
pub fn cpi_compressed_token_withdrawal<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<TokenTransferOutputData>,
    output_state_merkle_tree_account_indices: Vec<u8>,
    pubkey_array: Vec<Pubkey>,
    root_indices: Vec<u16>,
    proof: Option<CompressedProof>,
    cpi_context: &CompressedCpiContext,
    bump: u8,
) -> Result<()> {
    let bump = &[bump];
    let signer_bytes = ctx.accounts.signer.key.to_bytes();
    let seeds: [&[u8]; 3] = [b"escrow".as_slice(), signer_bytes.as_slice(), bump];
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof,
        root_indices,
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
        fee_payer: ctx.accounts.token_owner_pda.to_account_info(),
        authority: ctx.accounts.token_owner_pda.to_account_info(),
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
    };
    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    let cpi_context = CompressedCpiContext {
        cpi_signature_account_index: cpi_context.cpi_signature_account_index,
        execute: false,
    };
    psp_compressed_token::cpi::transfer(cpi_ctx, inputs, Some(cpi_context))?;
    Ok(())
}
