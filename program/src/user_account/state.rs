use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    //msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

pub const SIZE_UTXO: u8 = 64;

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

impl Pack for UserAccount {
    const LEN: usize = 34 + SIZE_UTXO as usize * 100;
    fn unpack_from_slice(input:  &[u8]) ->  Result<Self, ProgramError>{
        let input = array_ref![input, 0, UserAccount::LEN];

        let (
            is_initialized,
            account_type,
            owner_pubkey,
            enc_utxos
        ) = array_refs![input,1, 1, 32, SIZE_UTXO as usize  * 100];

        if is_initialized[0] == 0 {
            Ok(UserAccount {
                is_initialized: true,
                account_type: account_type[0],
                owner_pubkey: solana_program::pubkey::Pubkey::new(owner_pubkey),
                modified_ranges: Vec::new(),
                enc_utxos: enc_utxos.to_vec(),
                mode_init: true
            })
        } else {
            Ok(
                UserAccount {
                    is_initialized: true,
                    account_type: account_type[0],
                    owner_pubkey: solana_program::pubkey::Pubkey::new(owner_pubkey),
                    enc_utxos: enc_utxos.to_vec(),
                    modified_ranges: Vec::new(),
                    mode_init: false
                }
            )
        }

    }

    fn pack_into_slice(&self, dst: &mut [u8]) {

        let dst = array_mut_ref![dst, 0,  UserAccount::LEN];
        let (
            dst_is_initialized,
            dst_account_type,
            dst_owner_pubkey,
            dst_enc_utxos
        ) = mut_array_refs![dst,1, 1, 32, SIZE_UTXO as usize * 100];
        if self.mode_init {
            dst_is_initialized[0] = 1;
            dst_account_type[0] = 10;
            *dst_owner_pubkey = self.owner_pubkey.to_bytes().clone();
        } else {
            for modifying_index in self.modified_ranges.iter() {
                for (i, x) in dst_enc_utxos[modifying_index*SIZE_UTXO as usize..modifying_index*SIZE_UTXO as usize + SIZE_UTXO as usize].iter_mut().enumerate() {
                    *x= self.enc_utxos[i];
                }
            }

        }
    }
}
