//! LightCpi trait implementations for v2 instruction data types.

use light_compressed_account::{
    instruction_data::{
        compressed_proof::ValidityProof,
        with_account_info::InstructionDataInvokeCpiWithAccountInfo,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    CpiSigner,
};

use super::instruction::LightCpi;

impl LightCpi for InstructionDataInvokeCpiWithReadOnly {
    fn new_cpi(cpi_signer: CpiSigner, proof: ValidityProof) -> Self {
        Self {
            bump: cpi_signer.bump,
            invoking_program_id: cpi_signer.program_id.into(),
            proof: proof.into(),
            mode: 1,
            ..Default::default()
        }
    }
    fn write_to_cpi_context_first(self) -> Self {
        self.write_to_cpi_context_first()
    }
    fn write_to_cpi_context_set(self) -> Self {
        self.write_to_cpi_context_set()
    }
    fn execute_with_cpi_context(self) -> Self {
        self.execute_with_cpi_context()
    }
    fn get_mode(&self) -> u8 {
        self.mode
    }
    fn get_with_cpi_context(&self) -> bool {
        self.with_cpi_context
    }
    fn get_cpi_context(
        &self,
    ) -> &light_compressed_account::instruction_data::cpi_context::CompressedCpiContext {
        &self.cpi_context
    }
    fn get_bump(&self) -> u8 {
        self.bump
    }
    fn has_read_only_accounts(&self) -> bool {
        !self.read_only_accounts.is_empty()
    }
}

impl LightCpi for InstructionDataInvokeCpiWithAccountInfo {
    fn new_cpi(cpi_signer: CpiSigner, proof: ValidityProof) -> Self {
        Self {
            bump: cpi_signer.bump,
            invoking_program_id: cpi_signer.program_id.into(),
            proof: proof.into(),
            mode: 1,
            ..Default::default()
        }
    }
    fn write_to_cpi_context_first(self) -> Self {
        self.write_to_cpi_context_first()
    }
    fn write_to_cpi_context_set(self) -> Self {
        self.write_to_cpi_context_set()
    }
    fn execute_with_cpi_context(self) -> Self {
        self.execute_with_cpi_context()
    }
    fn get_mode(&self) -> u8 {
        self.mode
    }
    fn get_with_cpi_context(&self) -> bool {
        self.with_cpi_context
    }
    fn get_cpi_context(
        &self,
    ) -> &light_compressed_account::instruction_data::cpi_context::CompressedCpiContext {
        &self.cpi_context
    }
    fn get_bump(&self) -> u8 {
        self.bump
    }
    fn has_read_only_accounts(&self) -> bool {
        !self.read_only_accounts.is_empty()
    }
}
