use std::borrow::Cow;

use light_compressible::rent::{RentConfig, RentConfigTrait};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_pubkey::Pubkey;
use solana_sysvar::Sysvar;

use crate::{instruction::PackedAccounts, AnchorDeserialize, AnchorSerialize, ProgramError};

/// Replace 32-byte Pubkeys with 1-byte indices to save space.
/// If your type has no Pubkeys, just return self.
pub trait Pack {
    type Packed: AnchorSerialize + Clone + std::fmt::Debug;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed;
}

pub trait Unpack {
    type Unpacked;

    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Self::Unpacked, crate::ProgramError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum AccountState {
    Initialized,
    Frozen,
}

pub trait HasCompressionInfo {
    fn compression_info(&self) -> &CompressionInfo;
    fn compression_info_mut(&mut self) -> &mut CompressionInfo;
    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo>;
    fn set_compression_info_none(&mut self);
}

/// Account space when compressed.
pub trait CompressedInitSpace {
    const COMPRESSED_INIT_SPACE: usize;
}

/// Override what gets stored when compressing. Return Self or a different type.
pub trait CompressAs {
    type Output: crate::AnchorSerialize
        + crate::AnchorDeserialize
        + crate::LightDiscriminator
        + crate::account::Size
        + HasCompressionInfo
        + Default
        + Clone;

    fn compress_as(&self) -> Cow<'_, Self::Output>;
}

#[derive(Debug, Clone, Default, AnchorSerialize, AnchorDeserialize)]
pub struct CompressionInfo {
    /// Version of the compressible config used to initialize this account.
    pub config_version: u16,
    /// Authority that can compress and close the PDA.
    pub compression_authority: Pubkey,
    /// Recipient for rent exemption and completed-epoch rent.
    pub rent_sponsor: Pubkey,
    /// Lamports to top up on each write (heuristic)
    pub lamports_per_write: u32,
    /// Slot when rent was last claimed (epoch boundary accounting).
    pub last_claimed_slot: u64,
    /// Rent function parameters for determining compressibility/claims.
    pub rent_config: RentConfig,
    /// Account compression state.
    pub state: CompressionState,
}

#[derive(Debug, Clone, Default, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub enum CompressionState {
    #[default]
    Uninitialized,
    Decompressed,
    Compressed,
}

impl CompressionInfo {
    /// Create a new CompressionInfo initialized from a compressible config.
    pub fn new_from_config(
        cfg: &crate::compressible::CompressibleConfig,
        current_slot: u64,
        rent_sponsor_override: Option<Pubkey>,
    ) -> Self {
        Self {
            config_version: cfg.version as u16,
            compression_authority: cfg.compression_authority,
            rent_sponsor: rent_sponsor_override.unwrap_or(cfg.rent_sponsor),
            lamports_per_write: cfg.write_top_up,
            last_claimed_slot: current_slot,
            rent_config: cfg.rent_config,
            state: CompressionState::Decompressed,
        }
    }

    /// Backward-compat constructor used by older call sites; initializes minimal fields.
    pub fn new_decompressed() -> Result<Self, crate::ProgramError> {
        Ok(Self {
            config_version: 0,
            compression_authority: Pubkey::default(),
            rent_sponsor: Pubkey::default(),
            lamports_per_write: 0,
            last_claimed_slot: Clock::get()?.slot,
            rent_config: RentConfig::default(),
            state: CompressionState::Decompressed,
        })
    }

    /// Update last_claimed_slot to the current slot.
    pub fn bump_last_claimed_slot(&mut self) -> Result<(), crate::ProgramError> {
        self.last_claimed_slot = Clock::get()?.slot;
        Ok(())
    }

    /// Explicitly set last_claimed_slot.
    pub fn set_last_claimed_slot(&mut self, slot: u64) {
        self.last_claimed_slot = slot;
    }

    /// Get last_claimed_slot.
    pub fn last_claimed_slot(&self) -> u64 {
        self.last_claimed_slot
    }

    pub fn set_compressed(&mut self) {
        self.state = CompressionState::Compressed;
    }

    pub fn is_compressed(&self) -> bool {
        self.state == CompressionState::Compressed
    }
}

impl CompressionInfo {
    /// Calculate top-up lamports required for a write.
    /// - Returns 0 if sufficiently funded and skip policy met.
    /// - Returns lamports_per_write if needs regular top-up.
    /// - Returns lamports_per_write + rent deficit if account is compressible.
    pub fn calculate_top_up_lamports(
        &self,
        num_bytes: u64,
        current_slot: u64,
        current_lamports: u64,
        rent_exemption_lamports: u64,
    ) -> u64 {
        use light_compressible::rent::AccountRentState;

        let state = AccountRentState {
            num_bytes,
            current_slot,
            current_lamports,
            last_claimed_slot: self.last_claimed_slot(),
        };
        if let Some(rent_deficit) =
            state.is_compressible(&self.rent_config, rent_exemption_lamports)
        {
            return self.lamports_per_write as u64 + rent_deficit;
        }
        let available_balance = state.get_available_rent_balance(
            rent_exemption_lamports,
            self.rent_config.compression_cost(),
        );
        let rent_per_epoch = self.rent_config.rent_curve_per_epoch(num_bytes);
        let epochs_funded_ahead = available_balance / rent_per_epoch;
        if epochs_funded_ahead >= self.rent_config.max_funded_epochs as u64 {
            0
        } else {
            self.lamports_per_write as u64
        }
    }

