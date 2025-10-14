mod account_rent;
mod config;

pub use account_rent::*;
pub use config::*;

use crate::error::CompressibleError;

#[track_caller]
pub fn get_rent_exemption_lamports(_num_bytes: u64) -> Result<u64, CompressibleError> {
    #[cfg(target_os = "solana")]
    {
        use pinocchio::sysvars::Sysvar;
        return pinocchio::sysvars::rent::Rent::get()
            .map(|rent| rent.minimum_balance(_num_bytes as usize))
            .map_err(|_| CompressibleError::FailedBorrowRentSysvar);
    }
    #[cfg(not(target_os = "solana"))]
    {
        unimplemented!(
            "get_rent_exemption_lamports is only implemented for target os solana and tests"
        )
    }
}
