use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use solana_program::pubkey::Pubkey;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[repr(u8)]
pub enum AccountState {
    Initialized,
    Frozen,
}

#[derive(Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, Clone)]
pub struct TokenData {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// Placeholder for TokenExtension tlv data (unimplemented)
    pub tlv: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct TokenDataWithMerkleContext {
    pub token_data: TokenData,
    pub compressed_account: CompressedAccountWithMerkleContext,
}
