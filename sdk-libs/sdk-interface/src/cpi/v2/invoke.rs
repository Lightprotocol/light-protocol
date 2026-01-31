use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
use light_sdk_types::CpiSigner;
use super::lowlevel::CompressedCpiContext;
use super::lowlevel::{to_account_metas, InstructionDataInvokeCpiWithReadOnly};
use crate::cpi::{account::CpiAccountsTrait, instruction::LightCpiInstruction, v2::CpiAccounts};
use solana_account_info::AccountInfo;
use solana_instruction::AccountMeta;
use solana_program_error::ProgramError;

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
    fn get_cpi_context(&self) -> &CompressedCpiContext {
        &self.cpi_context
    }

    fn get_bump(&self) -> u8 {
        self.bump
    }
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
    fn get_cpi_context(&self) -> &CompressedCpiContext {
        &self.cpi_context
    }

    fn get_bump(&self) -> u8 {
        self.bump
    }
    fn has_read_only_accounts(&self) -> bool {
        !self.read_only_accounts.is_empty()
    }
}
