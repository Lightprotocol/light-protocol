use super::account_info_trait::AccountInfoTrait;
use super::account_meta_trait::AccountMetaTrait;
use crate::error::AccountError;

/// Owned account meta for pinocchio.
///
/// Pinocchio's native `AccountMeta<'a>` borrows the pubkey, so we need an
/// owned wrapper that can be stored in collections.
#[derive(Clone, Debug)]
pub struct OwnedAccountMeta {
    pub pubkey: [u8; 32],
    pub is_signer: bool,
    pub is_writable: bool,
}

impl AccountMetaTrait for OwnedAccountMeta {
    type Pubkey = [u8; 32];

    fn new(pubkey: [u8; 32], is_signer: bool, is_writable: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable,
        }
    }

    fn pubkey_to_bytes(pubkey: [u8; 32]) -> [u8; 32] {
        pubkey
    }

    fn pubkey_from_bytes(bytes: [u8; 32]) -> [u8; 32] {
        bytes
    }

    fn pubkey_bytes(&self) -> [u8; 32] {
        self.pubkey
    }

    fn is_signer(&self) -> bool {
        self.is_signer
    }

    fn is_writable(&self) -> bool {
        self.is_writable
    }

    fn set_is_signer(&mut self, val: bool) {
        self.is_signer = val;
    }

    fn set_is_writable(&mut self, val: bool) {
        self.is_writable = val;
    }
}

/// Implement trait for pinocchio AccountInfo
impl AccountInfoTrait for pinocchio::account_info::AccountInfo {
    type Pubkey = [u8; 32];
    type DataRef<'a> = pinocchio::account_info::Ref<'a, [u8]>;
    type DataRefMut<'a> = pinocchio::account_info::RefMut<'a, [u8]>;

    fn key(&self) -> [u8; 32] {
        *self.key()
    }

