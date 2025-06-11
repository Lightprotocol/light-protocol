use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use light_compressed_account::{
    compressed_account::{CompressedAccount, PackedCompressedAccountWithMerkleContext},
    instruction_data::{
        compressed_proof::CompressedProof, cpi_context::CompressedCpiContext,
        data::OutputCompressedAccountWithPackedContext, invoke_cpi::InstructionDataInvokeCpi,
    },
};
use light_compressed_token::{
    delegation::{CompressedTokenInstructionDataApprove, CompressedTokenInstructionDataRevoke},
    freeze::{CompressedTokenInstructionDataFreeze, CompressedTokenInstructionDataThaw},
    process_transfer::{
        CompressedTokenInstructionDataTransfer, InputTokenDataWithContext,
        PackedTokenTransferOutputData,
    },
    CompressedTokenInstructionDataBurn,
};
use light_system_program::program::LightSystemProgram;

use crate::ID;

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub enum WithInputAccountsMode {
    NotOwnedCompressedAccount,
    CpiContextMissing,
    CpiContextAccountMissing,
    CpiContextFeePayerMismatch,
    CpiContextEmpty,
    CpiContextInvalidInvokingProgram,
    CpiContextWriteAccessCheckFailed,
    CpiContextWriteToNotOwnedAccount,
    Approve,
    Revoke,
    Freeze,
    Thaw,
    Burn,
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
        WithInputAccountsMode::CpiContextFeePayerMismatch
        | WithInputAccountsMode::Burn
        | WithInputAccountsMode::Freeze
        | WithInputAccountsMode::Revoke
        | WithInputAccountsMode::Thaw
        | WithInputAccountsMode::Approve => cpi_context_tx(
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
        WithInputAccountsMode::CpiContextEmpty
        | WithInputAccountsMode::CpiContextFeePayerMismatch => Some(CompressedCpiContext {
            cpi_context_account_index: cpi_context.unwrap().cpi_context_account_index,
            set_context: false,
            first_set_context: false,
        }),
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
                    owner: light_compressed_token::ID.into(),
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
    msg!("input compressed_account {:?}", compressed_account);
    let inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: vec![compressed_account],
        output_compressed_accounts,
        proof,
        new_address_params: Vec::new(),
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context,
    };

    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();
    let seeds: [&[u8]; 2] = [CPI_AUTHORITY_PDA_SEED, &[bump]];

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
    pub light_system_program: Program<'info, LightSystemProgram>,
    pub account_compression_program: Program<'info, AccountCompression>,
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
    #[account(mut)]
    pub invalid_fee_payer: Signer<'info>,
    /// CHECK:
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub token_pool_account: AccountInfo<'info>,
    /// CHECK:
    pub token_program: AccountInfo<'info>,
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
    mut cpi_context: CompressedCpiContext,
) -> Result<()> {
    match mode {
        WithInputAccountsMode::CpiContextFeePayerMismatch => cpi_compressed_token_transfer(
            ctx,
            proof,
            token_transfer_data,
            mode.clone(),
            Some(cpi_context),
        )?,
        WithInputAccountsMode::Approve => cpi_compressed_token_approve_revoke(
            ctx,
            proof,
            token_transfer_data,
            mode.clone(),
            Some(cpi_context),
        )?,
        WithInputAccountsMode::Revoke => cpi_compressed_token_approve_revoke(
            ctx,
            proof,
            token_transfer_data,
            mode.clone(),
            Some(cpi_context),
        )?,
        WithInputAccountsMode::Freeze | WithInputAccountsMode::Thaw => {
            cpi_compressed_token_freeze_or_thaw(
                ctx,
                proof,
                token_transfer_data,
                mode.clone(),
                Some(cpi_context),
            )?
        }
        WithInputAccountsMode::Burn => {
            cpi_compressed_token_burn(ctx, proof, token_transfer_data, Some(cpi_context))?
        }
        _ => panic!("Invalid mode"),
    }
    match mode {
        WithInputAccountsMode::CpiContextFeePayerMismatch
        | WithInputAccountsMode::Burn
        | WithInputAccountsMode::Freeze
        | WithInputAccountsMode::Revoke
        | WithInputAccountsMode::Thaw
        | WithInputAccountsMode::Approve => {
            cpi_context.set_context = false;
            cpi_context.first_set_context = false;
        }
        _ => {}
    }
    write_into_cpi_account(
        ctx,
        compressed_account,
        Some(proof),
        bump,
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
    let fee_payer = match mode {
        WithInputAccountsMode::CpiContextFeePayerMismatch => {
            // This does not fail because the cpi context is initialized with this fee payer
            ctx.accounts.invalid_fee_payer.to_account_info()
        }
        _ => ctx.accounts.signer.to_account_info(),
    };
    msg!("cpi_context: {:?}", cpi_context);
    msg!(
        "cpi context account: {:?}",
        ctx.remaining_accounts[cpi_context.unwrap().cpi_context_account_index as usize].key()
    );
    msg!(
        "cpi context account: {:?}",
        ctx.remaining_accounts[cpi_context.unwrap().cpi_context_account_index as usize]
    );
    let mint = token_transfer_data.mint;
    let input_token_data_with_context = token_transfer_data.input_token_data_with_context;
    let output_token = PackedTokenTransferOutputData {
        amount: input_token_data_with_context.amount,
        owner: crate::ID,
        lamports: None,
        tlv: None,
        merkle_tree_index: input_token_data_with_context
            .merkle_context
            .merkle_tree_pubkey_index,
    };
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: Some(proof),
        mint,
        delegated_transfer: None,
        input_token_data_with_context: vec![input_token_data_with_context],
        output_compressed_accounts: vec![output_token],
        is_compress: false,
        compress_or_decompress_amount: None,
        cpi_context,
        lamports_change_account_merkle_tree_index: None,
        with_transaction_hash: false,
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer,
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

#[inline(never)]
pub fn cpi_compressed_token_approve_revoke<'info>(
    ctx: &Context<'_, '_, '_, 'info, InvalidateNotOwnedCompressedAccount<'info>>,
    proof: CompressedProof,
    token_transfer_data: TokenTransferData,
    mode: WithInputAccountsMode,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    msg!("cpi_context: {:?}", cpi_context);
    msg!(
        "cpi context account: {:?}",
        ctx.remaining_accounts[cpi_context.unwrap().cpi_context_account_index as usize].key()
    );
    msg!(
        "cpi context account: {:?}",
        ctx.remaining_accounts[cpi_context.unwrap().cpi_context_account_index as usize]
    );
    let mint = token_transfer_data.mint;
    let input_token_data_with_context = token_transfer_data.input_token_data_with_context;
    let merkle_tree_index = input_token_data_with_context
        .merkle_context
        .merkle_tree_pubkey_index;
    let mut inputs = Vec::new();
    match mode {
        WithInputAccountsMode::Approve => {
            let inputs_struct = CompressedTokenInstructionDataApprove {
                proof,
                mint,
                delegated_amount: input_token_data_with_context.amount,
                input_token_data_with_context: vec![input_token_data_with_context],
                delegate: ctx.accounts.invalid_fee_payer.key(),
                delegate_lamports: None,
                change_account_merkle_tree_index: merkle_tree_index,
                delegate_merkle_tree_index: merkle_tree_index,
                cpi_context,
            };
            msg!("cpi test program calling approve");

            CompressedTokenInstructionDataApprove::serialize(&inputs_struct, &mut inputs).unwrap();
        }
        WithInputAccountsMode::Revoke => {
            let inputs_struct = CompressedTokenInstructionDataRevoke {
                proof,
                mint,
                input_token_data_with_context: vec![input_token_data_with_context],
                output_account_merkle_tree_index: merkle_tree_index,
                cpi_context,
            };
            msg!("cpi test program calling revoke");

            CompressedTokenInstructionDataRevoke::serialize(&inputs_struct, &mut inputs).unwrap();
        }
        _ => panic!("Invalid mode"),
    }

    let cpi_accounts = light_compressed_token::cpi::accounts::GenericInstruction {
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
        system_program: ctx.accounts.system_program.to_account_info(),
    };

    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    match mode {
        WithInputAccountsMode::Approve => {
            light_compressed_token::cpi::approve(cpi_ctx, inputs)?;
        }
        WithInputAccountsMode::Revoke => {
            light_compressed_token::cpi::revoke(cpi_ctx, inputs)?;
        }
        _ => panic!("Invalid mode"),
    }
    Ok(())
}

#[inline(never)]
pub fn cpi_compressed_token_burn<'info>(
    ctx: &Context<'_, '_, '_, 'info, InvalidateNotOwnedCompressedAccount<'info>>,
    proof: CompressedProof,
    token_transfer_data: TokenTransferData,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    msg!("cpi_context: {:?}", cpi_context);
    msg!(
        "cpi context account: {:?}",
        ctx.remaining_accounts[cpi_context.unwrap().cpi_context_account_index as usize].key()
    );
    msg!(
        "cpi context account: {:?}",
        ctx.remaining_accounts[cpi_context.unwrap().cpi_context_account_index as usize]
    );
    let input_token_data_with_context = token_transfer_data.input_token_data_with_context;
    let merkle_tree_index = input_token_data_with_context
        .merkle_context
        .merkle_tree_pubkey_index;
    let mut inputs = Vec::new();
    let inputs_struct = CompressedTokenInstructionDataBurn {
        proof,
        delegated_transfer: None,
        input_token_data_with_context: vec![input_token_data_with_context.clone()],
        burn_amount: input_token_data_with_context.amount - 1,
        change_account_merkle_tree_index: merkle_tree_index,
        cpi_context,
    };
    CompressedTokenInstructionDataBurn::serialize(&inputs_struct, &mut inputs).unwrap();
    let cpi_accounts = light_compressed_token::cpi::accounts::BurnInstruction {
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
        system_program: ctx.accounts.system_program.to_account_info(),
        token_pool_pda: ctx.accounts.token_pool_account.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };

    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    light_compressed_token::cpi::burn(cpi_ctx, inputs)?;

    Ok(())
}

