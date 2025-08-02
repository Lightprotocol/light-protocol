use light_compressed_account::{
    instruction_data::{
        compressed_proof::CompressedProof, zero_copy_set::CompressedCpiContextTrait,
    },
    Pubkey,
};
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{
    instructions::create_compressed_mint::CompressedMintWithContext, state::CompressedMint,
    AnchorDeserialize, AnchorSerialize,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CompressedMintInputs {
    pub leaf_index: u32,
    pub prove_by_index: bool,
    pub root_index: u16,
    pub address: [u8; 32],
    pub compressed_mint_input: CompressedMint, //TODO: move supply and authority last so that we can send only the hash chain.
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct Recipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct MintToCompressedInstructionData {
    pub token_account_version: u8,
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub proof: Option<CompressedProof>,
    pub lamports: Option<u64>,
    pub recipients: Vec<Recipient>,
    pub cpi_context: Option<CpiContext>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct CpiContext {
    pub set_context: bool,
    pub first_set_context: bool,
    pub in_tree_index: u8,
    pub in_queue_index: u8,
    pub out_queue_index: u8,
    pub token_out_queue_index: u8,
}
impl CompressedCpiContextTrait for ZCpiContext<'_> {
    fn first_set_context(&self) -> u8 {
        self.first_set_context() as u8
    }

    fn set_context(&self) -> u8 {
        self.set_context() as u8
    }
}
