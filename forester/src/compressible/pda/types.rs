use solana_sdk::pubkey::Pubkey;

use crate::compressible::traits::CompressibleState;

/// Stores only metadata. The compressor fetches the compressed account
/// from the indexer to get validity proofs before closing the on-chain PDA.
#[derive(Clone, Debug)]
pub struct PdaAccountState {
    pub pubkey: Pubkey,
    pub program_id: Pubkey,
    pub lamports: u64,
    /// Ready to compress when current_slot > compressible_slot
    pub compressible_slot: u64,
}

impl CompressibleState for PdaAccountState {
    fn pubkey(&self) -> &Pubkey {
        &self.pubkey
    }

    fn lamports(&self) -> u64 {
        self.lamports
    }

    fn compressible_slot(&self) -> u64 {
        self.compressible_slot
    }
}
