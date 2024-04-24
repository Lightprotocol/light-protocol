use anchor_lang::prelude::*;
use light_compressed_pda::{
    compressed_account::{derive_address, CompressedAccount, CompressedAccountData},
    compressed_cpi::CompressedCpiContext,
    utils::CompressedProof,
    InstructionDataTransfer, NewAddressParamsPacked,
};
use light_hasher::{errors::HasherError, DataHasher, Hasher, Poseidon};

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub enum CreatePdaMode {
    ProgramIsSigner,
    ProgramIsNotSigner,
    InvalidSignerSeeds,
}
/// create compressed pda data
/// transfer tokens
/// execute complete transaction
pub fn process_create_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    data: [u8; 31],
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    owner_program: Pubkey,
    cpi_context: CompressedCpiContext,
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
        CreatePdaMode::ProgramIsSigner => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                b"cpi_signer".as_slice(),
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
                b"cpi_signer1".as_slice(),
            )?;
        }
    }
    Ok(())
}

fn cpi_compressed_pda_transfer_as_non_program<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: CompressedAccount,
    cpi_context: CompressedCpiContext,
) -> Result<()> {
    msg!("cpi_compressed_pda_transfer_as_non_program");
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

    let cpi_accounts = light_compressed_pda::cpi::accounts::TransferInstruction {
        signer: ctx.accounts.signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: Some(ctx.accounts.self_program.to_account_info()),
        compressed_sol_pda: None,
        compression_recipient: None,
        system_program: None,
        cpi_signature_account: None,
    };
    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_pda_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    light_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs, Some(cpi_context))?;
    Ok(())
}

fn cpi_compressed_pda_transfer_as_program<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: CompressedAccount,
    cpi_context: CompressedCpiContext,
    bump: u8,
    signer_seed: &[u8],
) -> Result<()> {
    let local_bump = Pubkey::find_program_address(&[signer_seed], &crate::ID).1;
    let seeds: [&[u8]; 2] = [signer_seed, &[local_bump]];
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
        signer_seeds: Some(seeds.iter().map(|seed| seed.to_vec()).collect()),
    };
    // defining seeds again so that the cpi doesn't fail we want to test the check in the compressed pda program
    let seeds: [&[u8]; 2] = [b"cpi_signer".as_slice(), &[bump]];
    let mut inputs = Vec::new();
    InstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_compressed_pda::cpi::accounts::TransferInstruction {
        signer: ctx.accounts.cpi_signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: Some(ctx.accounts.self_program.to_account_info()),
        compressed_sol_pda: None,
        compression_recipient: None,
        system_program: None,
        cpi_signature_account: None,
    };

    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_pda_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    light_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs, Some(cpi_context))?;
    Ok(())
}

fn create_compressed_pda_data(
    data: [u8; 31],
    ctx: &Context<'_, '_, '_, '_, CreateCompressedPda<'_>>,
    new_address_params: &NewAddressParamsPacked,
    owner_program: &Pubkey,
) -> Result<CompressedAccount> {
    let timelock_compressed_pda = RegisteredUser {
        user_pubkey: *ctx.accounts.signer.key,
        data,
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
        owner: *owner_program, // should be crate::ID, test provides an invalid owner
        lamports: 0,
        address: Some(derive_address),
        data: Some(compressed_account_data),
    })
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct RegisteredUser {
    pub user_pubkey: Pubkey,
    pub data: [u8; 31],
}

impl light_hasher::DataHasher for RegisteredUser {
    fn hash(&self) -> std::result::Result<[u8; 32], HasherError> {
        let truncated_user_pubkey =
            light_utils::hash_to_bn254_field_size_le(&self.user_pubkey.to_bytes())
                .unwrap()
                .0;

        Poseidon::hashv(&[truncated_user_pubkey.as_slice(), self.data.as_slice()])
    }
}

#[derive(Accounts)]
pub struct CreateCompressedPda<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub compressed_pda_program: Program<'info, light_compressed_pda::program::LightCompressedPda>,
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
    pub self_program: Program<'info, crate::program::ProgramOwnedAccountTest>,
    /// CHECK:
    pub cpi_signer: AccountInfo<'info>,
}