    fn key_ref(&self) -> &[u8] {
        self.key()
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

    fn get_current_slot() -> Result<u64, AccountError> {
        #[cfg(target_os = "solana")]
        {
            use pinocchio::sysvars::Sysvar;
            pinocchio::sysvars::clock::Clock::get()
                .map(|c| c.slot)
                .map_err(|_| AccountError::FailedSysvarAccess)
        }
        #[cfg(all(not(target_os = "solana"), feature = "solana"))]
        {
            use solana_sysvar::Sysvar;
            solana_sysvar::clock::Clock::get()
                .map(|c| c.slot)
                .map_err(|_| AccountError::FailedSysvarAccess)
        }
        #[cfg(all(not(target_os = "solana"), not(feature = "solana")))]
        {
            Err(AccountError::FailedSysvarAccess)
        }
    }

    fn assign(&self, new_owner: &[u8; 32]) -> Result<(), AccountError> {
        // SAFETY: We trust the caller to provide a valid owner.
        // This is safe in the Solana runtime context where the runtime
        // validates ownership changes.
        unsafe {
            self.assign(&pinocchio::pubkey::Pubkey::from(*new_owner));
        }
        Ok(())
    }

    fn realloc(&self, new_len: usize, _zero_init: bool) -> Result<(), AccountError> {
        self.resize(new_len).map_err(|e| AccountError::from(e))
    }

    fn sub_lamports(&self, amount: u64) -> Result<(), AccountError> {
        let mut lamports = self
            .try_borrow_mut_lamports()
            .map_err(AccountError::from)?;
        *lamports = lamports
            .checked_sub(amount)
            .ok_or(AccountError::ArithmeticOverflow)?;
        Ok(())
    }

    fn add_lamports(&self, amount: u64) -> Result<(), AccountError> {
        let mut lamports = self
            .try_borrow_mut_lamports()
            .map_err(AccountError::from)?;
        *lamports = lamports
            .checked_add(amount)
            .ok_or(AccountError::ArithmeticOverflow)?;
        Ok(())
    }

    fn close(&self, destination: &Self) -> Result<(), AccountError> {
        crate::close_account::close_account(self, destination)
    }

    #[inline(never)]
    fn create_pda_account(
        &self,
        lamports: u64,
        space: u64,
        owner: &[u8; 32],
        pda_seeds: &[&[u8]],
        rent_payer: &Self,
        rent_payer_seeds: &[&[u8]],
        _system_program: &Self,
    ) -> Result<(), AccountError> {
        extern crate alloc;
        use alloc::vec::Vec;
        use pinocchio::instruction::{Seed, Signer};

        let pda_seeds_vec: Vec<Seed> = pda_seeds.iter().map(|s| Seed::from(*s)).collect();
        let pda_signer = Signer::from(&pda_seeds_vec[..]);

        // Only build payer signer when rent_payer is itself a PDA.
        // Passing empty seeds to invoke_signed causes create_program_address(&[], program_id)
        // which can fail if the result happens to land on the ed25519 curve.
        let payer_seeds_vec: Vec<Seed> =
            rent_payer_seeds.iter().map(|s| Seed::from(*s)).collect();
        let has_payer_seeds = !rent_payer_seeds.is_empty();

        // Cold path: account already has lamports (e.g., attacker donation).
        // CreateAccount would fail, so use Assign + Allocate + Transfer.
        if self.lamports() > 0 {
            pinocchio_system::instructions::Assign {
                account: self,
                owner,
            }
            .invoke_signed(&[pda_signer.clone()])
            .map_err(AccountError::from)?;

            pinocchio_system::instructions::Allocate {
                account: self,
                space,
            }
            .invoke_signed(&[pda_signer])
            .map_err(AccountError::from)?;

            let current_lamports = self.lamports();
            if lamports > current_lamports {
                if has_payer_seeds {
                    let payer_signer = Signer::from(&payer_seeds_vec[..]);
                    pinocchio_system::instructions::Transfer {
                        from: rent_payer,
                        to: self,
                        lamports: lamports - current_lamports,
                    }
                    .invoke_signed(&[payer_signer])
                    .map_err(AccountError::from)?;
                } else {
                    pinocchio_system::instructions::Transfer {
                        from: rent_payer,
                        to: self,
                        lamports: lamports - current_lamports,
                    }
                    .invoke_signed(&[])
                    .map_err(AccountError::from)?;
                }
            }

            return Ok(());
        }

        // Normal path: CreateAccount
        let create_account = pinocchio_system::instructions::CreateAccount {
            from: rent_payer,
            to: self,
            lamports,
            space,
            owner,
        };
        if has_payer_seeds {
            let payer_signer = Signer::from(&payer_seeds_vec[..]);
            create_account
                .invoke_signed(&[payer_signer, pda_signer])
                .map_err(AccountError::from)
        } else {
            create_account
                .invoke_signed(&[pda_signer])
                .map_err(AccountError::from)
        }
    }

    fn transfer_lamports_cpi(
        &self,
        destination: &Self,
        lamports: u64,
        signer_seeds: &[&[u8]],
    ) -> Result<(), AccountError> {
        extern crate alloc;
        use alloc::vec::Vec;
        use pinocchio::instruction::{Seed, Signer};

        let seeds_vec: Vec<Seed> = signer_seeds.iter().map(|s| Seed::from(*s)).collect();
        let signer = Signer::from(&seeds_vec[..]);

        pinocchio_system::instructions::Transfer {
            from: self,
            to: destination,
            lamports,
        }
        .invoke_signed(&[signer])
        .map_err(AccountError::from)
    }

    fn invoke_cpi(
        program_id: &[u8; 32],
        instruction_data: &[u8],
        account_metas: &[super::account_info_trait::CpiMeta],
        account_infos: &[Self],
        signer_seeds: &[&[&[u8]]],
    ) -> Result<(), AccountError> {
        extern crate alloc;
        use alloc::vec::Vec;
        use pinocchio::instruction::{AccountMeta, Seed, Signer};

        // Build owned pubkeys so AccountMeta can borrow them
        let pubkeys: Vec<pinocchio::pubkey::Pubkey> = account_metas
            .iter()
            .map(|m| pinocchio::pubkey::Pubkey::from(m.pubkey))
            .collect();

        // Build pinocchio AccountMetas referencing the owned pubkeys
        let metas: Vec<AccountMeta<'_>> = account_metas
            .iter()
            .zip(pubkeys.iter())
            .map(|(m, pk)| AccountMeta::new(pk, m.is_writable, m.is_signer))
            .collect();

        let program_pubkey = pinocchio::pubkey::Pubkey::from(*program_id);
        let instruction = pinocchio::instruction::Instruction {
            program_id: &program_pubkey,
            accounts: &metas,
            data: instruction_data,
        };

        let info_refs: Vec<&pinocchio::account_info::AccountInfo> =
            account_infos.iter().collect();

        // Build signers from seeds
        let signer_seed_vecs: Vec<Vec<Seed>> = signer_seeds
            .iter()
            .map(|seeds| seeds.iter().map(|s| Seed::from(*s)).collect())
            .collect();
        let signers: Vec<Signer> = signer_seed_vecs
            .iter()
            .map(|seeds| Signer::from(&seeds[..]))
            .collect();

        pinocchio::cpi::invoke_signed_with_bounds::<64>(
            &instruction,
            &info_refs,
            &signers,
        )
        .map_err(AccountError::from)
    }
}
