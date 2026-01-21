mod account_rent;
mod config;

pub use account_rent::*;
pub use config::*;

use crate::error::CompressibleError;

#[track_caller]
pub fn get_rent_exemption_lamports(_num_bytes: u64) -> Result<u64, CompressibleError> {
    #[cfg(all(target_os = "solana", feature = "pinocchio"))]
    {
        use pinocchio::sysvars::Sysvar;
        return pinocchio::sysvars::rent::Rent::get()
            .map(|rent| rent.minimum_balance(_num_bytes as usize))
            .map_err(|_| CompressibleError::FailedBorrowRentSysvar);
    }
    #[cfg(all(target_os = "solana", not(feature = "pinocchio"), feature = "solana"))]
    {
        use solana_sysvar::Sysvar;
        return solana_sysvar::rent::Rent::get()
            .map(|rent| rent.minimum_balance(_num_bytes as usize))
            .map_err(|_| CompressibleError::FailedBorrowRentSysvar);
    }
    #[cfg(not(all(target_os = "solana", any(feature = "pinocchio", feature = "solana"))))]
    {
        Ok(solana_rent::Rent::default().minimum_balance(_num_bytes as usize))
    }
}
