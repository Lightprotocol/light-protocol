use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_zero_copy::ZeroCopy;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct CreateTokenAccountInstructionData {
    /// The owner of the token account
    pub owner: Pubkey,
}