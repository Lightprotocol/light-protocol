use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, invoke_cpi::InstructionDataInvokeCpi,
};

use super::traits::{LightCpiInstruction, LightInstructionData};
use crate::{
    account::{poseidon::LightAccount as LightAccountPoseidon, LightAccount},
    cpi::CpiSigner,
    error::LightSdkError,
    instruction::account_info::CompressedAccountInfoTrait,
    AnchorDeserialize, AnchorSerialize, DataHasher, LightDiscriminator, ProgramError,
};

/// V1 wrapper struct for InstructionDataInvokeCpi with CpiSigner
#[derive(Clone)]
pub struct LightSystemProgramCpiV1 {
    cpi_signer: CpiSigner,
    instruction_data: InstructionDataInvokeCpi,
}

impl LightSystemProgramCpiV1 {
    pub fn with_new_addresses(
        mut self,
        new_address_params: &[light_compressed_account::instruction_data::data::NewAddressParamsPacked],
    ) -> Self {
        self.instruction_data = self.instruction_data.with_new_addresses(new_address_params);
        self
    }

    pub fn with_input_compressed_accounts_with_merkle_context(
        mut self,
        input_compressed_accounts_with_merkle_context: &[light_compressed_account::compressed_account::PackedCompressedAccountWithMerkleContext],
    ) -> Self {
        self.instruction_data = self
            .instruction_data
            .with_input_compressed_accounts_with_merkle_context(
                input_compressed_accounts_with_merkle_context,
            );
        self
    }

    pub fn with_output_compressed_accounts(
        mut self,
        output_compressed_accounts: &[light_compressed_account::instruction_data::data::OutputCompressedAccountWithPackedContext],
    ) -> Self {
        self.instruction_data = self
            .instruction_data
            .with_output_compressed_accounts(output_compressed_accounts);
        self
    }

    pub fn compress_lamports(mut self, lamports: u64) -> Self {
        self.instruction_data = self.instruction_data.compress_lamports(lamports);
        self
    }

    pub fn decompress_lamports(mut self, lamports: u64) -> Self {
        self.instruction_data = self.instruction_data.decompress_lamports(lamports);
        self
    }

    pub fn write_to_cpi_context_set(mut self) -> Self {
        self.instruction_data = self.instruction_data.write_to_cpi_context_set();
        self
    }

    pub fn write_to_cpi_context_first(mut self) -> Self {
        self.instruction_data = self.instruction_data.write_to_cpi_context_first();
        self
    }

    pub fn with_cpi_context(
        mut self,
        cpi_context: light_compressed_account::instruction_data::cpi_context::CompressedCpiContext,
    ) -> Self {
        self.instruction_data = self.instruction_data.with_cpi_context(cpi_context);
        self
    }
}

impl LightCpiInstruction for LightSystemProgramCpiV1 {
    fn new_cpi(cpi_signer: CpiSigner, proof: ValidityProof) -> Self {
        Self {
            cpi_signer,
            instruction_data: InstructionDataInvokeCpi::new(proof.into()),
        }
    }

    fn with_light_account<A>(mut self, account: LightAccount<'_, A>) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
    {
        use light_compressed_account::compressed_account::PackedCompressedAccountWithMerkleContext;

        // Convert LightAccount to account info
        let account_info = account.to_account_info()?;

        // Handle input accounts - convert to PackedCompressedAccountWithMerkleContext
        if let Some(input_account) = account_info
            .input_compressed_account(self.cpi_signer.program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            let packed_input = PackedCompressedAccountWithMerkleContext {
                compressed_account: input_account.compressed_account,
                merkle_context: input_account.merkle_context,
                root_index: input_account.root_index,
                read_only: false, // Default to false for v1
            };
            self.instruction_data
                .input_compressed_accounts_with_merkle_context
                .push(packed_input);
        }

        // Handle output accounts
        if let Some(output_account) = account_info
            .output_compressed_account(self.cpi_signer.program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            self.instruction_data
                .output_compressed_accounts
                .push(output_account);
        }

        Ok(self)
    }

    fn with_light_account_poseidon<A>(
        mut self,
        account: LightAccountPoseidon<'_, A>,
    ) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default,
    {
        use light_compressed_account::compressed_account::PackedCompressedAccountWithMerkleContext;

        // Convert LightAccount to account info
        let account_info = account.to_account_info()?;

        // Handle input accounts - convert to PackedCompressedAccountWithMerkleContext
        if let Some(input_account) = account_info
            .input_compressed_account(self.cpi_signer.program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            let packed_input = PackedCompressedAccountWithMerkleContext {
                compressed_account: input_account.compressed_account,
                merkle_context: input_account.merkle_context,
                root_index: input_account.root_index,
                read_only: false, // Default to false for v1
            };
            self.instruction_data
                .input_compressed_accounts_with_merkle_context
                .push(packed_input);
        }

        // Handle output accounts
        if let Some(output_account) = account_info
            .output_compressed_account(self.cpi_signer.program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            self.instruction_data
                .output_compressed_accounts
                .push(output_account);
        }

        Ok(self)
    }

    fn get_mode(&self) -> u8 {
        0 // V1 uses regular mode by default
    }

    fn get_bump(&self) -> u8 {
        self.cpi_signer.bump
    }

    #[cfg(feature = "v2")]
    fn write_to_cpi_context_first(mut self) -> Self {
        self.instruction_data = self.instruction_data.write_to_cpi_context_first();
        self
    }

    #[cfg(feature = "v2")]
    fn write_to_cpi_context_set(mut self) -> Self {
        self.instruction_data = self.instruction_data.write_to_cpi_context_set();
        self
    }

    #[cfg(feature = "v2")]
    fn execute_with_cpi_context(self) -> Self {
        // V1 doesn't have a direct execute context, just return self
        // The execute happens through the invoke call
        self
    }

    #[cfg(feature = "v2")]
    fn get_with_cpi_context(&self) -> bool {
        self.instruction_data.cpi_context.is_some()
    }

    #[cfg(feature = "v2")]
    fn get_cpi_context(
        &self,
    ) -> &light_compressed_account::instruction_data::cpi_context::CompressedCpiContext {
        use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
        // Use a static default with all fields set to false/0
        static DEFAULT: CompressedCpiContext = CompressedCpiContext {
            set_context: false,
            first_set_context: false,
            cpi_context_account_index: 0,
        };
        self.instruction_data
            .cpi_context
            .as_ref()
            .unwrap_or(&DEFAULT)
    }
}

// Manual BorshSerialize implementation that only serializes instruction_data
impl AnchorSerialize for LightSystemProgramCpiV1 {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.instruction_data.serialize(writer)
    }
}

impl light_compressed_account::InstructionDiscriminator for LightSystemProgramCpiV1 {
    fn discriminator(&self) -> &'static [u8] {
        self.instruction_data.discriminator()
    }
}

impl LightInstructionData for LightSystemProgramCpiV1 {
    fn data(&self) -> Result<Vec<u8>, light_compressed_account::CompressedAccountError> {
        self.instruction_data.data()
    }
}
