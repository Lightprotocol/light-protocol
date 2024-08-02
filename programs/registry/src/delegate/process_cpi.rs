use super::traits::{
    CompressedCpiContextTrait, CompressedTokenProgramAccounts, MintToAccounts, SignerAccounts,
    SystemProgramAccounts,
};
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::{prelude::*, Bumps};
use light_compressed_token::process_transfer::{
    CompressedTokenInstructionDataTransfer, DelegatedTransfer, InputTokenDataWithContext,
    PackedTokenTransferOutputData,
};
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{compressed_account::PackedCompressedAccountWithMerkleContext, CompressedCpiContext},
    InstructionDataInvokeCpi, OutputCompressedAccountWithPackedContext,
};

#[inline(never)]
pub fn cpi_light_system_program<
    'a,
    'b,
    'c,
    'info,
    C: SignerAccounts<'info> + SystemProgramAccounts<'info> + CompressedCpiContextTrait<'info> + Bumps,
>(
    ctx: &Context<'a, 'b, 'c, 'info, C>,
    proof: Option<CompressedProof>,
    cpi_context: Option<CompressedCpiContext>,
    input_pda: Option<PackedCompressedAccountWithMerkleContext>,
    output_pda: OutputCompressedAccountWithPackedContext,
    remaining_accounts: Vec<AccountInfo<'info>>,
) -> Result<()> {
    // let cpi_context = if let Some(mut cpi_context) = cpi_context {
    //     cpi_context.set_context = true;
    //     Some(cpi_context)
    // } else {
    //     None
    // };
    let bump = &[BUMP_CPI_AUTHORITY];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let input_compressed_accounts_with_merkle_context = if let Some(input_pda) = input_pda {
        vec![input_pda]
    } else {
        vec![]
    };
    let inputs_struct = light_system_program::invoke_cpi::instruction::InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context,
        output_compressed_accounts: vec![output_pda],
        proof,
        new_address_params: Vec::new(),
        compress_or_decompress_lamports: None,
        is_compress: false,
        signer_seeds: seeds.iter().map(|seed| seed.to_vec()).collect(),
        cpi_context,
    };
    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).map_err(ProgramError::from)?;

    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.get_fee_payer(),
        authority: ctx.accounts.get_cpi_authority_pda(),
        registered_program_pda: ctx.accounts.get_registered_program_pda(),
        noop_program: ctx.accounts.get_noop_program(),
        account_compression_authority: ctx.accounts.get_account_compression_authority(),
        account_compression_program: ctx.accounts.get_account_compression_program(),
        invoking_program: ctx.accounts.get_self_program(),
        system_program: ctx.accounts.get_system_program(),
        sol_pool_pda: None,
        decompression_recipient: None,
        cpi_context_account: ctx.accounts.get_cpi_context(),
    };
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.get_light_system_program(),
        cpi_accounts,
        signer_seeds,
    );

    cpi_ctx.remaining_accounts = remaining_accounts.to_vec();

    light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)?;
    Ok(())
}

#[inline(never)]
pub fn cpi_compressed_token_transfer<
    'info,
    C: SignerAccounts<'info>
        + SystemProgramAccounts<'info>
        + CompressedTokenProgramAccounts<'info>
        + CompressedCpiContextTrait<'info>
        + Bumps,
    const SEED_LEN: usize,
>(
    ctx: &Context<'_, '_, '_, 'info, C>,
    proof: Option<CompressedProof>,
    compression_amount: Option<u64>,
    is_compress: bool,
    _salt: u64,
    mut cpi_context: CompressedCpiContext,
    mint: &Pubkey,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
    owner: &Pubkey,
    authority: AccountInfo<'info>,
    seeds: [&[u8]; SEED_LEN],
    mut remaining_accounts: Vec<AccountInfo<'info>>,
) -> Result<()> {
    cpi_context.cpi_context_account_index = remaining_accounts.len() as u8;
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof,
        mint: *mint,
        delegated_transfer: Some(DelegatedTransfer {
            owner: *owner,
            delegate_change_account_index: None,
        }),
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress,
        compress_or_decompress_amount: compression_amount,
        cpi_context: Some(cpi_context),
        lamports_change_account_merkle_tree_index: None,
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    // let authority = ctx.accounts.get_escrow_authority_pda();
    let (token_pool_pda, token_program, compress_or_decompress_token_account) =
        if compression_amount.is_some() {
            (
                Some(ctx.accounts.get_token_pool_pda()),
                Some(ctx.accounts.get_spl_token_program()),
                ctx.accounts.get_compress_or_decompress_token_account(),
            )
        } else {
            (None, None, None)
        };
    let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.get_fee_payer(),
        authority,
        registered_program_pda: ctx.accounts.get_registered_program_pda(),
        noop_program: ctx.accounts.get_noop_program(),
        account_compression_authority: ctx.accounts.get_account_compression_authority(),
        account_compression_program: ctx.accounts.get_account_compression_program(),
        self_program: ctx.accounts.get_compressed_token_program(),
        cpi_authority_pda: ctx.accounts.get_token_cpi_authority_pda(),
        light_system_program: ctx.accounts.get_light_system_program(),
        token_pool_pda,
        compress_or_decompress_token_account,
        token_program,
        system_program: ctx.accounts.get_system_program(),
    };
    let signer_seeds = &[&seeds[..]];
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.get_compressed_token_program(),
        cpi_accounts,
        signer_seeds,
    );
    remaining_accounts.push(ctx.accounts.get_cpi_context().unwrap());
    cpi_ctx.remaining_accounts = remaining_accounts;
    light_compressed_token::cpi::transfer(cpi_ctx, inputs)
}

