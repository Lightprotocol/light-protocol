use light_token_interface::state::Token;
use solana_sdk::pubkey::Pubkey;

use crate::compressible::traits::CompressibleState;

#[derive(Clone, Debug)]
pub struct CTokenAccountState {
    pub pubkey: Pubkey,
    pub account: Token,
    pub lamports: u64,
    /// Ready to compress when current_slot > compressible_slot
    pub compressible_slot: u64,
}

impl CompressibleState for CTokenAccountState {
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
