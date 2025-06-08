use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
    },
    discriminators::DISCRIMINATOR_INVOKE_CPI,
    instruction_data::{
        cpi_context::CompressedCpiContext,
        data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
        invoke_cpi::InstructionDataInvokeCpi,
        with_account_info::CompressedAccountInfo,
    },
};
use light_sdk_types::constants::{CPI_AUTHORITY_PDA_SEED, PROGRAM_ID_LIGHT_SYSTEM};
use pinocchio::{cpi::slice_invoke_signed, log::sol_log_compute_units, msg, pubkey::Pubkey};

use crate::{
    cpi::CpiAccounts,
    error::{LightSdkError, Result},
    BorshSerialize, ValidityProof,
};

// Trait to provide the missing methods for CompressedAccountInfo
pub trait CompressedAccountInfoExt {
    fn input_compressed_account(
        &self,
        owner: Pubkey,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>>;
    fn output_compressed_account(
        &self,
        owner: Pubkey,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>>;
}

impl CompressedAccountInfoExt for CompressedAccountInfo {
    fn input_compressed_account(
        &self,
        owner: Pubkey,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>> {
        match self.input.as_ref() {
            Some(input) => {
                let data = Some(CompressedAccountData {
                    discriminator: input.discriminator,
                    data: Vec::new(),
                    data_hash: input.data_hash,
                });
                Ok(Some(PackedCompressedAccountWithMerkleContext {
                    compressed_account: CompressedAccount {
                        owner: owner.into(),
                        lamports: input.lamports,
                        address: self.address,
                        data,
                    },
                    merkle_context: input.merkle_context,
                    root_index: input.root_index,
                    read_only: false,
                }))
            }
            None => Ok(None),
        }
    }

    fn output_compressed_account(
        &self,
        owner: Pubkey,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>> {
        match self.output.as_ref() {
            Some(output) => {
                let data = Some(CompressedAccountData {
                    discriminator: output.discriminator,
                    data: output.data.clone(),
                    data_hash: output.data_hash,
                });
                Ok(Some(OutputCompressedAccountWithPackedContext {
                    compressed_account: CompressedAccount {
                        owner: owner.into(),
                        lamports: output.lamports,
                        address: self.address,
                        data,
                    },
                    merkle_tree_index: output.output_merkle_tree_index,
                }))
            }
            None => Ok(None),
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct CpiInputs {
    pub proof: ValidityProof,
    pub account_infos: Option<Vec<CompressedAccountInfo>>,
    pub new_addresses: Option<Vec<NewAddressParamsPacked>>,
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

    pub fn invoke_light_system_program(self, cpi_accounts: CpiAccounts) -> Result<()> {
        light_system_progam_instruction_invoke_cpi(self, &cpi_accounts)
    }
}

pub fn light_system_progam_instruction_invoke_cpi(
    cpi_inputs: CpiInputs,
    cpi_accounts: &CpiAccounts,
) -> Result<()> {
    let owner = *cpi_accounts.invoking_program().key();
    let (input_compressed_accounts_with_merkle_context, output_compressed_accounts) =
        if let Some(account_infos) = cpi_inputs.account_infos.as_ref() {
            let mut input_compressed_accounts_with_merkle_context =
                Vec::with_capacity(account_infos.len());
            let mut output_compressed_accounts = Vec::with_capacity(account_infos.len());
            for account_info in account_infos.iter() {
                if let Some(input_account) =
                    CompressedAccountInfoExt::input_compressed_account(account_info, owner)?
                {
                    input_compressed_accounts_with_merkle_context.push(input_account);
                }
                if let Some(output_account) =
                    CompressedAccountInfoExt::output_compressed_account(account_info, owner)?
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

    let inputs = InstructionDataInvokeCpi {
        proof: cpi_inputs.proof.0,
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
    data.extend_from_slice(&DISCRIMINATOR_INVOKE_CPI);
    data.extend_from_slice(&(inputs.len() as u32).to_le_bytes());
    data.extend(inputs);

    let account_metas: Vec<pinocchio::instruction::AccountMeta> = cpi_accounts.to_account_metas();

    // Create instruction with owned data and immediately invoke it
    use pinocchio::instruction::{Instruction, Seed, Signer};
    sol_log_compute_units();

    // Use the precomputed CPI signer and bump from the config
    let bump = cpi_accounts.bump();
    sol_log_compute_units();
    let bump_seed = [bump];
    let seed_array = [
        Seed::from(CPI_AUTHORITY_PDA_SEED),
        Seed::from(bump_seed.as_slice()),
    ];
    let signer = Signer::from(&seed_array);
    sol_log_compute_units();

    let instruction = Instruction {
        program_id: &PROGRAM_ID_LIGHT_SYSTEM,
        accounts: &account_metas,
        data: &data,
    };
    sol_log_compute_units();
    let account_infos = cpi_accounts.to_account_infos();
    sol_log_compute_units();

    match slice_invoke_signed(&instruction, &account_infos, &[signer]) {
        Ok(()) => {}
        Err(e) => {
            msg!(format!("slice_invoke_signed failed: {:?}", e).as_str());
            return Err(LightSdkError::ProgramError(e));
        }
    }
    Ok(())
}