#[inline(never)]
pub fn cpi_compressed_token_mint_to<
    'a,
    'b,
    'c,
    'info,
    C: SignerAccounts<'info>
        + SystemProgramAccounts<'info>
        + CompressedTokenProgramAccounts<'info>
        + MintToAccounts<'info>
        + Bumps,
    const SEED_LEN: usize,
>(
    ctx: &Context<'a, 'b, 'c, 'info, C>,
    recipients: Vec<Pubkey>,
    amounts: Vec<u64>,
    seeds: [&[u8]; SEED_LEN],
    merkle_tree: AccountInfo<'info>,
) -> Result<()> {
    let signer_seeds = &[&seeds[..]];
    let cpi_accounts = light_compressed_token::cpi::accounts::MintToInstruction {
        fee_payer: ctx.accounts.get_fee_payer(),
        authority: ctx.accounts.get_cpi_authority_pda(),
        registered_program_pda: ctx.accounts.get_registered_program_pda(),
        noop_program: ctx.accounts.get_noop_program(),
        account_compression_authority: ctx.accounts.get_account_compression_authority(),
        account_compression_program: ctx.accounts.get_account_compression_program(),
        self_program: ctx.accounts.get_compressed_token_program(),
        cpi_authority_pda: ctx.accounts.get_token_cpi_authority_pda(),
        light_system_program: ctx.accounts.get_light_system_program(),
        token_pool_pda: ctx.accounts.get_token_pool_pda(),
        token_program: ctx.accounts.get_spl_token_program(),
        system_program: ctx.accounts.get_system_program(),
        mint: ctx.accounts.get_mint(),
        merkle_tree,
        sol_pool_pda: None,
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.get_compressed_token_program(),
        cpi_accounts,
        signer_seeds,
    );

    light_compressed_token::cpi::mint_to(cpi_ctx, recipients, amounts, None)
}

pub const BUMP_CPI_AUTHORITY: u8 = 254;
/// Get static cpi signer seeds
pub fn get_cpi_signer_seeds() -> [&'static [u8]; 2] {
    let bump: &[u8; 1] = &[BUMP_CPI_AUTHORITY];
    let seeds: [&'static [u8]; 2] = [CPI_AUTHORITY_PDA_SEED, bump];
    seeds
}

#[inline(never)]
pub fn mint_spl_to_pool_pda<
    'info,
    C: SignerAccounts<'info>
        + SystemProgramAccounts<'info>
        + CompressedTokenProgramAccounts<'info>
        + CompressedCpiContextTrait<'info>
        + MintToAccounts<'info>
        + Bumps,
    const SEED_LEN: usize,
>(
    ctx: &Context<'_, '_, '_, 'info, C>,
    mint_amount: u64,
    recipient: AccountInfo<'info>,
    seeds: [&[u8]; SEED_LEN],
) -> Result<()> {
    let cpi_accounts = anchor_spl::token::MintTo {
        mint: ctx.accounts.get_mint(),
        to: recipient,
        authority: ctx.accounts.get_cpi_authority_pda(),
    };
    let signer_seeds = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.get_spl_token_program(),
        cpi_accounts,
        signer_seeds,
    );

    anchor_spl::token::mint_to(cpi_ctx, mint_amount)?;
    Ok(())
}

#[inline(never)]
pub fn approve_spl_token<
    'info,
    C: SignerAccounts<'info>
        + SystemProgramAccounts<'info>
        + CompressedTokenProgramAccounts<'info>
        + CompressedCpiContextTrait<'info>
        + Bumps,
    const SEED_LEN: usize,
>(
    ctx: &Context<'_, '_, '_, 'info, C>,
    amount: u64,
    recipient: AccountInfo<'info>,
    delegate: AccountInfo<'info>,
    seeds: [&[u8]; SEED_LEN],
) -> Result<()> {
    let cpi_accounts = anchor_spl::token::Approve {
        to: recipient,
        authority: ctx.accounts.get_cpi_authority_pda(),
        delegate,
    };
    let signer_seeds = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.get_spl_token_program(),
        cpi_accounts,
        signer_seeds,
    );

    anchor_spl::token::approve(cpi_ctx, amount)?;
    Ok(())
}
