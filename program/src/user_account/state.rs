use crate::config::USER_ACCOUNT_TYPE;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

pub const SIZE_UTXO: usize = 64; // todo: rename to skp+ekp
pub const UTXO_CAPACITY: usize = 1; // amount of utxos that can be stored in the user account at once

#[derive(Debug, Clone)]
pub struct UserAccount {
    is_initialized: bool,
    pub account_type: u8,
    pub owner_pubkey: Pubkey,
    pub enc_utxos: Vec<u8>,
    pub modified_ranges: Vec<usize>,
    pub mode_init: bool,
}

impl Sealed for UserAccount {}

impl IsInitialized for UserAccount {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[allow(clippy::ptr_offset_with_cast)]
impl Pack for UserAccount {
    const LEN: usize = 34 + SIZE_UTXO * UTXO_CAPACITY;
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, UserAccount::LEN];

        let (is_initialized, _account_type, owner_pubkey, enc_utxos) =
            array_refs![input, 1, 1, 32, SIZE_UTXO * UTXO_CAPACITY];

        if is_initialized[0] == 0 {
            Ok(UserAccount {
                is_initialized: true,
                account_type: USER_ACCOUNT_TYPE,
                owner_pubkey: solana_program::pubkey::Pubkey::new(owner_pubkey),
                modified_ranges: Vec::new(),
                enc_utxos: enc_utxos.to_vec(),
                mode_init: true,
            })
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, UserAccount::LEN];
        let (dst_is_initialized, dst_account_type, dst_owner_pubkey, dst_enc_utxos) =
            mut_array_refs![dst, 1, 1, 32, SIZE_UTXO * UTXO_CAPACITY];

        if self.mode_init {
            dst_is_initialized[0] = 1;
            dst_account_type[0] = 10;
            for (i, x) in dst_enc_utxos.iter_mut().enumerate() {
                *x = self.enc_utxos[i]
            }
            *dst_owner_pubkey = self.owner_pubkey.to_bytes();
        }
    }
}
