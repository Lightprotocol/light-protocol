use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum AccountState {
    Initialized,
    Frozen,
}

#[derive(Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Clone)]
pub struct TokenData {
    /// The mint associated with this account
    pub mint: [u8; 32],
    /// The owner of this account.
    pub owner: [u8; 32],
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<[u8; 32]>,
    /// The account's state
    pub state: AccountState,
    /// Placeholder for TokenExtension tlv data (unimplemented)
    pub tlv: Option<Vec<u8>>,
}
