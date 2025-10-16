use std::mem::MaybeUninit;

use light_zero_copy::{ZeroCopy, ZeroCopyMut};
use pinocchio::pubkey::Pubkey;
use solana_pubkey::MAX_SEEDS;
use tinyvec::ArrayVec;

use crate::{AnchorDeserialize, AnchorSerialize, CTokenError};

#[derive(
    Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
pub struct CompressibleExtensionInstructionData {
    /// Version of the compressed token account when ctoken account is
    /// compressed and closed. (The version specifies the hashing scheme.)
    pub token_account_version: u8,
    /// Rent payment in epochs.
    /// Paid once at initialization.
    pub rent_payment: u8,
    pub has_top_up: u8,
    pub write_top_up: u32,
    pub compress_to_account_pubkey: Option<CompressToPubkey>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
pub struct CompressToPubkey {
    pub bump: u8,
    pub program_id: [u8; 32],
    pub seeds: Vec<Vec<u8>>,
}

impl CompressToPubkey {
    pub fn check_seeds(&self, pubkey: &Pubkey) -> Result<(), CTokenError> {
        if self.seeds.len() > MAX_SEEDS {
            return Err(CTokenError::TooManySeeds(MAX_SEEDS));
        }
        let mut references = ArrayVec::<[&[u8]; MAX_SEEDS]>::new();
        for seed in self.seeds.iter() {
            references.push(seed.as_slice());
        }
        let derived_pubkey = derive_address(references.as_slice(), self.bump, &self.program_id)?;
        if derived_pubkey != *pubkey {
            Err(CTokenError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

// Taken from pinocchio 0.9.2.
// Modifications:
// -  seeds: &[&[u8]; N], ->  seeds: &[&[u8]],
// - if seeds.len() > MAX_SEEDS CTokenError::InvalidAccountData
pub fn derive_address(
    seeds: &[&[u8]],
    bump: u8,
    program_id: &Pubkey,
) -> Result<Pubkey, CTokenError> {
    const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";
    if seeds.len() > MAX_SEEDS {
        return Err(CTokenError::TooManySeeds(MAX_SEEDS));
    }
    const UNINIT: MaybeUninit<&[u8]> = MaybeUninit::<&[u8]>::uninit();
    let mut data = [UNINIT; MAX_SEEDS + 2];
    let mut i = 0;

    while i < seeds.len() {
        // SAFETY: `data` is guaranteed to have enough space for `N` seeds,
        // so `i` will always be within bounds.
        unsafe {
            data.get_unchecked_mut(i).write(seeds.get_unchecked(i));
        }
        i += 1;
    }

    // TODO: replace this with `as_slice` when the MSRV is upgraded
    // to `1.84.0+`.
    let bump_seed = [bump];

    // SAFETY: `data` is guaranteed to have enough space for `MAX_SEEDS + 2`
    // elements, and `MAX_SEEDS` is as large as `N`.
    unsafe {
        data.get_unchecked_mut(i).write(&bump_seed);
        i += 1;

        data.get_unchecked_mut(i).write(program_id.as_ref());
        data.get_unchecked_mut(i + 1).write(PDA_MARKER.as_ref());
    }

    #[cfg(target_os = "solana")]
    {
        use pinocchio::syscalls::sol_sha256;
        let mut pda = MaybeUninit::<[u8; 32]>::uninit();

        // SAFETY: `data` has `i + 2` elements initialized.
        unsafe {
            sol_sha256(
                data.as_ptr() as *const u8,
                (i + 2) as u64,
                pda.as_mut_ptr() as *mut u8,
            );
        }

        // SAFETY: `pda` has been initialized by the syscall.
        unsafe { Ok(pda.assume_init()) }
    }

    #[cfg(not(target_os = "solana"))]
    unreachable!("deriving a pda is only available on target `solana`");
}
