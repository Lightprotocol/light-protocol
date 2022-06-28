use anchor_lang::prelude::*;

#[account]
pub struct FeeEscrowState {
    pub verifier_state_pubkey: Pubkey,
    pub relayer_pubkey: Pubkey,
    pub user_pubkey: Pubkey,
    pub tx_fee: u64,      // fees for tx (tx_fee = number_of_tx * 0.000005)
    pub relayer_fee: u64, // for relayer
    pub creation_slot: u64,
}