    /// Top up rent on write if needed and transfer lamports from payer to account.
    /// This is the standard pattern for all write operations on compressible PDAs.
    ///
    /// # Arguments
    /// * `account_info` - The PDA account to top up
    /// * `payer_info` - The payer account (will be debited)
    ///
    /// # Returns
    /// * `Ok(())` if top-up succeeded or was not needed
    /// * `Err(ProgramError)` if transfer failed
    pub fn top_up_rent(
        &self,
        account_info: &AccountInfo,
        payer_info: &AccountInfo,
    ) -> Result<(), crate::ProgramError> {
        use solana_clock::Clock;
        use solana_sysvar::{rent::Rent, Sysvar};

        let bytes = account_info.data_len() as u64;
        let current_lamports = account_info.lamports();
        let current_slot = Clock::get()?.slot;
        let rent_exemption_lamports = Rent::get()?.minimum_balance(bytes as usize);

        let top_up = self.calculate_top_up_lamports(
            bytes,
            current_slot,
            current_lamports,
            rent_exemption_lamports,
        );

        if top_up > 0 {
            let mut payer_lamports = payer_info.try_borrow_mut_lamports()?;
            let mut dst_lamports = account_info.try_borrow_mut_lamports()?;
            let new_payer = payer_lamports
                .checked_sub(top_up)
                .ok_or(crate::ProgramError::InsufficientFunds)?;
            let new_dst = dst_lamports
                .checked_add(top_up)
                .ok_or(crate::ProgramError::Custom(0))?;
            **payer_lamports = new_payer;
            **dst_lamports = new_dst;
        }

        Ok(())
    }
}

pub trait Space {
    const INIT_SPACE: usize;
}

impl Space for CompressionInfo {
    // 2 (u16 config_version) + 32 (compression_authority) + 32 (rent_sponsor) + 4 (u32 lamports_per_write) + 8 (u64 last_claimed_slot) + size_of::<RentConfig>() + 1 (CompressionState)
    const INIT_SPACE: usize = 2 + 32 + 32 + 4 + 8 + core::mem::size_of::<RentConfig>() + 1;
}

#[cfg(feature = "anchor")]
impl anchor_lang::Space for CompressionInfo {
    const INIT_SPACE: usize = <Self as Space>::INIT_SPACE;
}

/// Compressed account data used when decompressing.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CompressedAccountData<T> {
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    pub data: T,
}

/// Claim completed-epoch rent to the provided rent sponsor and update last_claimed_slot.
/// Returns Some(claimed) if any lamports were claimed; None if account is compressible or nothing to claim.
pub fn claim_completed_epoch_rent<'info, A>(
    account_info: &AccountInfo<'info>,
    account_data: &mut A,
    rent_sponsor: &AccountInfo<'info>,
) -> Result<Option<u64>, ProgramError>
where
    A: HasCompressionInfo,
{
    use light_compressible::rent::{AccountRentState, SLOTS_PER_EPOCH};
    use solana_sysvar::rent::Rent;

    let current_slot = Clock::get()?.slot;
    let bytes = account_info.data_len() as u64;
    let current_lamports = account_info.lamports();
    let rent_exemption_lamports = Rent::get()
        .map_err(|_| ProgramError::Custom(0))?
        .minimum_balance(bytes as usize);

    let ci = account_data.compression_info_mut();
    let state = AccountRentState {
        num_bytes: bytes,
        current_slot,
        current_lamports,
        last_claimed_slot: ci.last_claimed_slot(),
    };

    // If compressible (insufficient for current+next epoch), do not claim
    if state
        .is_compressible(&ci.rent_config, rent_exemption_lamports)
        .is_some()
    {
        return Ok(None);
    }

    // Claim only completed epochs
    let claimable = state.calculate_claimable_rent(&ci.rent_config, rent_exemption_lamports);
    if let Some(amount) = claimable {
        if amount > 0 {
            // Advance last_claimed_slot by completed epochs
            let completed_epochs = state.get_completed_epochs();
            ci.set_last_claimed_slot(
                ci.last_claimed_slot()
                    .saturating_add(completed_epochs * SLOTS_PER_EPOCH),
            );

            // Transfer lamports to rent sponsor
            {
                let mut src = account_info
                    .try_borrow_mut_lamports()
                    .map_err(|_| ProgramError::Custom(0))?;
                let mut dst = rent_sponsor
                    .try_borrow_mut_lamports()
                    .map_err(|_| ProgramError::Custom(0))?;
                let new_src = src
                    .checked_sub(amount)
                    .ok_or(ProgramError::InsufficientFunds)?;
                let new_dst = dst.checked_add(amount).ok_or(ProgramError::Custom(0))?;
                **src = new_src;
                **dst = new_dst;
            }
            return Ok(Some(amount));
        }
    }
    Ok(Some(0))
}
