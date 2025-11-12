use light_ctoken_types::state::CToken;
use solana_sdk::pubkey::Pubkey;

/// State of a compressible CToken account
#[derive(Clone, Debug)]
pub struct CompressibleAccountState {
    /// Account public key
    pub pubkey: Pubkey,
    pub account: CToken,
    pub lamports: u64,
}
