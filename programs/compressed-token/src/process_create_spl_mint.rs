use crate::{
    constants::{COMPRESSED_MINT_DISCRIMINATOR, POOL_SEED},
    create_mint::CompressedMint,
    instructions::create_spl_mint::CreateSplMintInstruction,
    process_mint::CompressedMintInputs,
    process_transfer::get_cpi_signer_seeds,
};
use anchor_lang::prelude::*;
use anchor_spl::token_2022;
use anchor_spl::token_interface;
use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
    },
    instruction_data::{
        data::OutputCompressedAccountWithPackedContext, invoke_cpi::InstructionDataInvokeCpi,
    },
};

/// Creates a Token-2022 mint account that corresponds to a compressed mint
/// and updates the compressed mint to mark it as is_decompressed=true
///
/// This instruction creates the SPL mint PDA that was referenced in the compressed mint's
/// spl_mint field when create_compressed_mint was called, and updates the compressed mint
/// to enable syncing between compressed and SPL representations.
pub fn process_create_spl_mint<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateSplMintInstruction<'info>>,
    _token_pool_bump: u8,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    compressed_mint_inputs: CompressedMintInputs,
) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.mint.key(),
        compressed_mint_inputs.compressed_mint_input.spl_mint,
        crate::ErrorCode::InvalidMintPda
    );

    // Create the mint account manually (PDA derived from our program, owned by token program)
    create_mint_account(&ctx)?;

    // Initialize the mint account using Token-2022's initialize_mint2 instruction
    let cpi_accounts = token_2022::InitializeMint2 {
        mint: ctx.accounts.mint.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    token_2022::initialize_mint2(
        cpi_ctx,
        decimals,
        &mint_authority,
        freeze_authority.as_ref(),
    )?;

    // Create the token pool account manually (PDA derived from our program, owned by token program)
    create_token_pool_account_manual(&ctx)?;

    // Initialize the token pool account
    initialize_token_pool_account(&ctx)?;

    // Mint the existing supply to the token pool if there's any supply
    if compressed_mint_inputs.compressed_mint_input.supply > 0 {
        mint_existing_supply_to_pool(&ctx, &compressed_mint_inputs, &mint_authority)?;
    }

    // Update the compressed mint to mark it as is_decompressed = true
    update_compressed_mint_to_decompressed(
        &ctx,
        compressed_mint_inputs,
        decimals,
        mint_authority,
        freeze_authority,
    )?;

    Ok(())
}

fn update_compressed_mint_to_decompressed<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateSplMintInstruction<'info>>,
    compressed_mint_inputs: CompressedMintInputs,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
) -> Result<()> {
    // Create the updated compressed mint with is_decompressed = true
    let mut updated_compressed_mint = CompressedMint {
        spl_mint: compressed_mint_inputs.compressed_mint_input.spl_mint,
        supply: compressed_mint_inputs.compressed_mint_input.supply,
        decimals,
        is_decompressed: false, // Mark as decompressed
        mint_authority: Some(mint_authority),
        freeze_authority,
        num_extensions: compressed_mint_inputs.compressed_mint_input.num_extensions,
    };
    let input_compressed_account = {
        // Calculate data hash
        let input_data_hash = updated_compressed_mint
            .hash()
            .map_err(|_| crate::ErrorCode::HashToFieldError)?;

        // Create compressed account data
        let input_compressed_account_data = CompressedAccountData {
            discriminator: COMPRESSED_MINT_DISCRIMINATOR,
            data: Vec::new(),
            data_hash: input_data_hash,
        };
        // Create input compressed account
        PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: crate::ID.into(),
                lamports: 0,
                data: Some(input_compressed_account_data),
                address: Some(compressed_mint_inputs.address),
            },
            merkle_context: compressed_mint_inputs.merkle_context,
            root_index: compressed_mint_inputs.root_index,
            read_only: false,
        }
    };

    updated_compressed_mint.is_decompressed = true;

    let output_compressed_account = {
        // Serialize the updated compressed mint data
        let mut compressed_mint_bytes = Vec::new();
        updated_compressed_mint.serialize(&mut compressed_mint_bytes)?;

        let output_compressed_account_data = CompressedAccountData {
            discriminator: COMPRESSED_MINT_DISCRIMINATOR,
            data: compressed_mint_bytes,
            data_hash: updated_compressed_mint.hash().map_err(ProgramError::from)?,
        };

        // Create output compressed account (updated compressed mint)
        OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: crate::ID.into(),
                lamports: 0,
                data: Some(output_compressed_account_data),
                address: Some(compressed_mint_inputs.address),
            },
            merkle_tree_index: compressed_mint_inputs.output_merkle_tree_index,
        }
    };

    // Create CPI instruction data
    let inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: vec![input_compressed_account],
        output_compressed_accounts: vec![output_compressed_account],
        proof: compressed_mint_inputs.proof,
        new_address_params: Vec::new(),
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context: None,
    };

    // Execute CPI to light system program to update the compressed mint
    execute_compressed_mint_update_cpi(ctx, inputs_struct)?;

    Ok(())
}

