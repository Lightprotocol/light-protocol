use super::account_info_trait::AccountInfoTrait;
use crate::error::AccountError;

/// Implement trait for solana AccountInfo
impl AccountInfoTrait for solana_account_info::AccountInfo<'_> {
    type Pubkey = solana_pubkey::Pubkey;
    type DataRef<'b>
        = core::cell::Ref<'b, [u8]>
    where
        Self: 'b;
    type DataRefMut<'b>
        = core::cell::RefMut<'b, [u8]>
    where
        Self: 'b;

    fn key(&self) -> [u8; 32] {
        self.key.to_bytes()
    }

    fn pubkey(&self) -> Self::Pubkey {
        *self.key
    }

    fn pubkey_from_bytes(bytes: [u8; 32]) -> Self::Pubkey {
        solana_pubkey::Pubkey::from(bytes)
    }

    fn is_writable(&self) -> bool {
        self.is_writable
    }

    fn is_signer(&self) -> bool {
        self.is_signer
    }

    fn executable(&self) -> bool {
        self.executable
    }

    fn lamports(&self) -> u64 {
        **self.lamports.borrow()
    }

    fn data_len(&self) -> usize {
        self.data.borrow().len()
    }

    fn try_borrow_data(&self) -> Result<Self::DataRef<'_>, AccountError> {
        self.data
            .try_borrow()
            .map(|r| core::cell::Ref::map(r, |data| &**data))
            .map_err(Into::into)
    }

    fn try_borrow_mut_data(&self) -> Result<Self::DataRefMut<'_>, AccountError> {
        self.data
            .try_borrow_mut()
            .map(|r| core::cell::RefMut::map(r, |data| &mut **data))
            .map_err(Into::into)
    }

    fn is_owned_by(&self, program: &[u8; 32]) -> bool {
        self.owner.as_ref() == program
    }

    fn find_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> ([u8; 32], u8) {
        let program_pubkey = solana_pubkey::Pubkey::from(*program_id);
        let (pubkey, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &program_pubkey);
        (pubkey.to_bytes(), bump)
    }

    fn create_program_address(
        seeds: &[&[u8]],
        program_id: &[u8; 32],
    ) -> Result<[u8; 32], AccountError> {
        let program_pubkey = solana_pubkey::Pubkey::from(*program_id);
        solana_pubkey::Pubkey::create_program_address(seeds, &program_pubkey)
            .map(|pubkey| pubkey.to_bytes())
            .map_err(|_| AccountError::InvalidSeeds)
    }

    fn get_min_rent_balance(size: usize) -> Result<u64, AccountError> {
        use solana_sysvar::Sysvar;
        solana_sysvar::rent::Rent::get()
            .map(|rent| rent.minimum_balance(size))
            .map_err(|_| AccountError::FailedBorrowRentSysvar)
    }
}
