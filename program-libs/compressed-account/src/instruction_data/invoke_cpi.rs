use light_zero_copy::ZeroCopyMut;

use super::{
    cpi_context::CompressedCpiContext,
    data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
};
use crate::{
    compressed_account::PackedCompressedAccountWithMerkleContext,
    discriminators::DISCRIMINATOR_INVOKE_CPI,
    instruction_data::{compressed_proof::CompressedProof, traits::LightInstructionData},
    InstructionDiscriminator, Vec,
};

#[repr(C)]
#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone, ZeroCopyMut)]
pub struct InstructionDataInvokeCpi {
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CompressedCpiContext>,
}

impl LightInstructionData for InstructionDataInvokeCpi {
    #[cfg(feature = "alloc")]
    fn data(&self) -> Result<Vec<u8>, crate::CompressedAccountError> {
        #[cfg(feature = "anchor")]
        use anchor_lang::AnchorSerialize as BorshSerialize;
        #[cfg(not(feature = "anchor"))]
        use borsh::BorshSerialize;

        let inputs = self
            .try_to_vec()
            .map_err(|_| crate::CompressedAccountError::InvalidArgument)?;
        let mut data = Vec::with_capacity(12 + inputs.len());
        data.extend_from_slice(self.discriminator());
        data.extend_from_slice(&(inputs.len() as u32).to_le_bytes());
        data.extend_from_slice(inputs.as_slice());
        Ok(data)
    }
}

impl InstructionDataInvokeCpi {
    pub fn new(proof: Option<CompressedProof>) -> Self {
        Self {
            proof,
            ..Default::default()
        }
    }

    #[must_use = "with_new_addresses returns a new value"]
    pub fn with_new_addresses(mut self, new_address_params: &[NewAddressParamsPacked]) -> Self {
        if !new_address_params.is_empty() {
            self.new_address_params
                .extend_from_slice(new_address_params);
        }
        self
    }

    #[must_use = "with_input_compressed_accounts_with_merkle_context returns a new value"]
    pub fn with_input_compressed_accounts_with_merkle_context(
        mut self,
        input_compressed_accounts_with_merkle_context: &[PackedCompressedAccountWithMerkleContext],
    ) -> Self {
        if !input_compressed_accounts_with_merkle_context.is_empty() {
            self.input_compressed_accounts_with_merkle_context
                .extend_from_slice(input_compressed_accounts_with_merkle_context);
        }
        self
    }

    #[must_use = "with_output_compressed_accounts returns a new value"]
    pub fn with_output_compressed_accounts(
        mut self,
        output_compressed_accounts: &[OutputCompressedAccountWithPackedContext],
    ) -> Self {
        if !output_compressed_accounts.is_empty() {
            self.output_compressed_accounts
                .extend_from_slice(output_compressed_accounts);
        }
        self
    }

    #[must_use = "compress_lamports returns a new value"]
    pub fn compress_lamports(mut self, lamports: u64) -> Self {
        self.compress_or_decompress_lamports = Some(lamports);
        self.is_compress = true;
        self
    }

    #[must_use = "decompress_lamports returns a new value"]
    pub fn decompress_lamports(mut self, lamports: u64) -> Self {
        self.compress_or_decompress_lamports = Some(lamports);
        self.is_compress = false;
        self
    }

    #[must_use = "write_to_cpi_context_set returns a new value"]
    pub fn write_to_cpi_context_set(mut self) -> Self {
        self.cpi_context = Some(CompressedCpiContext::set());
        self
    }

    #[must_use = "write_to_cpi_context_first returns a new value"]
    pub fn write_to_cpi_context_first(mut self) -> Self {
        self.cpi_context = Some(CompressedCpiContext::first());
        self
    }

    #[must_use = "with_cpi_context returns a new value"]
    pub fn with_cpi_context(mut self, cpi_context: CompressedCpiContext) -> Self {
        self.cpi_context = Some(cpi_context);
        self
    }
}

impl InstructionDiscriminator for InstructionDataInvokeCpi {
    fn discriminator(&self) -> &'static [u8] {
        &DISCRIMINATOR_INVOKE_CPI
    }
}
