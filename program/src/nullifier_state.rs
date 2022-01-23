use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::program_pack::IsInitialized;
use solana_program::program_pack::Pack;
use solana_program::program_pack::Sealed;
use solana_program::{msg, program_error::ProgramError};

#[derive(Clone, Debug)]
pub struct NullifierState {
    pub is_initialized: bool,
    pub account_type: u8,
}

impl Sealed for NullifierState {}
impl IsInitialized for NullifierState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for NullifierState {
    const LEN: usize = 2;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, NullifierState::LEN];

        let (is_initialized, _account_type) = array_refs![input, 1, 1];

        if is_initialized[0] != 0 {
            msg!("nullifier already spent");
            panic!();
        }
        Ok(NullifierState {
            is_initialized: true,
            account_type: 3,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, NullifierState::LEN];
        let (is_initialized_dst, account_type_dst) = mut_array_refs![dst, 1, 1];
        *is_initialized_dst = [1];
        *account_type_dst = [3];
        msg!("packed inserted_nullifier");
    }
}
