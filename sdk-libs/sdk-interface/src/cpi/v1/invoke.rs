use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, invoke_cpi::InstructionDataInvokeCpi,
};

use crate::{
    cpi::{instruction::LightCpiInstruction, invoke::LightInstructionData, CpiSigner},
    AnchorSerialize,
};

/// Light system program CPI instruction data builder.
///
/// Use this builder to construct instructions for compressed account operations:
/// creating, updating, closing accounts, and compressing/decompressing SOL.
///
/// # Builder Methods
///
/// ## Common Methods
///
/// - [`with_new_addresses()`](Self::with_new_addresses) - Create new compressed account addresses
/// - [`compress_lamports()`](Self::compress_lamports) - Compress SOL into compressed accounts
/// - [`decompress_lamports()`](Self::decompress_lamports) - Decompress SOL from compressed accounts
///
/// **Note**: An instruction can either compress **or** decompress lamports, not both.
///
/// ## Advanced Methods
///
/// For fine-grained control:
///
/// - [`with_input_compressed_accounts_with_merkle_context()`](Self::with_input_compressed_accounts_with_merkle_context) - Manually specify input accounts
/// - [`with_output_compressed_accounts()`](Self::with_output_compressed_accounts) - Manually specify output accounts
#[derive(Clone)]
pub struct LightSystemProgramCpi {
    cpi_signer: CpiSigner,
    instruction_data: InstructionDataInvokeCpi,
}

impl LightSystemProgramCpi {
    #[must_use = "with_new_addresses returns a new value"]
    pub fn with_new_addresses(
        mut self,
        new_address_params: &[light_compressed_account::instruction_data::data::NewAddressParamsPacked],
    ) -> Self {
        self.instruction_data = self.instruction_data.with_new_addresses(new_address_params);
        self
    }

    #[must_use = "with_input_compressed_accounts_with_merkle_context returns a new value"]
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

    #[must_use = "with_output_compressed_accounts returns a new value"]
    pub fn with_output_compressed_accounts(
        mut self,
        output_compressed_accounts: &[light_compressed_account::instruction_data::data::OutputCompressedAccountWithPackedContext],
    ) -> Self {
        self.instruction_data = self
            .instruction_data
            .with_output_compressed_accounts(output_compressed_accounts);
        self
    }

    #[must_use = "compress_lamports returns a new value"]
    pub fn compress_lamports(mut self, lamports: u64) -> Self {
        self.instruction_data = self.instruction_data.compress_lamports(lamports);
        self
    }

    #[must_use = "decompress_lamports returns a new value"]
    pub fn decompress_lamports(mut self, lamports: u64) -> Self {
        self.instruction_data = self.instruction_data.decompress_lamports(lamports);
        self
    }
    #[must_use = "write_to_cpi_context_set returns a new value"]
    pub fn write_to_cpi_context_set(mut self) -> Self {
        self.instruction_data = self.instruction_data.write_to_cpi_context_set();
        self
    }
    #[must_use = "write_to_cpi_context_first returns a new value"]
    pub fn write_to_cpi_context_first(mut self) -> Self {
        self.instruction_data = self.instruction_data.write_to_cpi_context_first();
        self
    }
    #[must_use = "with_cpi_context returns a new value"]
    pub fn with_cpi_context(
        mut self,
        cpi_context: light_compressed_account::instruction_data::cpi_context::CompressedCpiContext,
    ) -> Self {
        self.instruction_data = self.instruction_data.with_cpi_context(cpi_context);
        self
    }

    /// Returns a reference to the inner instruction data.
    pub fn instruction_data(&self) -> &InstructionDataInvokeCpi {
        &self.instruction_data
    }

    /// Returns a mutable reference to the inner instruction data.
    pub fn instruction_data_mut(&mut self) -> &mut InstructionDataInvokeCpi {
        &mut self.instruction_data
    }

    /// Returns the CPI signer.
    pub fn cpi_signer(&self) -> &CpiSigner {
        &self.cpi_signer
    }
}

impl LightCpiInstruction for LightSystemProgramCpi {
    fn new_cpi(cpi_signer: CpiSigner, proof: ValidityProof) -> Self {
        Self {
            cpi_signer,
            instruction_data: InstructionDataInvokeCpi::new(proof.into()),
        }
    }

    fn get_mode(&self) -> u8 {
        0 // V1 uses regular mode by default
    }

    fn get_bump(&self) -> u8 {
        self.cpi_signer.bump
    }
    fn write_to_cpi_context_first(mut self) -> Self {
        self.instruction_data = self.instruction_data.write_to_cpi_context_first();
        self
    }
    fn write_to_cpi_context_set(mut self) -> Self {
        self.instruction_data = self.instruction_data.write_to_cpi_context_set();
        self
    }
    fn execute_with_cpi_context(self) -> Self {
        // V1 doesn't have a direct execute context, just return self
        // The execute happens through the invoke call
        self
    }
    fn get_with_cpi_context(&self) -> bool {
        self.instruction_data.cpi_context.is_some()
    }
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
    fn has_read_only_accounts(&self) -> bool {
        // V1 doesn't support read-only accounts
        false
    }
}

// Manual BorshSerialize implementation that only serializes instruction_data
impl AnchorSerialize for LightSystemProgramCpi {
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