#[inline(never)]
pub fn cpi_compressed_token_freeze_or_thaw<'info>(
    ctx: &Context<'_, '_, '_, 'info, InvalidateNotOwnedCompressedAccount<'info>>,
    proof: CompressedProof,
    token_transfer_data: TokenTransferData,
    mode: WithInputAccountsMode,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    msg!("cpi_context: {:?}", cpi_context);
    msg!(
        "cpi context account: {:?}",
        ctx.remaining_accounts[cpi_context.unwrap().cpi_context_account_index as usize].key()
    );
    msg!(
        "cpi context account: {:?}",
        ctx.remaining_accounts[cpi_context.unwrap().cpi_context_account_index as usize]
    );
    let input_token_data_with_context = token_transfer_data.input_token_data_with_context;
    let merkle_tree_index = input_token_data_with_context
        .merkle_context
        .merkle_tree_pubkey_index;
    let mut inputs = Vec::new();
    match mode {
        WithInputAccountsMode::Freeze => {
            let inputs_struct = CompressedTokenInstructionDataFreeze {
                proof,
                input_token_data_with_context: vec![input_token_data_with_context],
                owner: ctx.accounts.signer.key(),
                cpi_context,
                outputs_merkle_tree_index: merkle_tree_index,
            };
            CompressedTokenInstructionDataFreeze::serialize(&inputs_struct, &mut inputs).unwrap();
        }
        WithInputAccountsMode::Thaw => {
            let inputs_struct = CompressedTokenInstructionDataThaw {
                proof,
                input_token_data_with_context: vec![input_token_data_with_context],
                outputs_merkle_tree_index: merkle_tree_index,
                cpi_context,
                owner: ctx.accounts.signer.key(),
            };
            CompressedTokenInstructionDataThaw::serialize(&inputs_struct, &mut inputs).unwrap();
        }
        _ => panic!("Invalid mode"),
    }

    let cpi_accounts = light_compressed_token::cpi::accounts::FreezeInstruction {
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
        system_program: ctx.accounts.system_program.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };

    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    match mode {
        WithInputAccountsMode::Freeze => {
            light_compressed_token::cpi::freeze(cpi_ctx, inputs)?;
        }
        WithInputAccountsMode::Thaw => {
            light_compressed_token::cpi::thaw(cpi_ctx, inputs)?;
        }
        _ => panic!("Invalid mode"),
    }
    Ok(())
}

fn write_into_cpi_account<'info>(
    ctx: &Context<'_, '_, '_, 'info, InvalidateNotOwnedCompressedAccount<'info>>,
    compressed_account: PackedCompressedAccountWithMerkleContext,
    proof: Option<CompressedProof>,
    bump: u8,
    // compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    let compressed_pda = OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            data: compressed_account.compressed_account.data.clone(),
            owner: ID.into(),
            lamports: 0,
            address: compressed_account.compressed_account.address,
        },
        merkle_tree_index: 0,
    };
    let seeds: [&[u8]; 2] = [CPI_AUTHORITY_PDA_SEED, &[bump]];

    let inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: vec![compressed_account],
        output_compressed_accounts: vec![compressed_pda],
        proof,
        new_address_params: Vec::new(),
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context,
    };

    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();
    let cpi_context_account = cpi_context.map(|cpi_context| {
        ctx.remaining_accounts
            .get(cpi_context.cpi_context_account_index as usize)
            .unwrap()
            .to_account_info()
    });
    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.cpi_signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: ctx.accounts.self_program.to_account_info(),
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
