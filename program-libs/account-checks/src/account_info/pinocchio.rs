use super::account_info_trait::AccountInfoTrait;
use crate::error::AccountError;

/// Implement trait for pinocchio AccountInfo
impl AccountInfoTrait for pinocchio::account_info::AccountInfo {
    type Pubkey = [u8; 32];
    type DataRef<'a> = pinocchio::account_info::Ref<'a, [u8]>;
    type DataRefMut<'a> = pinocchio::account_info::RefMut<'a, [u8]>;

    fn key(&self) -> [u8; 32] {
        *self.key()
    }

    fn pubkey(&self) -> Self::Pubkey {
        *self.key()
    }

    fn pubkey_from_bytes(bytes: [u8; 32]) -> Self::Pubkey {
        bytes
    }

    #[inline(always)]
    fn is_writable(&self) -> bool {
        self.is_writable()
    }

    #[inline(always)]
    fn is_signer(&self) -> bool {
        self.is_signer()
    }

    #[inline(always)]
    fn executable(&self) -> bool {
        self.executable()
    }

    fn lamports(&self) -> u64 {
        self.lamports()
    }

    fn data_len(&self) -> usize {
        self.data_len()
    }

    fn try_borrow_data(&self) -> Result<Self::DataRef<'_>, AccountError> {
        self.try_borrow_data().map_err(Into::into)
    }

    fn try_borrow_mut_data(&self) -> Result<Self::DataRefMut<'_>, AccountError> {
        self.try_borrow_mut_data().map_err(Into::into)
    }

    fn is_owned_by(&self, program: &[u8; 32]) -> bool {
        pinocchio::account_info::AccountInfo::is_owned_by(self, program)
    }

    fn find_program_address(_seeds: &[&[u8]], _program_id: &[u8; 32]) -> ([u8; 32], u8) {
        #[cfg(target_os = "solana")]
        {
            let program_pubkey = pinocchio::pubkey::Pubkey::from(*_program_id);
            let (pubkey, bump) = pinocchio::pubkey::find_program_address(_seeds, &program_pubkey);
            (pubkey, bump)
        }
        // Pinocchio does not support find_program_address outside of target_os solana.
        #[cfg(all(not(target_os = "solana"), feature = "solana"))]
        {
            let program_pubkey = solana_pubkey::Pubkey::from(*_program_id);
            let (pubkey, bump) =
                solana_pubkey::Pubkey::find_program_address(_seeds, &program_pubkey);
            (pubkey.to_bytes(), bump)
        }
        #[cfg(all(not(target_os = "solana"), not(feature = "solana")))]
        {
            panic!("find_program_address not supported with pinocchio outside target_os = solana without solana feature");
        }
    }

    fn create_program_address(
        _seeds: &[&[u8]],
        _program_id: &[u8; 32],
    ) -> Result<[u8; 32], AccountError> {
        #[cfg(target_os = "solana")]
        {
            let program_pubkey = pinocchio::pubkey::Pubkey::from(*_program_id);
            pinocchio::pubkey::create_program_address(_seeds, &program_pubkey)
                .map_err(|_| AccountError::InvalidSeeds)
        }
        // Pinocchio does not support create_program_address outside of target_os solana.
        #[cfg(all(not(target_os = "solana"), feature = "solana"))]
        {
            let program_pubkey = solana_pubkey::Pubkey::from(*_program_id);
            let pubkey = solana_pubkey::Pubkey::create_program_address(_seeds, &program_pubkey)
                .map_err(|_| AccountError::InvalidSeeds)?;
            Ok(pubkey.to_bytes())
        }
        #[cfg(all(not(target_os = "solana"), not(feature = "solana")))]
        {
            Err(AccountError::InvalidSeeds)
        }
    }

    fn get_min_rent_balance(_size: usize) -> Result<u64, AccountError> {
        #[cfg(target_os = "solana")]
        {
            use pinocchio::sysvars::Sysvar;
            pinocchio::sysvars::rent::Rent::get()
                .map(|rent| rent.minimum_balance(_size))
                .map_err(|_| AccountError::FailedBorrowRentSysvar)
        }
        #[cfg(all(not(target_os = "solana"), feature = "solana"))]
        {
            use solana_sysvar::Sysvar;

            solana_sysvar::rent::Rent::get()
                .map(|rent| rent.minimum_balance(_size))
                .map_err(|_| AccountError::FailedBorrowRentSysvar)
        }
        #[cfg(all(not(target_os = "solana"), not(feature = "solana")))]
        {
            Err(AccountError::FailedBorrowRentSysvar)
        }
    }
}
