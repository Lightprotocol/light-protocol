use solana_sdk::pubkey::Pubkey;

/// State of a compressible CToken account
#[derive(Clone, Debug)]
pub struct CompressibleAccountState {
    /// Account public key
    pub pubkey: Pubkey,
    /// Mint public key
    pub mint: Pubkey,
    /// Owner public key
    pub owner: Pubkey,
    /// Token balance
    pub balance: u64,
    /// Last slot when rent was claimed
    pub last_claimed_slot: u64,
    /// Compression authority (registry PDA)
    pub compression_authority: Pubkey,
    /// Rent sponsor (receives claimed rent)
    pub rent_sponsor: Pubkey,
    /// Whether to compress to account pubkey (for PDA accounts)
    pub compress_to_pubkey: bool,
    /// Last slot when account was seen/updated
    pub last_seen_slot: u64,
    /// Whether the account is currently compressible
    pub is_compressible: bool,
}
