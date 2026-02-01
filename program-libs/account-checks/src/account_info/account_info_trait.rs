use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::error::AccountError;

/// Lightweight owned account metadata for CPI instruction building.
///
/// Replaces solana_instruction::AccountMeta / pinocchio::instruction::AccountMeta
/// as a framework-agnostic type that can be stored in collections without lifetime issues.
#[derive(Clone, Debug)]
pub struct CpiMeta {
    pub pubkey: [u8; 32],
    pub is_signer: bool,
    pub is_writable: bool,
}

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

    /// Get the current clock slot from sysvar.
    /// Only meaningful on-chain; implementations may error off-chain.
    fn get_current_slot() -> Result<u64, AccountError>;

    /// Assign the account to a new owner program.
    fn assign(&self, new_owner: &[u8; 32]) -> Result<(), AccountError>;

    /// Resize the account data (truncating or zero-extending).
    fn realloc(&self, new_len: usize, zero_init: bool) -> Result<(), AccountError>;

    /// Subtract lamports from the account (checked).
    fn sub_lamports(&self, amount: u64) -> Result<(), AccountError>;

    /// Add lamports to the account (checked).
    fn add_lamports(&self, amount: u64) -> Result<(), AccountError>;

    fn data_is_empty(&self) -> bool {
        self.data_len() == 0
    }

    /// Close this account: zero data, transfer all lamports to destination,
    /// assign to system program.
    fn close(&self, destination: &Self) -> Result<(), AccountError>;

    /// Create a PDA account via system program CPI (invoke_signed).
    ///
    /// `self` is the uninitialized PDA account to be created.
    /// Handles the edge case where the account already has lamports
    /// (e.g. attacker donation) by falling back to Assign + Allocate + Transfer.
    ///
    /// # Arguments
    /// * `lamports` - Amount of lamports for rent-exemption
    /// * `space` - Size of the account data in bytes
    /// * `owner` - Program that will own the created account
    /// * `pda_seeds` - Seeds for this PDA (including bump) for signing
    /// * `rent_payer` - Account paying for rent
    /// * `rent_payer_seeds` - Seeds for the rent payer PDA for signing
    /// * `system_program` - The system program account
    #[allow(clippy::too_many_arguments)]
    fn create_pda_account(
        &self,
        lamports: u64,
        space: u64,
        owner: &[u8; 32],
        pda_seeds: &[&[u8]],
        rent_payer: &Self,
        rent_payer_seeds: &[&[u8]],
        system_program: &Self,
    ) -> Result<(), AccountError>;

    /// Transfer lamports by direct lamport manipulation (no CPI).
    fn transfer_lamports(&self, destination: &Self, lamports: u64) -> Result<(), AccountError> {
        self.sub_lamports(lamports)?;
        destination.add_lamports(lamports)
    }

    /// Transfer lamports via system program CPI with invoke_signed.
    /// Pass `&[]` for `signer_seeds` if the sender is already a signer.
    fn transfer_lamports_cpi(
        &self,
        destination: &Self,
        lamports: u64,
        signer_seeds: &[&[u8]],
    ) -> Result<(), AccountError>;

    /// Invoke an arbitrary program via CPI with optional PDA signing.
    ///
    /// This is the generic CPI entry point. It builds a native instruction
    /// from the decomposed components and calls the runtime's invoke_signed.
    ///
    /// # Arguments
    /// * `program_id` - Target program to invoke
    /// * `instruction_data` - Serialized instruction data
    /// * `account_metas` - Account metadata describing each account's role
    /// * `account_infos` - The actual account info objects (must match metas)
    /// * `signer_seeds` - PDA signer seeds; pass `&[]` for no PDA signing
    fn invoke_cpi(
        program_id: &[u8; 32],
        instruction_data: &[u8],
        account_metas: &[CpiMeta],
        account_infos: &[Self],
        signer_seeds: &[&[&[u8]]],
    ) -> Result<(), AccountError>
    where
        Self: Sized;
}
