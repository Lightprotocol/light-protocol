use light_compressed_account::{
    compressed_account::ReadOnlyCompressedAccount,
    instruction_data::{
        cpi_context::CompressedCpiContext,
        data::{NewAddressParamsPacked, ReadOnlyAddress},
        invoke_cpi::InstructionDataInvokeCpi,
        with_account_info::CompressedAccountInfo,
    },
};
use light_sdk_types::constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID};

use crate::{
    cpi::{get_account_metas_from_config, CpiAccounts, CpiInstructionConfig},
    error::{LightSdkError, Result},
    instruction::{account_info::CompressedAccountInfoTrait, ValidityProof},
    invoke_signed,
    light_account_checks::AccountInfoTrait,
    AccountInfo, AnchorSerialize, Instruction,
};

#[derive(Debug, Default, PartialEq, Clone)]
pub struct CpiInputs {
    pub proof: ValidityProof,
    pub account_infos: Option<Vec<CompressedAccountInfo>>,
    pub read_only_accounts: Option<Vec<ReadOnlyCompressedAccount>>,
    pub new_addresses: Option<Vec<NewAddressParamsPacked>>,
    pub read_only_address: Option<Vec<ReadOnlyAddress>>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CompressedCpiContext>,
}

impl CpiInputs {
    pub fn new(proof: ValidityProof, account_infos: Vec<CompressedAccountInfo>) -> Self {
        Self {
            proof,
            account_infos: Some(account_infos),
            ..Default::default()
        }
    }

    pub fn new_with_address(
        proof: ValidityProof,
        account_infos: Vec<CompressedAccountInfo>,
        new_addresses: Vec<NewAddressParamsPacked>,
    ) -> Self {
        Self {
            proof,
            account_infos: Some(account_infos),
            new_addresses: Some(new_addresses),
            ..Default::default()
        }
    }

    pub fn invoke_light_system_program(self, cpi_accounts: CpiAccounts<'_, '_>) -> Result<()> {
        let bump = cpi_accounts.bump();
        let account_infos = cpi_accounts.to_account_infos();
        let instruction = create_light_system_progam_instruction_invoke_cpi(self, cpi_accounts)?;
        invoke_light_system_program(account_infos.as_slice(), instruction, bump)
    }
}

pub fn create_light_system_progam_instruction_invoke_cpi(
    cpi_inputs: CpiInputs,
    cpi_accounts: CpiAccounts<'_, '_>,
) -> Result<Instruction> {
    let owner = *cpi_accounts.invoking_program()?.key;
    let (input_compressed_accounts_with_merkle_context, output_compressed_accounts) =
        if let Some(account_infos) = cpi_inputs.account_infos.as_ref() {
            let mut input_compressed_accounts_with_merkle_context =
                Vec::with_capacity(account_infos.len());
            let mut output_compressed_accounts = Vec::with_capacity(account_infos.len());
            for account_info in account_infos.iter() {
                if let Some(input_account) =
                    account_info.input_compressed_account(owner.to_bytes().into())?
                {
                    input_compressed_accounts_with_merkle_context.push(input_account);
                }
                if let Some(output_account) =
                    account_info.output_compressed_account(owner.to_bytes().into())?
                {
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
    if cpi_inputs.read_only_accounts.is_some() {
        unimplemented!("read_only_accounts are only supported with v2 soon on Devnet.");
    }
    #[cfg(not(feature = "v2"))]
    if cpi_inputs.read_only_address.is_some() {
        unimplemented!("read_only_addresses are only supported with v2 soon on Devnet.");
    }

    let inputs = InstructionDataInvokeCpi {
        proof: cpi_inputs.proof.into(),
        new_address_params: cpi_inputs.new_addresses.unwrap_or_default(),
        relay_fee: None,
        input_compressed_accounts_with_merkle_context,
        output_compressed_accounts,
        compress_or_decompress_lamports: cpi_inputs.compress_or_decompress_lamports,
        is_compress: cpi_inputs.is_compress,
        cpi_context: cpi_inputs.cpi_context,
    };
    let inputs = inputs.try_to_vec().map_err(|_| LightSdkError::Borsh)?;

    let mut data = Vec::with_capacity(8 + 4 + inputs.len());
    data.extend_from_slice(&light_compressed_account::discriminators::DISCRIMINATOR_INVOKE_CPI);
    data.extend_from_slice(&(inputs.len() as u32).to_le_bytes());
    data.extend(inputs);

    // Use new config-based approach instead of expensive to_account_metas
    let config = CpiInstructionConfig {
        fee_payer: cpi_accounts.fee_payer().key().into(),
        cpi_signer: cpi_accounts.config.cpi_signer().into(),
        invoking_program: cpi_accounts.config.cpi_signer.program_id.into(),
        sol_pool_pda_pubkey: if cpi_accounts.config.sol_pool_pda {
            Some(cpi_accounts.sol_pool_pda()?.key().into())
        } else {
            None
        },
        sol_compression_recipient_pubkey: if cpi_accounts.config.sol_compression_recipient {
            Some(cpi_accounts.decompression_recipient()?.key().into())
        } else {
            None
        },
        cpi_context_pubkey: if cpi_accounts.config.cpi_context {
            Some(cpi_accounts.cpi_context()?.key().into())
        } else {
            None
        },
        packed_accounts: cpi_accounts.tree_accounts().unwrap_or(&[]),
    };

    let account_metas = get_account_metas_from_config(config);

    Ok(Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data,
    })
}

/// Invokes the light system program to verify and apply a zk-compressed state
/// transition. Serializes CPI instruction data, configures necessary accounts,
/// and executes the CPI.
pub fn verify_borsh<'a, T>(light_system_accounts: CpiAccounts<'a, 'a>, inputs: &T) -> Result<()>
where
    T: AnchorSerialize,
{
    let inputs = inputs.try_to_vec().map_err(|_| LightSdkError::Borsh)?;

    let mut data = Vec::with_capacity(8 + 4 + inputs.len());
    data.extend_from_slice(&light_compressed_account::discriminators::DISCRIMINATOR_INVOKE_CPI);
    data.extend_from_slice(&(inputs.len() as u32).to_le_bytes());
    data.extend(inputs);
    let account_infos = light_system_accounts.to_account_infos();

    let bump = light_system_accounts.bump();
    // Use new config-based approach instead of expensive to_account_metas
    let config = CpiInstructionConfig {
        fee_payer: light_system_accounts.fee_payer().key().into(),
        cpi_signer: light_system_accounts.config.cpi_signer().into(),
        invoking_program: light_system_accounts.config.cpi_signer.program_id.into(),
        sol_pool_pda_pubkey: if light_system_accounts.config.sol_pool_pda {
            Some(light_system_accounts.sol_pool_pda()?.key().into())
        } else {
            None
        },
        sol_compression_recipient_pubkey: if light_system_accounts.config.sol_compression_recipient
        {
            Some(
                light_system_accounts
                    .decompression_recipient()?
                    .key()
                    .into(),
            )
        } else {
            None
        },
        cpi_context_pubkey: if light_system_accounts.config.cpi_context {
            Some(light_system_accounts.cpi_context()?.key().into())
        } else {
            None
        },
        packed_accounts: light_system_accounts.tree_accounts().unwrap_or(&[]),
    };

    let account_metas = get_account_metas_from_config(config);
    let instruction = Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data,
    };
    invoke_light_system_program(account_infos.as_slice(), instruction, bump)
}

#[inline(always)]
pub fn invoke_light_system_program(
    account_infos: &[AccountInfo],
    instruction: Instruction,
    bump: u8,
) -> Result<()> {
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
