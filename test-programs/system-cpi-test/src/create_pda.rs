use anchor_lang::prelude::*;
use light_hasher::{errors::HasherError, DataHasher, Poseidon};
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        address::derive_address,
        compressed_account::{CompressedAccount, CompressedAccountData},
        CompressedCpiContext,
    },
    InstructionDataInvokeCpi, NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
};

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq)]
pub enum CreatePdaMode {
    ProgramIsSigner,
    ProgramIsNotSigner,
    InvalidSignerSeeds,
    InvalidInvokingProgram,
    WriteToAccountNotOwned,
    NoData,
}

pub fn process_create_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    data: [u8; 31],
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    owner_program: Pubkey,
    cpi_context: Option<CompressedCpiContext>,
    is_program_signer: CreatePdaMode,
    bump: u8,
) -> Result<()> {
    let compressed_pda =
        create_compressed_pda_data(data, &ctx, &new_address_params, &owner_program)?;

    match is_program_signer {
        CreatePdaMode::ProgramIsNotSigner => {
            cpi_compressed_pda_transfer_as_non_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
            )?;
        }
        // functional test
        CreatePdaMode::ProgramIsSigner => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                CreatePdaMode::ProgramIsSigner,
            )?;
        }
        CreatePdaMode::InvalidSignerSeeds => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                CreatePdaMode::InvalidSignerSeeds,
            )?;
        }
        CreatePdaMode::InvalidInvokingProgram => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                CreatePdaMode::InvalidInvokingProgram,
            )?;
        }
        CreatePdaMode::WriteToAccountNotOwned => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                CreatePdaMode::WriteToAccountNotOwned,
            )?;
        }
        CreatePdaMode::NoData => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                CreatePdaMode::NoData,
            )?;
        }
    }
    Ok(())
}

/// Functional:
/// 1. ProgramIsSigner
fn cpi_compressed_pda_transfer_as_non_program<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    let inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: vec![compressed_pda],
        proof,
        new_address_params: vec![new_address_params],
        compress_or_decompress_lamports: None,
        is_compress: false,
        signer_seeds: Vec::new(),
        cpi_context,
    };

    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: ctx.accounts.self_program.to_account_info(),
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_context_account: None,
    };
    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.light_system_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)?;
    Ok(())
}

fn cpi_compressed_pda_transfer_as_program<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: Option<CompressedCpiContext>,
    bump: u8,
    mode: CreatePdaMode,
) -> Result<()> {
    let signer_seed = match mode {
        CreatePdaMode::InvalidSignerSeeds => b"cpi_signer1".as_slice(),
        _ => b"cpi_signer".as_slice(),
    };
    let invoking_program = match mode {
        CreatePdaMode::InvalidInvokingProgram => ctx.accounts.signer.to_account_info(),
        _ => ctx.accounts.self_program.to_account_info(),
    };
    let compressed_pda = match mode {
        CreatePdaMode::WriteToAccountNotOwned => {
            // account with data needs to be owned by the program
            let mut compressed_pda = compressed_pda;
            compressed_pda.compressed_account.owner = ctx.accounts.signer.key();
            compressed_pda
        }
        CreatePdaMode::NoData => {
            let mut compressed_pda = compressed_pda;

            compressed_pda.compressed_account.data = None;
            compressed_pda
        }
        _ => compressed_pda,
    };

    let local_bump = Pubkey::find_program_address(&[signer_seed], &invoking_program.key()).1;
    let seeds: [&[u8]; 2] = [signer_seed, &[local_bump]];
    let inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: vec![compressed_pda],
        proof,
        new_address_params: vec![new_address_params],
        compress_or_decompress_lamports: None,
        is_compress: false,
        signer_seeds: seeds.iter().map(|seed| seed.to_vec()).collect(),
        cpi_context,
    };
    // defining seeds again so that the cpi doesn't fail we want to test the check in the compressed pda program
    let seeds: [&[u8]; 2] = [b"cpi_signer".as_slice(), &[bump]];
    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();

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
        cpi_context_account: None,
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

fn create_compressed_pda_data(
    data: [u8; 31],
    ctx: &Context<'_, '_, '_, '_, CreateCompressedPda<'_>>,
    new_address_params: &NewAddressParamsPacked,
    owner_program: &Pubkey,
) -> Result<OutputCompressedAccountWithPackedContext> {
    let timelock_compressed_pda = RegisteredUser {
        user_pubkey: *ctx.accounts.signer.key,
        data,
    };
    let compressed_account_data = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: timelock_compressed_pda.try_to_vec().unwrap(),
        data_hash: timelock_compressed_pda
            .hash::<Poseidon>()
            .map_err(ProgramError::from)?,
    };
    let derive_address = derive_address(
        &ctx.remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
            .key(),
        &new_address_params.seed,
    )
    .map_err(|_| ProgramError::InvalidArgument)?;
    Ok(OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            owner: *owner_program, // should be crate::ID, test provides an invalid owner
            lamports: 0,
            address: Some(derive_address),
            data: Some(compressed_account_data),
        },
        merkle_tree_index: 0,
    })
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct RegisteredUser {
    pub user_pubkey: Pubkey,
    pub data: [u8; 31],
}

impl light_hasher::DataHasher for RegisteredUser {
    fn hash<H: light_hasher::Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        let truncated_user_pubkey =
            light_utils::hash_to_bn254_field_size_be(&self.user_pubkey.to_bytes())
                .unwrap()
                .0;

        H::hashv(&[truncated_user_pubkey.as_slice(), self.data.as_slice()])
    }
}

#[derive(Accounts)]
pub struct CreateCompressedPda<'info> {
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
}
