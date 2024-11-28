#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_hasher::{DataHasher, Discriminator};
use solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    account::LightAccount,
    account_info::LightAccountInfo,
    address::PackedNewAddressParams,
    compressed_account::{
        OutputCompressedAccountWithPackedContext, PackedCompressedAccountWithMerkleContext,
    },
    error::LightSdkError,
    proof::{CompressedProof, ProofRpcResult},
    system_accounts::LightCpiAccounts,
    CPI_AUTHORITY_PDA_SEED, PROGRAM_ID_LIGHT_SYSTEM,
};

pub fn find_cpi_signer(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address([CPI_AUTHORITY_PDA_SEED].as_slice(), program_id).0
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompressedCpiContext {
    /// Is set by the program that is invoking the CPI to signal that is should
    /// set the cpi context.
    pub set_context: bool,
    /// Is set to wipe the cpi context since someone could have set it before
    /// with unrelated data.
    pub first_set_context: bool,
    /// Index of cpi context account in remaining accounts.
    pub cpi_context_account_index: u8,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct InstructionDataInvokeCpi {
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<PackedNewAddressParams>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CompressedCpiContext>,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InvokeCpi {
    pub inputs: Vec<u8>,
}

#[inline(always)]
pub fn invoke_cpi<'info>(
    account_infos: &[AccountInfo<'info>],
    accounts_metas: Vec<AccountMeta>,
    inputs: Vec<u8>,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction_data = InvokeCpi { inputs };

    // `InvokeCpi`'s discriminator
    let mut data = [49, 212, 191, 129, 39, 194, 43, 196].to_vec();
    data.extend(instruction_data.try_to_vec()?);

    #[cfg(feature = "anchor")]
    {
        anchor_lang::prelude::msg!("ACCOUNT METAS (len: {}):", accounts_metas.len(),);
        for (i, acc_meta) in accounts_metas.iter().enumerate() {
            anchor_lang::prelude::msg!("{}: {:?}", i, acc_meta);
        }
    }

    let instruction = Instruction {
        program_id: PROGRAM_ID_LIGHT_SYSTEM,
        accounts: accounts_metas,
        data,
    };
    invoke_signed(&instruction, account_infos, signer_seeds)?;

    Ok(())
}

/// Invokes the light system program to verify and apply a zk-compressed state
/// transition. Serializes CPI instruction data, configures necessary accounts,
/// and executes the CPI.
fn verify<'c, 'info, T>(
    light_system_accounts: &'c LightCpiAccounts<'c, 'info>,
    inputs: &T,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError>
where
    T: AnchorSerialize,
{
    if light_system_accounts.light_system_program().key != &PROGRAM_ID_LIGHT_SYSTEM {
        return Err(LightSdkError::InvalidLightSystemProgram.into());
    }

    let inputs = inputs.try_to_vec()?;

    let (account_infos, account_metas) = light_system_accounts.setup_cpi_accounts();
    invoke_cpi(&account_infos, account_metas, inputs, signer_seeds)?;
    Ok(())
}

pub fn verify_light_account_infos<'a, 'b, 'c, 'info>(
    light_cpi_accounts: &'c LightCpiAccounts<'c, 'info>,
    proof: Option<ProofRpcResult>,
    light_accounts: &[LightAccountInfo],
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<(), ProgramError> {
    let bump = Pubkey::find_program_address(
        &[CPI_AUTHORITY_PDA_SEED],
        light_cpi_accounts.invoking_program().key,
    )
    .1;
    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];

    let mut new_address_params = Vec::with_capacity(light_accounts.len());
    let mut input_compressed_accounts_with_merkle_context =
        Vec::with_capacity(light_accounts.len());
    let mut output_compressed_accounts = Vec::with_capacity(light_accounts.len());

    for light_account in light_accounts.iter() {
        if let Some(new_address_param) = light_account.new_address_params() {
            new_address_params.push(new_address_param);
        }
        if let Some(input_account) = light_account.input_compressed_account()? {
            input_compressed_accounts_with_merkle_context.push(input_account);
        }
        if let Some(output_account) = light_account.output_compressed_account()? {
            output_compressed_accounts.push(output_account);
        }
    }

    let instruction = InstructionDataInvokeCpi {
        proof: proof.map(|proof| proof.proof),
        new_address_params,
        relay_fee: None,
        input_compressed_accounts_with_merkle_context,
        output_compressed_accounts,
        compress_or_decompress_lamports,
        is_compress,
        cpi_context,
    };

    verify(light_cpi_accounts, &instruction, &[&signer_seeds[..]])?;

    Ok(())
}

pub fn verify_light_accounts<'a, 'b, 'c, 'info, T>(
    light_cpi_accounts: &'c LightCpiAccounts<'c, 'info>,
    proof: Option<ProofRpcResult>,
    light_accounts: &[LightAccount<T>],
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<(), ProgramError>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    let bump = Pubkey::find_program_address(
        &[CPI_AUTHORITY_PDA_SEED],
        light_cpi_accounts.invoking_program().key,
    )
    .1;
    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];

    let mut new_address_params = Vec::with_capacity(light_accounts.len());
    let mut input_compressed_accounts_with_merkle_context =
        Vec::with_capacity(light_accounts.len());
    let mut output_compressed_accounts = Vec::with_capacity(light_accounts.len());

    for light_account in light_accounts.iter() {
        if let Some(new_address_param) = light_account.new_address_params() {
            new_address_params.push(new_address_param);
        }
        if let Some(input_account) = light_account.input_compressed_account()? {
            input_compressed_accounts_with_merkle_context.push(input_account);
        }
        if let Some(output_account) = light_account.output_compressed_account()? {
            output_compressed_accounts.push(output_account);
        }
    }

    let instruction = InstructionDataInvokeCpi {
        proof: proof.map(|proof| proof.proof),
        new_address_params,
        relay_fee: None,
        input_compressed_accounts_with_merkle_context,
        output_compressed_accounts,
        compress_or_decompress_lamports,
        is_compress,
        cpi_context,
    };

    verify(light_cpi_accounts, &instruction, &[&signer_seeds[..]])?;

    Ok(())
}
