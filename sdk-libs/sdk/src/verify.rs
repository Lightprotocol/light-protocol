use light_compressed_account::{
    compressed_account::PackedCompressedAccountWithMerkleContext,
    instruction_data::{
        compressed_proof::CompressedProof,
        data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
    },
};
use solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    pubkey::Pubkey,
};

use crate::{
    account_info::LightAccountInfo,
    error::{LightSdkError, Result},
    system_accounts::LightCpiAccounts,
    BorshDeserialize, BorshSerialize, CPI_AUTHORITY_PDA_SEED, PROGRAM_ID_LIGHT_SYSTEM,
};

pub fn find_cpi_signer(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address([CPI_AUTHORITY_PDA_SEED].as_slice(), program_id).0
}

#[macro_export]
macro_rules! find_cpi_signer_macro {
    ($program_id:expr) => {
        Pubkey::find_program_address([CPI_AUTHORITY_PDA_SEED].as_slice(), $program_id).0
    };
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
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

#[derive(Debug, PartialEq, Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct InstructionDataInvokeCpi {
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CompressedCpiContext>,
}

pub fn verify_light_account_infos(
    light_cpi_accounts: &LightCpiAccounts,
    proof: Option<CompressedProof>,
    light_account_infos: &[LightAccountInfo],
    new_address_params: Option<Vec<NewAddressParamsPacked>>,
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    // TODO: send bump with instruction data or hardcode (best generate with macro during compile time -> hardcode it this way)
    let bump = Pubkey::find_program_address(
        &[CPI_AUTHORITY_PDA_SEED],
        light_cpi_accounts.invoking_program().key,
    )
    .1;
    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];

    let mut input_compressed_accounts_with_merkle_context =
        Vec::with_capacity(light_account_infos.len());
    let mut output_compressed_accounts = Vec::with_capacity(light_account_infos.len());

    for light_account_info in light_account_infos.iter() {
        if let Some(input_account) = light_account_info.input_compressed_account()? {
            input_compressed_accounts_with_merkle_context.push(input_account);
        }
        if let Some(output_account) = light_account_info.output_compressed_account()? {
            output_compressed_accounts.push(output_account);
        }
    }

    // TODO: make e2e zero copy version
    let instruction = InstructionDataInvokeCpi {
        proof,
        new_address_params: new_address_params.unwrap_or_default(),
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

// // TODO: remove only verify light account infos should exist
// pub fn verify_light_accounts<T>(
//     light_cpi_accounts: &LightCpiAccounts,
//     proof: Option<ProofRpcResult>,
//     light_accounts: &[LightAccount<T>],
//     compress_or_decompress_lamports: Option<u64>,
//     is_compress: bool,
//     cpi_context: Option<CompressedCpiContext>,
// ) -> Result<()>
// where
//     T: BorshSerialize
//         + BorshDeserialize
//         + Clone
//         + DataHasher
//         + Default
//         + Discriminator
//         + std::fmt::Debug,
// {
//     // TODO: send bump with instruction data or hardcode (best generate with macro during compile time -> hardcode it this way)
//     let bump = Pubkey::find_program_address(
//         &[CPI_AUTHORITY_PDA_SEED],
//         light_cpi_accounts.invoking_program().key,
//     )
//     .1;
//     let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];

//     let mut new_address_params = Vec::with_capacity(light_accounts.len());
//     let mut input_compressed_accounts_with_merkle_context =
//         Vec::with_capacity(light_accounts.len());
//     let mut output_compressed_accounts = Vec::with_capacity(light_accounts.len());

//     for light_account in light_accounts.iter() {
//         if let Some(new_address_param) = light_account.new_address_params() {
//             new_address_params.push(new_address_param);
//         }
//         if let Some(input_account) = light_account.input_compressed_account()? {
//             input_compressed_accounts_with_merkle_context.push(input_account);
//         }
//         if let Some(output_account) = light_account.output_compressed_account()? {
//             output_compressed_accounts.push(output_account);
//         }
//     }

//     let instruction = InstructionDataInvokeCpi {
//         proof: proof.map(|proof| proof.proof),
//         new_address_params,
//         relay_fee: None,
//         input_compressed_accounts_with_merkle_context,
//         output_compressed_accounts,
//         compress_or_decompress_lamports,
//         is_compress,
//         cpi_context,
//     };

//     verify(light_cpi_accounts, &instruction, &[&signer_seeds[..]])?;

//     Ok(())
// }

/// Invokes the light system program to verify and apply a zk-compressed state
/// transition. Serializes CPI instruction data, configures necessary accounts,
/// and executes the CPI.
pub fn verify<T>(
    light_system_accounts: &LightCpiAccounts,
    inputs: &T,
    signer_seeds: &[&[&[u8]]],
) -> Result<()>
where
    T: BorshSerialize,
{
    // Probably unnecessary check, since we hardcode program id in instruction.
    if light_system_accounts.light_system_program().key != &PROGRAM_ID_LIGHT_SYSTEM {
        return Err(LightSdkError::InvalidLightSystemProgram);
    }
    let inputs = inputs.try_to_vec().map_err(|_| LightSdkError::Borsh)?;

    let account_infos = light_system_accounts.to_account_infos();
    let account_metas = light_system_accounts.to_account_metas();
    invoke_cpi(&account_infos, account_metas, inputs, signer_seeds)?;
    Ok(())
}

#[inline(always)]
pub fn invoke_cpi(
    account_infos: &[AccountInfo],
    accounts_metas: Vec<AccountMeta>,
    inputs: Vec<u8>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data = Vec::with_capacity(8 + 4 + inputs.len());
    // `InvokeCpi`'s discriminator
    data.extend_from_slice(&light_compressed_account::discriminators::DISCRIMINATOR_INVOKE_CPI);
    data.extend_from_slice(&(inputs.len() as u32).to_le_bytes());
    data.extend(inputs);
    solana_program::msg!(
        "account_infos {:?}",
        account_infos.iter().map(|x| x.key).collect::<Vec<_>>()
    );

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

pub fn verify_system_info(light_system_accounts: &LightCpiAccounts, data: Vec<u8>) -> Result<()> {
    let account_infos = light_system_accounts.to_account_infos();
    let account_metas = light_system_accounts.to_account_metas();
    invoke_system_info_cpi(
        light_system_accounts.invoking_program().key,
        &account_infos,
        account_metas,
        data,
    )
}

#[inline(always)]
pub fn invoke_system_info_cpi(
    invoking_program_id: &Pubkey,
    account_infos: &[AccountInfo],
    accounts_metas: Vec<AccountMeta>,
    data: Vec<u8>,
) -> Result<()> {
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
    // TODO: hardcode with macro.
    let (_, bump) = Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], invoking_program_id);
    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];

    invoke_signed(&instruction, account_infos, &[signer_seeds.as_slice()])?;
    Ok(())
}
