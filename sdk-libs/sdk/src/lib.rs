pub mod account;
pub mod account_info;
pub mod address;
pub mod constants;
pub use constants::*;
pub mod cpi;
pub mod error;
pub mod instruction;
pub mod legacy;
pub mod token;
pub mod transfer;
pub mod utils;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use light_account_checks::{discriminator::Discriminator as LightDiscriminator, *};
pub use light_compressed_account::{self, instruction_data::data::*};
pub use light_hasher::*;
pub use light_sdk_macros::*;
pub use light_verifier as verifier;
use light_verifier::CompressedProof;
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_instruction::{AccountMeta, Instruction};
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::{pubkey, Pubkey};

pub type AddressProof = CompressedProof;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
pub struct ValidityProof(pub Option<CompressedProof>);

impl ValidityProof {
    pub fn new(proof: Option<CompressedProof>) -> Self {
        Self(proof)
    }
}

impl From<CompressedProof> for ValidityProof {
    fn from(proof: CompressedProof) -> Self {
        Self(Some(proof))
    }
}

impl From<Option<CompressedProof>> for ValidityProof {
    fn from(proof: Option<CompressedProof>) -> Self {
        Self(proof)
    }
}
impl From<&CompressedProof> for ValidityProof {
    fn from(proof: &CompressedProof) -> Self {
        Self(Some(*proof))
    }
}

impl From<&Option<CompressedProof>> for ValidityProof {
    fn from(proof: &Option<CompressedProof>) -> Self {
        Self(*proof)
    }
}

#[allow(clippy::from_over_into)]
impl Into<Option<CompressedProof>> for ValidityProof {
    fn into(self) -> Option<CompressedProof> {
        self.0
    }
}
