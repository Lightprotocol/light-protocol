use anchor_lang::prelude::*;
use light_hasher::{errors::HasherError, DataHasher, Hasher, Poseidon};
use psp_compressed_pda::{
    compressed_account::{
        derive_address, CompressedAccount, CompressedAccountData,
        CompressedAccountWithMerkleContext,
    },
    compressed_cpi::CompressedCpiContext,
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
    cpi_context: CompressedCpiContext,
) -> Result<()> {
    let compressed_pda = create_compressed_pda_data(lock_up_time, &ctx, &new_address_params)?;
    let escrow_token_data = TokenTransferOutputData {
        amount: escrow_amount,
        owner: ctx.accounts.token_owner_pda.key(),
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
        root_indices,
        proof.clone(),
        &cpi_context,
    )?;
    msg!("escrow compressed tokens with compressed pda");
    cpi_compressed_pda_transfer(&ctx, proof, new_address_params, compressed_pda, cpi_context)?;
    Ok(())
}

fn cpi_compressed_pda_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: CompressedAccount,
    cpi_context: CompressedCpiContext,
) -> Result<()> {
    let inputs_struct = InstructionDataTransfer {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: vec![compressed_pda],
        input_root_indices: Vec::new(),
        output_state_merkle_tree_account_indices: vec![0],
        proof,
        new_address_params: vec![new_address_params],
        compression_lamports: None,
        is_compress: false,
        signer_seeds: None,
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
        invoking_program: Some(ctx.accounts.self_program.to_account_info()),
        compressed_sol_pda: None,
        compression_recipient: None,
        system_program: None,
        cpi_signature_account: Some(
            ctx.remaining_accounts[cpi_context.cpi_signature_account_index as usize]
                .to_account_info(),
        ),
    };
    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_pda_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    psp_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs, Some(cpi_context))?;
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
    let derive_address = derive_address(
        &ctx.remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
            .key(),
        &new_address_params.seed,
    )
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
    /// CHECK:
    #[account(seeds = [b"escrow".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub token_owner_pda: AccountInfo<'info>,
    pub compressed_token_program: Program<'info, psp_compressed_token::program::PspCompressedToken>,
    pub compressed_pda_program: Program<'info, psp_compressed_pda::program::PspCompressedPda>,
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
    pub self_program: Program<'info, crate::program::TokenEscrow>,
}

// TODO: add functionality to deposit into an existing escrow account
#[inline(never)]
pub fn cpi_compressed_token_transfer_pda<'info>(
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
) -> Result<()> {
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
    };

    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    let cpi_context = CompressedCpiContext {
        cpi_signature_account_index: cpi_context.cpi_signature_account_index,
        execute: false,
    };
    psp_compressed_token::cpi::transfer(cpi_ctx, inputs, Some(cpi_context))?;
    Ok(())
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct MerkleContext {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub leaf_index: u32,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedInputCompressedPda {
    pub old_lock_up_time: u64,
    pub new_lock_up_time: u64,
    pub address: [u8; 32],
    pub merkle_context: PackedMerkleContext,
}

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
    msg!("escrow compressed tokens with compressed pda");
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

fn create_compressed_pda_data_based_on_diff<'info>(
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
    msg!("signer: {:?}", ctx.accounts.signer.key());
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
