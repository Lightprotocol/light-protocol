//! V2 LightSystemProgramCpi implementation optimized for Compressed Pdas.
//!
//! InstructionDataInvokeCpiWithReadOnly provides more flexibility
//! for complex operations such as changing the compressed account owner.

use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
pub use light_compressed_account::instruction_data::{
    cpi_context::*,
    with_account_info::{InstructionDataInvokeCpiWithAccountInfo as LightSystemProgramCpi, *},
    with_readonly::*,
};
use light_sdk_types::CpiSigner;

use crate::cpi::LightCpiInstruction;

impl LightCpiInstruction for InstructionDataInvokeCpiWithReadOnly {
    fn new_cpi(cpi_signer: CpiSigner, proof: ValidityProof) -> Self {
        Self {
            bump: cpi_signer.bump,
            invoking_program_id: cpi_signer.program_id.into(),
            proof: proof.0,
            mode: 1,
            ..Default::default()
        }
    }

    #[cfg(feature = "light-account")]
    fn with_light_account<A>(
        mut self,
        account: crate::LightAccount<A>,
    ) -> Result<Self, pinocchio::program_error::ProgramError>
    where
        A: crate::BorshSerialize
            + crate::BorshDeserialize
            + crate::LightDiscriminator
            + light_hasher::DataHasher
            + Default,
    {
        // Convert LightAccount to instruction data format
        let account_info = account
            .to_account_info()
            .map_err(|e| pinocchio::program_error::ProgramError::Custom(u64::from(e) as u32))?;

        // Handle input accounts
        if let Some(input) = account_info.input.as_ref() {
            let in_account = InAccount {
                discriminator: input.discriminator,
                data_hash: input.data_hash,
                merkle_context: input.merkle_context,
                root_index: input.root_index,
                lamports: input.lamports,
                address: account_info.address,
            };
            self.input_compressed_accounts.push(in_account);
        }

        // Handle output accounts
        if let Some(output) = account_info.output.as_ref() {
            let output_account = light_compressed_account::instruction_data::data::OutputCompressedAccountWithPackedContext {
                compressed_account: light_compressed_account::compressed_account::CompressedAccount {
                    owner: self.invoking_program_id.to_bytes().into(),
                    lamports: output.lamports,
                    address: account_info.address,
                    data: Some(light_compressed_account::compressed_account::CompressedAccountData {
                        discriminator: output.discriminator,
                        data: output.data.clone(),
                        data_hash: output.data_hash,
                    }),
                },
                merkle_tree_index: output.output_merkle_tree_index,
            };
            self.output_compressed_accounts.push(output_account);
        }

        Ok(self)
    }

    fn get_mode(&self) -> u8 {
        self.mode
    }

    fn get_bump(&self) -> u8 {
        self.bump
    }
}

impl LightCpiInstruction for InstructionDataInvokeCpiWithAccountInfo {
    fn new_cpi(cpi_signer: CpiSigner, proof: ValidityProof) -> Self {
        Self {
            bump: cpi_signer.bump,
            invoking_program_id: cpi_signer.program_id.into(),
            proof: proof.0,
            mode: 1,
            ..Default::default()
        }
    }

    #[cfg(feature = "light-account")]
    fn with_light_account<A>(
        mut self,
        account: crate::LightAccount<A>,
    ) -> Result<Self, pinocchio::program_error::ProgramError>
    where
        A: crate::BorshSerialize
            + borsh::BorshDeserialize
            + crate::LightDiscriminator
            + light_hasher::DataHasher
            + Default,
    {
        // Convert LightAccount to instruction data format

        use pinocchio::program_error::ProgramError;
        let account_info = account
            .to_account_info()
            .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
        self.account_infos.push(account_info);
        Ok(self)
    }

    fn get_mode(&self) -> u8 {
        self.mode
    }

    fn get_bump(&self) -> u8 {
        self.bump
    }
}
