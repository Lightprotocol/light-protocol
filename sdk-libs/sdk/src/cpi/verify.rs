use light_compressed_account::{
    compressed_account::ReadOnlyCompressedAccount,
    instruction_data::{
        cpi_context::CompressedCpiContext,
        data::{NewAddressParamsPacked, ReadOnlyAddress},
        invoke_cpi::InstructionDataInvokeCpi,
        with_account_info::CompressedAccountInfo,
    },
};

use crate::{
    account_info::AccountInfoTrait,
    cpi::accounts::CompressionCpiAccounts,
    error::{LightSdkError, Result},
    find_cpi_signer_macro, invoke_signed, AccountInfo, AccountMeta, AddressProof, AnchorSerialize,
    Instruction, Pubkey, ValidityProof, CPI_AUTHORITY_PDA_SEED, PROGRAM_ID_LIGHT_SYSTEM,
};

#[derive(Debug, Default, PartialEq, Clone)]
pub struct CompressionInstruction {
    pub proof: ValidityProof,
    pub account_infos: Option<Vec<CompressedAccountInfo>>,
    pub read_only_accounts: Option<Vec<ReadOnlyCompressedAccount>>,
    pub new_addresses: Option<Vec<NewAddressParamsPacked>>,
    pub read_only_address: Option<Vec<ReadOnlyAddress>>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CompressedCpiContext>,
}

impl CompressionInstruction {
    pub fn new(proof: ValidityProof, account_infos: Vec<CompressedAccountInfo>) -> Self {
        Self {
            proof,
            account_infos: Some(account_infos),
            ..Default::default()
        }
    }

    pub fn new_with_address(
        proof: AddressProof,
        account_infos: Vec<CompressedAccountInfo>,
        new_addresses: Vec<NewAddressParamsPacked>,
    ) -> Self {
        Self {
            proof: proof.into(),
            account_infos: Some(account_infos),
            new_addresses: Some(new_addresses),
            ..Default::default()
        }
    }
}

pub fn verify_compression_instruction(
    light_cpi_accounts: &CompressionCpiAccounts,
    instruction: CompressionInstruction,
) -> Result<()> {
    let owner = *light_cpi_accounts.invoking_program().key;
    let (input_compressed_accounts_with_merkle_context, output_compressed_accounts) =
        if let Some(account_infos) = instruction.account_infos.as_ref() {
            let mut input_compressed_accounts_with_merkle_context =
                Vec::with_capacity(account_infos.len());
            let mut output_compressed_accounts = Vec::with_capacity(account_infos.len());
            for account_info in account_infos.iter() {
                if let Some(input_account) = account_info.input_compressed_account(owner)? {
                    input_compressed_accounts_with_merkle_context.push(input_account);
                }
                if let Some(output_account) = account_info.output_compressed_account(owner)? {
                    output_compressed_accounts.push(output_account);
                }
            }
            (
                input_compressed_accounts_with_merkle_context,
                output_compressed_accounts,
            )
        } else {
            (vec![], vec![])
        };
    #[cfg(not(feature = "v2"))]
    if instruction.read_only_accounts.is_some() {
        unimplemented!("read_only_accounts are only supported with v2 soon on Devnet.");
    }
    #[cfg(not(feature = "v2"))]
    if instruction.read_only_address.is_some() {
        unimplemented!("read_only_addresses are only supported with v2 soon on Devnet.");
    }

    let instruction = InstructionDataInvokeCpi {
        proof: instruction.proof.into(),
        new_address_params: instruction.new_addresses.unwrap_or_default(),
        relay_fee: None,
        input_compressed_accounts_with_merkle_context,
        output_compressed_accounts,
        compress_or_decompress_lamports: instruction.compress_or_decompress_lamports,
        is_compress: instruction.is_compress,
        cpi_context: instruction.cpi_context,
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
    let account_infos: Vec<AccountInfo> = light_system_accounts.to_account_infos();

    let account_metas: Vec<AccountMeta> = light_system_accounts.to_account_metas();
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
