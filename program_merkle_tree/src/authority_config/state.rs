use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::convert::TryInto;
// Account struct to determine state of the computation
// and perform basic security checks.
#[derive(Debug, Clone)]
pub struct AuthorityConfig {
    is_initialized: bool,
    pub authority_key: Pubkey,
}

impl Sealed for AuthorityConfig {}

impl IsInitialized for AuthorityConfig {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl AuthorityConfig {
    pub fn new (authority_key: Pubkey) -> Result<AuthorityConfig, ProgramError> {
        Ok(AuthorityConfig {
            is_initialized: true,
            authority_key: authority_key,
        })
    }
}
impl Pack for AuthorityConfig {
    const LEN: usize = 33;
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, AuthorityConfig::LEN];

        let (
            is_initialized,
            authority_key,
        ) = array_refs![input, 1, 32];
        msg!("is_initialized[0], {}", is_initialized[0]);
        if is_initialized[0] == 0 {
            Err(ProgramError::UninitializedAccount)
        } else {
            Ok(AuthorityConfig {
                is_initialized: true,
                authority_key: Pubkey::new(authority_key),
            })
        }
    }

    fn pack_into_slice(&self, _dst: &mut [u8]) {
        let dst = array_mut_ref![_dst, 0, 33];
        let (
            is_initialized_dst,
            authority_key_dst,
        ) = mut_array_refs![dst, 1, 32];
        is_initialized_dst[0] = self.is_initialized as u8;
        authority_key_dst.copy_from_slice(self.authority_key.as_ref());
    }
}