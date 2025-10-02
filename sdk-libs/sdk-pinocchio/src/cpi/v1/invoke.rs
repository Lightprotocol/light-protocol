use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, invoke_cpi::InstructionDataInvokeCpi,
};
use light_sdk_types::instruction::account_info::CompressedAccountInfoTrait;

use crate::{
    account::LightAccount,
    cpi::traits::{LightCpiInstruction, LightInstructionData},
    error::LightSdkError,
    BorshDeserialize, BorshSerialize, CpiSigner, LightDiscriminator,
};

/// V1 wrapper struct for InstructionDataInvokeCpi with CpiSigner
#[derive(Clone)]
pub struct LightSystemProgramCpi {
    cpi_signer: CpiSigner,
    instruction_data: InstructionDataInvokeCpi,
}

impl LightSystemProgramCpi {
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
}

impl LightCpiInstruction for LightSystemProgramCpi {
    fn new_cpi(cpi_signer: CpiSigner, proof: ValidityProof) -> Self {
        Self {
            cpi_signer,
            instruction_data: InstructionDataInvokeCpi::new(proof.into()),
        }
    }

    fn with_light_account<A>(mut self, account: LightAccount<'_, A>) -> Result<Self, LightSdkError>
    where
        A: BorshSerialize
            + BorshDeserialize
            + LightDiscriminator
            + light_hasher::DataHasher
            + Default,
    {
        use light_compressed_account::compressed_account::PackedCompressedAccountWithMerkleContext;

        // Convert LightAccount to account info
        let account_info = account.to_account_info()?;

        // Handle input accounts - convert to PackedCompressedAccountWithMerkleContext
        if let Some(input_account) = account_info
            .input_compressed_account(self.cpi_signer.program_id.into())
            .map_err(LightSdkError::from)?
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
            .map_err(LightSdkError::from)?
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
}

// Manual BorshSerialize implementation that only serializes instruction_data
impl BorshSerialize for LightSystemProgramCpi {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.instruction_data.serialize(writer)
    }
}

impl light_compressed_account::InstructionDiscriminator for LightSystemProgramCpi {
    fn discriminator(&self) -> &'static [u8] {
        self.instruction_data.discriminator()
    }
}

impl LightInstructionData for LightSystemProgramCpi {
    fn data(&self) -> Result<Vec<u8>, light_compressed_account::CompressedAccountError> {
        self.instruction_data.data()
    }
}
