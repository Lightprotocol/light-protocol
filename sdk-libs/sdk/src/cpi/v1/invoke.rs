use light_compressed_account::compressed_account::PackedCompressedAccountWithMerkleContext;

#[cfg(feature = "poseidon")]
use crate::{account::poseidon::LightAccount as LightAccountPoseidon, DataHasher};
use crate::{
    account::LightAccount,
    cpi::instruction::WithLightAccount,
    error::LightSdkError,
    instruction::account_info::CompressedAccountInfoTrait,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

// Re-export LightSystemProgramCpi from interface
pub use light_sdk_interface::cpi::v1::LightSystemProgramCpi;

impl WithLightAccount for LightSystemProgramCpi {
    fn with_light_account<A>(mut self, account: LightAccount<A>) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
    {
        // Convert LightAccount to account info
        let account_info = account.to_account_info()?;

        // Handle input accounts - convert to PackedCompressedAccountWithMerkleContext
        if let Some(input_account) = account_info
            .input_compressed_account(self.cpi_signer().program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            let packed_input = PackedCompressedAccountWithMerkleContext {
                compressed_account: input_account.compressed_account,
                merkle_context: input_account.merkle_context,
                root_index: input_account.root_index,
                read_only: false, // Default to false for v1
            };
            self.instruction_data_mut()
                .input_compressed_accounts_with_merkle_context
                .push(packed_input);
        }

        // Handle output accounts
        if let Some(output_account) = account_info
            .output_compressed_account(self.cpi_signer().program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            self.instruction_data_mut()
                .output_compressed_accounts
                .push(output_account);
        }

        Ok(self)
    }

    #[cfg(feature = "poseidon")]
    fn with_light_account_poseidon<A>(
        mut self,
        account: LightAccountPoseidon<A>,
    ) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default,
    {
        // Convert LightAccount to account info
        let account_info = account.to_account_info()?;

        // Handle input accounts - convert to PackedCompressedAccountWithMerkleContext
        if let Some(input_account) = account_info
            .input_compressed_account(self.cpi_signer().program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            let packed_input = PackedCompressedAccountWithMerkleContext {
                compressed_account: input_account.compressed_account,
                merkle_context: input_account.merkle_context,
                root_index: input_account.root_index,
                read_only: false, // Default to false for v1
            };
            self.instruction_data_mut()
                .input_compressed_accounts_with_merkle_context
                .push(packed_input);
        }

        // Handle output accounts
        if let Some(output_account) = account_info
            .output_compressed_account(self.cpi_signer().program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            self.instruction_data_mut()
                .output_compressed_accounts
                .push(output_account);
        }

        Ok(self)
    }
}
