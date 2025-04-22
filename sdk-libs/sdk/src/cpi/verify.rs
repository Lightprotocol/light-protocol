use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, cpi_context::CompressedCpiContext,
    data::NewAddressParamsPacked, invoke_cpi::InstructionDataInvokeCpi,
    with_account_info::CompressedAccountInfo,
};

use crate::{
    account_info::AccountInfoTrait,
    cpi::accounts::CompressionCpiAccounts,
    error::{LightSdkError, Result},
    find_cpi_signer_macro, invoke_signed, AccountInfo, AccountMeta, AnchorSerialize, Instruction,
    Pubkey, CPI_AUTHORITY_PDA_SEED, PROGRAM_ID_LIGHT_SYSTEM,
};

pub fn verify_compressed_account_infos(
    light_cpi_accounts: &CompressionCpiAccounts,
    proof: Option<CompressedProof>,
    light_accounts: &[CompressedAccountInfo],
    new_address_params: Option<Vec<NewAddressParamsPacked>>,
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    let mut input_compressed_accounts_with_merkle_context =
        Vec::with_capacity(light_accounts.len());
    let mut output_compressed_accounts = Vec::with_capacity(light_accounts.len());
    let owner = *light_cpi_accounts.invoking_program().key;
    for light_account in light_accounts.iter() {
        if let Some(input_account) = light_account.input_compressed_account(owner)? {
            input_compressed_accounts_with_merkle_context.push(input_account);
        }
        if let Some(output_account) = light_account.output_compressed_account(owner)? {
            output_compressed_accounts.push(output_account);
        }
    }

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
    verify_borsh(light_cpi_accounts, &instruction)
}

/// Invokes the light system program to verify and apply a zk-compressed state
/// transition. Serializes CPI instruction data, configures necessary accounts,
/// and executes the CPI.
pub fn verify_borsh<T>(light_system_accounts: &CompressionCpiAccounts, inputs: &T) -> Result<()>
where
    T: AnchorSerialize,
{
    let inputs = inputs.try_to_vec().map_err(|_| LightSdkError::Borsh)?;

    let mut data = Vec::with_capacity(8 + 4 + inputs.len());
    data.extend_from_slice(&light_compressed_account::discriminators::DISCRIMINATOR_INVOKE_CPI);
    data.extend_from_slice(&(inputs.len() as u32).to_le_bytes());
    data.extend(inputs);
    verify_system_info(light_system_accounts, data)
}

pub fn verify_system_info(
    light_system_accounts: &CompressionCpiAccounts,
    data: Vec<u8>,
) -> Result<()> {
    let account_infos = light_system_accounts.to_account_infos();

    let account_metas = light_system_accounts.to_account_metas();
    invoke_light_system_program(
        light_system_accounts.self_program_id(),
        &account_infos,
        account_metas,
        data,
    )
}

#[inline(always)]
pub fn invoke_light_system_program(
    invoking_program_id: &Pubkey,
    account_infos: &[AccountInfo],
    account_metas: Vec<AccountMeta>,
    data: Vec<u8>,
) -> Result<()> {
    let instruction = Instruction {
        program_id: PROGRAM_ID_LIGHT_SYSTEM,
        accounts: account_metas,
        data,
    };

    let (_authority, bump) = find_cpi_signer_macro!(invoking_program_id);
    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];

    // TODO: restore but not a priority it is a convenience check
    // It's index 0 for small instruction accounts.
    // if *account_infos[1].key != authority {
    //     #[cfg(feature = "anchor")]
    //     anchor_lang::prelude::msg!(
    //         "System program signer authority is invalid. Expected {:?}, found {:?}",
    //         authority,
    //         account_infos[1].key
    //     );
    //     #[cfg(feature = "anchor")]
    //     anchor_lang::prelude::msg!(
    //         "Seeds to derive expected pubkey: [CPI_AUTHORITY_PDA_SEED] {:?}",
    //         [CPI_AUTHORITY_PDA_SEED]
    //     );
    //     return Err(LightSdkError::InvalidCpiSignerAccount);
    // }

    invoke_signed(&instruction, account_infos, &[signer_seeds.as_slice()])?;
    Ok(())
}
