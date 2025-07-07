use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_zero_copy::ZeroCopy;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct CreateAssociatedTokenAccountInstructionData {
    /// The owner of the associated token account
    pub owner: Pubkey,
    /// The mint for the associated token account
    pub mint: Pubkey,
    pub bump: u8,
}
