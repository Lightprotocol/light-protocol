use anchor_lang::prelude::*;
use light_compressed_token::process_transfer::{
    CompressedTokenInstructionDataTransfer, InputTokenDataWithContext,
    PackedTokenTransferOutputData,
};
use light_hasher::{DataHasher, Poseidon};
use light_sdk::verify::verify;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        compressed_account::{
            CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        },
        CompressedCpiContext,
    },
    InstructionDataInvokeCpi, OutputCompressedAccountWithPackedContext,
};

use crate::{
    create_change_output_compressed_token_account, EscrowCompressedTokensWithCompressedPda,
    EscrowError, EscrowTimeLock, PackedInputCompressedPda,
};

pub fn process_withdraw_compressed_tokens_with_compressed_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    withdrawal_amount: u64,
    proof: CompressedProof,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_state_merkle_tree_account_indices: Vec<u8>,
    cpi_context: CompressedCpiContext,
    input_compressed_pda: PackedInputCompressedPda,
    bump: u8,
) -> Result<()> {
    let current_slot = Clock::get()?.slot;
    if current_slot < input_compressed_pda.old_lock_up_time {
        return err!(EscrowError::EscrowLocked);
    }
    let (old_state, new_state) = create_compressed_pda_data_based_on_diff(&input_compressed_pda)?;
    let withdrawal_token_data = PackedTokenTransferOutputData {
        amount: withdrawal_amount,
        owner: ctx.accounts.signer.key(),
        lamports: None,
        merkle_tree_index: output_state_merkle_tree_account_indices[0],
        tlv: None,
    };
    let escrow_change_token_data = create_change_output_compressed_token_account(
        &input_token_data_with_context,
        &[withdrawal_token_data.clone()],
        &ctx.accounts.token_owner_pda.key(),
        output_state_merkle_tree_account_indices[1],
    );
    let output_compressed_accounts = vec![withdrawal_token_data, escrow_change_token_data];
    cpi_compressed_token_withdrawal(
        &ctx,
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
        proof.clone(),
        bump,
        cpi_context,
    )?;

    cpi_compressed_pda_withdrawal(ctx, proof, old_state, new_state, cpi_context, bump)?;
    Ok(())
}

fn create_compressed_pda_data_based_on_diff(
    input_compressed_pda: &PackedInputCompressedPda,
) -> Result<(
    PackedCompressedAccountWithMerkleContext,
    OutputCompressedAccountWithPackedContext,
)> {
    let current_slot = Clock::get()?.slot;

    let old_timelock_compressed_pda = EscrowTimeLock {
        slot: input_compressed_pda.old_lock_up_time,
    };
    let old_compressed_account_data = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: old_timelock_compressed_pda.try_to_vec().unwrap(),
        data_hash: old_timelock_compressed_pda
            .hash::<Poseidon>()
            .map_err(ProgramError::from)?,
    };
    let old_compressed_account = OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            owner: crate::ID,
            lamports: 0,
            address: Some(input_compressed_pda.address),
            data: Some(old_compressed_account_data),
        },
        merkle_tree_index: input_compressed_pda.merkle_context.merkle_tree_pubkey_index,
    };
    let old_compressed_account_with_context = PackedCompressedAccountWithMerkleContext {
        compressed_account: old_compressed_account.compressed_account,
        merkle_context: input_compressed_pda.merkle_context,
        root_index: input_compressed_pda.root_index,
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
            .hash::<Poseidon>()
            .map_err(ProgramError::from)?,
    };
    let new_state = OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            owner: crate::ID,
            lamports: 0,
            address: Some(input_compressed_pda.address),
            data: Some(new_compressed_account_data),
        },
        merkle_tree_index: input_compressed_pda.merkle_context.merkle_tree_pubkey_index,
    };
    Ok((old_compressed_account_with_context, new_state))
}

fn cpi_compressed_pda_withdrawal<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    proof: CompressedProof,
    old_state: PackedCompressedAccountWithMerkleContext,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    mut cpi_context: CompressedCpiContext,
    bump: u8,
) -> Result<()> {
    // Create CPI signer seed
    let bump_seed = &[bump];
    let signer_key_bytes = ctx.accounts.signer.key.to_bytes();
    let signer_seeds = [&b"escrow"[..], &signer_key_bytes[..], bump_seed];
    cpi_context.first_set_context = false;

    // Create CPI inputs
    let inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: vec![old_state],
        output_compressed_accounts: vec![compressed_pda],
        proof: Some(proof),
        new_address_params: Vec::new(),
        compress_or_decompress_lamports: None,
        is_compress: false,
        signer_seeds: signer_seeds.iter().map(|seed| seed.to_vec()).collect(),
        cpi_context: Some(cpi_context),
    };

    verify(ctx, &inputs_struct, &[&signer_seeds])?;

    Ok(())
}

// TODO: test with delegate (is disabled right now)
#[inline(never)]
pub fn cpi_compressed_token_withdrawal<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    mint: Pubkey,
    _signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
    proof: CompressedProof,
    bump: u8,
    mut cpi_context: CompressedCpiContext,
) -> Result<()> {
    let bump = &[bump];
    let signer_bytes = ctx.accounts.signer.key.to_bytes();
    let seeds: [&[u8]; 3] = [b"escrow".as_slice(), signer_bytes.as_slice(), bump];
    cpi_context.set_context = true;

    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: Some(proof),
        mint,
        delegated_transfer: None,
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress: false,
        compress_or_decompress_amount: None,
        cpi_context: Some(cpi_context),
        lamports_change_account_merkle_tree_index: None,
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.token_owner_pda.to_account_info(),
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
    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    light_compressed_token::cpi::transfer(cpi_ctx, inputs)?;
    Ok(())
}
