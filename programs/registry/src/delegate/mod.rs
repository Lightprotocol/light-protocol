pub mod delegate_instruction;
pub mod deposit;
pub mod deposit_instruction;
pub mod process_cpi;
pub mod process_delegate;
pub mod state;
// TODO: move into cpi dir
pub mod traits;
use anchor_lang::solana_program::pubkey::Pubkey;

pub const ESCROW_TOKEN_ACCOUNT_SEED: &[u8] = b"ESCROW_TOKEN_ACCOUNT_SEED";
pub const DELEGATE_ACCOUNT_DISCRIMINATOR: [u8; 8] = [1, 0, 0, 0, 0, 0, 0, 0];
pub const FORESTER_EPOCH_RESULT_ACCOUNT_DISCRIMINATOR: [u8; 8] = [2, 0, 0, 0, 0, 0, 0, 0];

pub fn get_escrow_token_authority(authority: &Pubkey, salt: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            ESCROW_TOKEN_ACCOUNT_SEED,
            authority.as_ref(),
            salt.to_le_bytes().as_slice(),
        ],
        &crate::ID,
    )
}
