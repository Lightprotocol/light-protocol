use anchor_lang::prelude::*;
use light_compressed_token::{
    CompressedTokenInstructionDataTransfer, InputTokenDataWithContext,
    PackedTokenTransferOutputData,
};
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        compressed_account::{CompressedAccount, PackedCompressedAccountWithMerkleContext},
        CompressedCpiContext,
    },
    InstructionDataInvokeCpi, OutputCompressedAccountWithPackedContext,
};

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub enum WithInputAccountsMode {
    NotOwnedCompressedAccount,
    CpiContextMissing,
    CpiContextAccountMissing,
    CpiContextProofMismatch,
    CpiContextEmpty,
    CpiContextInvalidInvokingProgram,
    CpiContextInvalidSignerSeeds,
    CpiContextWriteAccessCheckFailed,
    CpiContextWriteToNotOwnedAccount,
}

// TODO: Functional tests with cpi context:
// - ability to store multiple accounts in cpi context account and combine them successfully (can check with multiple token program invocations)
pub fn process_with_input_accounts<'info>(
    ctx: Context<'_, '_, '_, 'info, InvalidateNotOwnedCompressedAccount<'info>>,
    compressed_account: PackedCompressedAccountWithMerkleContext,
    proof: Option<CompressedProof>,
    bump: u8,
    mode: WithInputAccountsMode,
    cpi_context: Option<CompressedCpiContext>,
    token_transfer_data: Option<TokenTransferData>,
) -> Result<()> {
    match mode {
        WithInputAccountsMode::NotOwnedCompressedAccount => {
            process_invalidate_not_owned_compressed_account(
                &ctx,
                compressed_account,
                proof,
                bump,
                mode,
                cpi_context,
            )
        }
        WithInputAccountsMode::CpiContextMissing
        | WithInputAccountsMode::CpiContextAccountMissing
        | WithInputAccountsMode::CpiContextEmpty
        | WithInputAccountsMode::CpiContextInvalidInvokingProgram
        | WithInputAccountsMode::CpiContextInvalidSignerSeeds
        | WithInputAccountsMode::CpiContextWriteToNotOwnedAccount => {
            process_invalidate_not_owned_compressed_account(
                &ctx,
                compressed_account,
                proof,
                bump,
                mode,
                cpi_context,
            )
        }
        WithInputAccountsMode::CpiContextProofMismatch => cpi_context_tx(
            &ctx,
            compressed_account,
            proof.unwrap(),
            bump,
            token_transfer_data.unwrap(),
            mode,
            cpi_context.unwrap(),
        ),
        _ => panic!("Invalid mode"),
    }
}