fn execute_compressed_mint_update_cpi<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateSplMintInstruction<'info>>,
    inputs_struct: InstructionDataInvokeCpi,
) -> Result<()> {
    let invoking_program = ctx.accounts.self_program.to_account_info();

    let seeds = get_cpi_signer_seeds();
    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.fee_payer.to_account_info(),
        authority: ctx.accounts.cpi_authority_pda.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program,
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_context_account: None,
    };

    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.light_system_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );

    // Add remaining accounts (merkle trees)
    cpi_ctx.remaining_accounts = vec![
        ctx.accounts.in_merkle_tree.to_account_info(),
        ctx.accounts.in_output_queue.to_account_info(),
        ctx.accounts.out_output_queue.to_account_info(),
    ];

    light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)?;
    Ok(())
}

/// Initializes the token pool account (assumes account already exists)
fn initialize_token_pool_account<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateSplMintInstruction<'info>>,
) -> Result<()> {
    // Initialize the token account
    let cpi_accounts = token_interface::InitializeAccount3 {
        account: ctx.accounts.token_pool_pda.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        authority: ctx.accounts.cpi_authority_pda.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);

    token_interface::initialize_account3(cpi_ctx)?;
    Ok(())
}

/// Creates the token pool account manually as a PDA derived from our program but owned by the token program
fn create_token_pool_account_manual<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateSplMintInstruction<'info>>,
) -> Result<()> {
    let token_account_size = 165; // Size of Token account
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(token_account_size);

    // Derive the token pool PDA seeds and bump
    let mint_key = ctx.accounts.mint.key();
    let (expected_token_pool, bump) =
        Pubkey::find_program_address(&[POOL_SEED, mint_key.as_ref()], &crate::ID);

    // Verify the provided token pool account matches the expected PDA
    require_keys_eq!(
        ctx.accounts.token_pool_pda.key(),
        expected_token_pool,
        crate::ErrorCode::InvalidTokenPoolPda
    );

    let seeds = &[POOL_SEED, mint_key.as_ref(), &[bump]];

    // Create account owned by token program but derived from our program
    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        &ctx.accounts.fee_payer.key(),
        &ctx.accounts.token_pool_pda.key(),
        lamports,
        token_account_size as u64,
        &ctx.accounts.token_program.key(), // Owned by token program
    );

    anchor_lang::solana_program::program::invoke_signed(
        &create_account_ix,
        &[
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.token_pool_pda.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[seeds], // Signed with our program's PDA seeds
    )?;

    Ok(())
}

/// Mints the existing supply from compressed mint to the token pool
fn mint_existing_supply_to_pool<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateSplMintInstruction<'info>>,
    compressed_mint_inputs: &CompressedMintInputs,
    mint_authority: &Pubkey,
) -> Result<()> {
    // Only mint if the authority matches
    require_keys_eq!(
        ctx.accounts.authority.key(),
        *mint_authority,
        crate::ErrorCode::InvalidAuthorityMint
    );

    let supply = compressed_mint_inputs.compressed_mint_input.supply;

    // Mint tokens to the pool
    let cpi_accounts = token_interface::MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.token_pool_pda.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);

    token_interface::mint_to(cpi_ctx, supply)?;
    Ok(())
}

/// Creates the mint account manually as a PDA derived from our program but owned by the token program
fn create_mint_account<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateSplMintInstruction<'info>>,
) -> Result<()> {
    let mint_account_size = 82; // Size of Token-2022 Mint account
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(mint_account_size);

    // Derive the mint PDA seeds and bump
    let (expected_mint, bump) = Pubkey::find_program_address(
        &[b"compressed_mint", ctx.accounts.mint_signer.key().as_ref()],
        &crate::ID,
    );

    // Verify the provided mint account matches the expected PDA
    require_keys_eq!(
        ctx.accounts.mint.key(),
        expected_mint,
        crate::ErrorCode::InvalidMintPda
    );

    let mint_signer_key = ctx.accounts.mint_signer.key();
    let seeds = &[b"compressed_mint", mint_signer_key.as_ref(), &[bump]];

    // Create account owned by token program but derived from our program
    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        &ctx.accounts.fee_payer.key(),
        &ctx.accounts.mint.key(),
        lamports,
        mint_account_size as u64,
        &ctx.accounts.token_program.key(), // Owned by token program
    );

    anchor_lang::solana_program::program::invoke_signed(
        &create_account_ix,
        &[
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[seeds], // Signed with our program's PDA seeds
    )?;

    Ok(())
}
