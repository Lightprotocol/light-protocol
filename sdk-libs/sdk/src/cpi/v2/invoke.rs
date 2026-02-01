use light_compressed_account::instruction_data::with_account_info::InstructionDataInvokeCpiWithAccountInfo;
use light_compressed_account::instruction_data::with_readonly::{
    InAccount, InstructionDataInvokeCpiWithReadOnly,
};
use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof,
    cpi_context::CompressedCpiContext,
};

#[cfg(feature = "poseidon")]
use crate::{account::poseidon::LightAccount as LightAccountPoseidon, DataHasher};
use crate::{
    account::LightAccount,
    cpi::{delegate_light_cpi, LightCpiInstruction},
    error::LightSdkError,
    instruction::account_info::CompressedAccountInfoTrait,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

impl LightCpiInstruction for InstructionDataInvokeCpiWithReadOnly {
    delegate_light_cpi!(InstructionDataInvokeCpiWithReadOnly);

    fn with_light_account<A>(mut self, account: LightAccount<A>) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
    {
        // Check if this is a read-only account
        if account.read_only_account_hash.is_some() {
            let read_only_account = account.to_packed_read_only_account()?;
            self.read_only_accounts.push(read_only_account);
            return Ok(self);
        }

        // Convert LightAccount to instruction data format
        let account_info = account.to_account_info()?;

        // Handle input accounts
        if let Some(input_account) = account_info
            .input_compressed_account(self.invoking_program_id.to_bytes().into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            // Convert to InAccount format
            let in_account = InAccount {
                discriminator: input_account
                    .compressed_account
                    .data
                    .as_ref()
                    .map(|d| d.discriminator)
                    .unwrap_or_default(),
                data_hash: input_account
                    .compressed_account
                    .data
                    .as_ref()
                    .map(|d| d.data_hash)
                    .unwrap_or_default(),
                merkle_context: input_account.merkle_context,
                root_index: input_account.root_index,
                lamports: input_account.compressed_account.lamports,
                address: input_account.compressed_account.address,
            };
            self.input_compressed_accounts.push(in_account);
        }

        // Handle output accounts
        if let Some(output_account) = account_info
            .output_compressed_account(self.invoking_program_id.to_bytes().into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            self.output_compressed_accounts.push(output_account);
        }

        Ok(self)
    }

    #[cfg(feature = "poseidon")]
    fn with_light_account_poseidon<A>(
        mut self,
        account: LightAccountPoseidon<A>,
    ) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + DataHasher + LightDiscriminator + Default,
    {
        // Check if this is a read-only account
        if account.read_only_account_hash.is_some() {
            let read_only_account = account.to_packed_read_only_account()?;
            self.read_only_accounts.push(read_only_account);
            return Ok(self);
        }

        // Convert LightAccount to instruction data format
        let account_info = account.to_account_info()?;

        // Handle input accounts
        if let Some(input_account) = account_info
            .input_compressed_account(self.invoking_program_id.to_bytes().into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            // Convert to InAccount format
            let in_account = InAccount {
                discriminator: input_account
                    .compressed_account
                    .data
                    .as_ref()
                    .map(|d| d.discriminator)
                    .unwrap_or_default(),
                data_hash: input_account
                    .compressed_account
                    .data
                    .as_ref()
                    .map(|d| d.data_hash)
                    .unwrap_or_default(),
                merkle_context: input_account.merkle_context,
                root_index: input_account.root_index,
                lamports: input_account.compressed_account.lamports,
                address: input_account.compressed_account.address,
            };
            self.input_compressed_accounts.push(in_account);
        }

        // Handle output accounts
        if let Some(output_account) = account_info
            .output_compressed_account(self.invoking_program_id.to_bytes().into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            self.output_compressed_accounts.push(output_account);
        }

        Ok(self)
    }
}

impl LightCpiInstruction for InstructionDataInvokeCpiWithAccountInfo {
    delegate_light_cpi!(InstructionDataInvokeCpiWithAccountInfo);

    fn with_light_account<A>(mut self, account: LightAccount<A>) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
    {
        // Check if this is a read-only account
        if account.read_only_account_hash.is_some() {
            let read_only_account = account.to_packed_read_only_account()?;
            self.read_only_accounts.push(read_only_account);
            return Ok(self);
        }

        // Convert LightAccount to instruction data format
        let account_info = account.to_account_info()?;
        self.account_infos.push(account_info);
        Ok(self)
    }

    #[cfg(feature = "poseidon")]
    fn with_light_account_poseidon<A>(
        mut self,
        account: crate::account::poseidon::LightAccount<A>,
    ) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default,
    {
        // Check if this is a read-only account
        if account.read_only_account_hash.is_some() {
            let read_only_account = account.to_packed_read_only_account()?;
            self.read_only_accounts.push(read_only_account);
            return Ok(self);
        }

        // Convert LightAccount to instruction data format
        let account_info = account.to_account_info()?;
        self.account_infos.push(account_info);
        Ok(self)
    }
}
