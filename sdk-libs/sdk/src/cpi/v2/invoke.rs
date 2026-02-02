use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
use light_sdk_types::CpiSigner;

#[cfg(feature = "cpi-context")]
use super::lowlevel::CompressedCpiContext;
use super::lowlevel::{to_account_metas, InAccount, InstructionDataInvokeCpiWithReadOnly};
#[cfg(feature = "poseidon")]
use crate::{account::poseidon::LightAccount as LightAccountPoseidon, DataHasher};
use crate::{
    account::LightAccount,
    cpi::{account::CpiAccountsTrait, instruction::LightCpiInstruction, v2::CpiAccounts},
    error::LightSdkError,
    instruction::account_info::CompressedAccountInfoTrait,
    AccountInfo, AccountMeta, AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

impl<'info> CpiAccountsTrait<'info> for CpiAccounts<'_, 'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        self.to_account_infos()
    }

    fn to_account_metas(&self) -> Result<Vec<AccountMeta>, ProgramError> {
        to_account_metas(self).map_err(ProgramError::from)
    }

    fn get_mode(&self) -> Option<u8> {
        Some(1) // v2 mode
    }
}

impl LightCpiInstruction for InstructionDataInvokeCpiWithReadOnly {
    fn new_cpi(cpi_signer: CpiSigner, proof: ValidityProof) -> Self {
        Self {
            bump: cpi_signer.bump,
            invoking_program_id: cpi_signer.program_id.into(),
            proof: proof.into(),
            mode: 1,
            ..Default::default()
        }
    }

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

    #[cfg(feature = "cpi-context")]
    fn write_to_cpi_context_first(self) -> Self {
        self.write_to_cpi_context_first()
    }

    #[cfg(feature = "cpi-context")]
    fn write_to_cpi_context_set(self) -> Self {
        self.write_to_cpi_context_set()
    }

    #[cfg(feature = "cpi-context")]
    fn execute_with_cpi_context(self) -> Self {
        self.execute_with_cpi_context()
    }

    fn get_mode(&self) -> u8 {
        self.mode
    }

    #[cfg(feature = "cpi-context")]
    fn get_with_cpi_context(&self) -> bool {
        self.with_cpi_context
    }

    #[cfg(feature = "cpi-context")]
    fn get_cpi_context(&self) -> &CompressedCpiContext {
        &self.cpi_context
    }

    fn get_bump(&self) -> u8 {
        self.bump
    }

    #[cfg(feature = "cpi-context")]
    fn has_read_only_accounts(&self) -> bool {
        !self.read_only_accounts.is_empty()
    }
}

impl LightCpiInstruction for InstructionDataInvokeCpiWithAccountInfo {
    fn new_cpi(cpi_signer: CpiSigner, proof: ValidityProof) -> Self {
        Self {
            bump: cpi_signer.bump,
            invoking_program_id: cpi_signer.program_id.into(),
            proof: proof.into(),
            mode: 1,
            ..Default::default()
        }
    }

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

    #[cfg(feature = "cpi-context")]
    fn write_to_cpi_context_first(self) -> Self {
        self.write_to_cpi_context_first()
    }

    #[cfg(feature = "cpi-context")]
    fn write_to_cpi_context_set(self) -> Self {
        self.write_to_cpi_context_set()
    }

    #[cfg(feature = "cpi-context")]
    fn execute_with_cpi_context(self) -> Self {
        self.execute_with_cpi_context()
    }

    fn get_mode(&self) -> u8 {
        self.mode
    }

    #[cfg(feature = "cpi-context")]
    fn get_with_cpi_context(&self) -> bool {
        self.with_cpi_context
    }

    #[cfg(feature = "cpi-context")]
    fn get_cpi_context(&self) -> &CompressedCpiContext {
        &self.cpi_context
    }

    fn get_bump(&self) -> u8 {
        self.bump
    }

    #[cfg(feature = "cpi-context")]
    fn has_read_only_accounts(&self) -> bool {
        !self.read_only_accounts.is_empty()
    }
}