/// create compressed pda data
/// transfer tokens
/// execute complete transaction
pub fn process_invalidate_not_owned_compressed_account<'info>(
    ctx: &Context<'_, '_, '_, 'info, InvalidateNotOwnedCompressedAccount<'info>>,
    compressed_account: PackedCompressedAccountWithMerkleContext,
    proof: Option<CompressedProof>,
    bump: u8,
    mode: WithInputAccountsMode,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    let cpi_context_account = cpi_context.map(|cpi_context| {
        ctx.remaining_accounts
            .get(cpi_context.cpi_context_account_index as usize)
            .unwrap()
            .to_account_info()
    });
    let cpi_context = match mode {
        WithInputAccountsMode::CpiContextMissing => None,
        WithInputAccountsMode::CpiContextEmpty | WithInputAccountsMode::CpiContextProofMismatch => {
            Some(CompressedCpiContext {
                cpi_context_account_index: cpi_context.unwrap().cpi_context_account_index,
                set_context: false,
            })
        }
        _ => cpi_context,
    };
    let cpi_context_account = match mode {
        WithInputAccountsMode::CpiContextAccountMissing => None,
        _ => cpi_context_account,
    };
    let output_compressed_accounts = match mode {
        WithInputAccountsMode::CpiContextWriteToNotOwnedAccount => {
            vec![OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    data: compressed_account.compressed_account.data.clone(),
                    owner: light_compressed_token::ID,
                    lamports: 0,
                    address: compressed_account.compressed_account.address,
                },
                merkle_tree_index: 0,
            }]
        }
        _ => Vec::new(),
    };
    let invoking_program = match mode {
        WithInputAccountsMode::CpiContextInvalidInvokingProgram => {
            ctx.accounts.signer.to_account_info()
        }
        _ => ctx.accounts.self_program.to_account_info(),
    };
    let signer_seed = match mode {
        WithInputAccountsMode::CpiContextInvalidSignerSeeds => b"cpi_signer1".as_slice(),
        _ => b"cpi_signer".as_slice(),
    };
    let local_bump = Pubkey::find_program_address(&[signer_seed], &invoking_program.key()).1;
    let seeds: [&[u8]; 2] = [signer_seed, &[local_bump]];

    let inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: vec![compressed_account],
        output_compressed_accounts,
        proof,
        new_address_params: Vec::new(),
        compress_or_decompress_lamports: None,
        is_compress: false,
        signer_seeds: seeds.iter().map(|seed| seed.to_vec()).collect(),
        cpi_context,
    };

    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();
    let seeds: [&[u8]; 2] = [b"cpi_signer".as_slice(), &[bump]];

    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.cpi_signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program,
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_context_account,
    };
    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.light_system_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)?;
    Ok(())
}

#[derive(Accounts)]
pub struct InvalidateNotOwnedCompressedAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub light_system_program: Program<'info, light_system_program::program::LightSystemProgram>,
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
    pub self_program: Program<'info, crate::program::SystemCpiTest>,
    /// CHECK:
    pub cpi_signer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub compressed_token_program:
        Program<'info, light_compressed_token::program::LightCompressedToken>,
}
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct TokenTransferData {
    pub mint: Pubkey,
    pub input_token_data_with_context: InputTokenDataWithContext,
}
#[inline(never)]
pub fn cpi_context_tx<'info>(
    ctx: &Context<'_, '_, '_, 'info, InvalidateNotOwnedCompressedAccount<'info>>,
    compressed_account: PackedCompressedAccountWithMerkleContext,
    proof: CompressedProof,
    bump: u8,
    token_transfer_data: TokenTransferData,
    mode: WithInputAccountsMode,
    cpi_context: CompressedCpiContext,
) -> Result<()> {
    cpi_compressed_token_transfer(
        ctx,
        proof.clone(),
        token_transfer_data,
        mode.clone(),
        Some(cpi_context),
    )?;
    process_invalidate_not_owned_compressed_account(
        ctx,
        compressed_account,
        Some(proof),
        bump,
        mode,
        Some(cpi_context),
    )
}
#[inline(never)]
pub fn cpi_compressed_token_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, InvalidateNotOwnedCompressedAccount<'info>>,
    proof: CompressedProof,
    token_transfer_data: TokenTransferData,
    mode: WithInputAccountsMode,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    let proof = match mode {
        WithInputAccountsMode::CpiContextProofMismatch => {
            // This does not fail because the proof is not verified in the cpi call.
            // It will fail in the next cpi call because the proofs will not match.
            let mut proof = proof;
            proof.a = proof.c;
            proof
        }
        _ => proof,
    };
    let mint = token_transfer_data.mint;
    let input_token_data_with_context = token_transfer_data.input_token_data_with_context;
    let output_token = PackedTokenTransferOutputData {
        amount: input_token_data_with_context.amount,
        owner: crate::ID,
        lamports: None,
        merkle_tree_index: input_token_data_with_context
            .merkle_context
            .merkle_tree_pubkey_index,
    };
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: Some(proof),
        mint,
        signer_is_delegate: false,
        input_token_data_with_context: vec![input_token_data_with_context],
        output_compressed_accounts: vec![output_token],
        is_compress: false,
        compress_or_decompress_amount: None,
        cpi_context,
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
        decompress_token_account: None,
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
