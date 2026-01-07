use light_token_interface::state::Mint;
use solana_sdk::pubkey::Pubkey;

use crate::compressible::traits::CompressibleState;

#[derive(Clone, Debug)]
pub struct MintAccountState {
    pub pubkey: Pubkey,
    pub mint_seed: Pubkey,
    pub compressed_address: [u8; 32],
    pub mint: Mint,
    pub lamports: u64,
    /// Ready to compress when current_slot > compressible_slot
    pub compressible_slot: u64,
}

impl CompressibleState for MintAccountState {
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
