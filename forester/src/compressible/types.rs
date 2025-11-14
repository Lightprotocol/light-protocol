use light_ctoken_types::state::CToken;
use solana_sdk::pubkey::Pubkey;

/// State of a compressible CToken account
#[derive(Clone, Debug)]
pub struct CompressibleAccountState {
    /// Account public key
    pub pubkey: Pubkey,
    pub account: CToken,
    pub lamports: u64,
    /// The slot at which this account becomes compressible (last_funded_epoch * SLOTS_PER_EPOCH)
    /// Accounts are ready to compress when current_slot > compressible_slot
    pub compressible_slot: u64,
}
