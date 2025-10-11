use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::error::AccountError;

/// Trait to abstract over different AccountInfo implementations (pinocchio vs solana)
pub trait AccountInfoTrait {
    type Pubkey: Copy + Clone + Debug + PartialEq;
    type DataRef<'a>: Deref<Target = [u8]>
    where
        Self: 'a;
    type DataRefMut<'a>: DerefMut<Target = [u8]>
    where
        Self: 'a;

    /// Return raw byte array for maximum compatibility
    fn key(&self) -> [u8; 32];
    /// Return the pubkey in the native format
    fn pubkey(&self) -> Self::Pubkey;
    fn is_writable(&self) -> bool;
    fn is_signer(&self) -> bool;
    fn executable(&self) -> bool;
    fn lamports(&self) -> u64;
    fn data_len(&self) -> usize;

    /// Unified data access interface
    fn try_borrow_data(&self) -> Result<Self::DataRef<'_>, AccountError>;
    fn try_borrow_mut_data(&self) -> Result<Self::DataRefMut<'_>, AccountError>;

    /// Check ownership safely - each implementation handles this without exposing owner
    fn is_owned_by(&self, program: &[u8; 32]) -> bool;

    /// Convert byte array to native Pubkey type
    fn pubkey_from_bytes(bytes: [u8; 32]) -> Self::Pubkey;

    /// PDA functions - each implementation uses its own backend
    fn find_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> ([u8; 32], u8);
    fn create_program_address(
        seeds: &[&[u8]],
        program_id: &[u8; 32],
    ) -> Result<[u8; 32], AccountError>;

    /// Get minimum rent balance for a given size
    fn get_min_rent_balance(size: usize) -> Result<u64, AccountError>;

    fn data_is_empty(&self) -> bool {
        self.data_len() == 0
    }
}
