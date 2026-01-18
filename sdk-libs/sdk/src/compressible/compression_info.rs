use std::borrow::Cow;

use light_compressible::rent::RentConfig;
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

#[derive(Debug, Clone, Default, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressionInfo {
    /// Version of the compressible config used to initialize this account.
    pub config_version: u16,
    /// Lamports to top up on each write (from config, stored per-account to avoid passing config on every write)
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
    ///
    /// Rent sponsor is always the config's rent_sponsor (not stored per-account).
    /// This means rent always flows to the protocol's rent pool upon compression,
    /// regardless of who paid for account creation.
    pub fn new_from_config(
        cfg: &crate::compressible::CompressibleConfig,
        current_slot: u64,
    ) -> Self {
        Self {
            config_version: cfg.version as u16,
            lamports_per_write: cfg.write_top_up,
            last_claimed_slot: current_slot,
            rent_config: cfg.rent_config,
            state: CompressionState::Decompressed,
        }
    }

    /// Backward-compat constructor used by older call sites; initializes minimal fields.
    /// Rent will flow to config's rent_sponsor upon compression.
    pub fn new_decompressed() -> Result<Self, crate::ProgramError> {
        Ok(Self {
            config_version: 0,
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
    ///
    /// Logic (same as CTokens):
    /// - If account is compressible (can't pay current + next epoch): return lamports_per_write + deficit
    /// - If account has >= max_funded_epochs: return 0 (no top-up needed)
    /// - Otherwise: return lamports_per_write (maintenance mode)
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

        // If compressible (emergency mode), return lamports_per_write + deficit
        if let Some(rent_deficit) =
            state.is_compressible(&self.rent_config, rent_exemption_lamports)
        {
            return self.lamports_per_write as u64 + rent_deficit;
        }

        let epochs_funded_ahead =
            state.epochs_funded_ahead(&self.rent_config, rent_exemption_lamports);

        // If already at or above target, no top-up needed (cruise control)
        if epochs_funded_ahead >= self.rent_config.max_funded_epochs as u64 {
            return 0;
        }

        // Maintenance mode - add lamports_per_write each time
        self.lamports_per_write as u64
    }

    /// Top up rent on write if needed and transfer lamports from payer to account.
    /// This is the standard pattern for all write operations on compressible PDAs.
    ///
    /// # Arguments
    /// * `account_info` - The PDA account to top up
    /// * `payer_info` - The payer account (will be debited)
    /// * `system_program_info` - The System Program account for CPI
    ///
    /// # Returns
    /// * `Ok(())` if top-up succeeded or was not needed
    /// * `Err(ProgramError)` if transfer failed
    pub fn top_up_rent<'a>(
        &self,
        account_info: &AccountInfo<'a>,
        payer_info: &AccountInfo<'a>,
        system_program_info: &AccountInfo<'a>,
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
            // Use System Program CPI to transfer lamports
            // This is required because the payer account is owned by the System Program,
            // not by the calling program
            transfer_lamports_cpi(payer_info, account_info, system_program_info, top_up)?;
        }

        Ok(())
    }
}

pub trait Space {
    const INIT_SPACE: usize;
}

impl Space for CompressionInfo {
    // 2 (u16 config_version) + 4 (u32 lamports_per_write) + 8 (u64 last_claimed_slot) + size_of::<RentConfig>() + 1 (CompressionState)
    const INIT_SPACE: usize = 2 + 4 + 8 + core::mem::size_of::<RentConfig>() + 1;
}

#[cfg(feature = "anchor")]
impl anchor_lang::Space for CompressionInfo {
    const INIT_SPACE: usize = <Self as Space>::INIT_SPACE;
}

/// Space required for Option<CompressionInfo> when Some (1 byte discriminator + INIT_SPACE).
/// Use this constant in account space calculations.
pub const OPTION_COMPRESSION_INFO_SPACE: usize = 1 + CompressionInfo::INIT_SPACE;

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

/// Transfer lamports from one account to another using System Program CPI.
/// This is required when transferring from accounts owned by the System Program.
///
/// # Arguments
/// * `from` - Source account (owned by System Program)
/// * `to` - Destination account
/// * `system_program` - System Program account
/// * `lamports` - Amount of lamports to transfer
fn transfer_lamports_cpi<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    lamports: u64,
) -> Result<(), ProgramError> {
    use solana_cpi::invoke;
    use solana_instruction::{AccountMeta, Instruction};

    // System Program ID
    const SYSTEM_PROGRAM_ID: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];

    // System Program Transfer instruction discriminator: 2 (u32 little-endian)
    let mut instruction_data = vec![2, 0, 0, 0];
    instruction_data.extend_from_slice(&lamports.to_le_bytes());

    let transfer_instruction = Instruction {
        program_id: Pubkey::from(SYSTEM_PROGRAM_ID),
        accounts: vec![
            AccountMeta::new(*from.key, true),
            AccountMeta::new(*to.key, false),
        ],
        data: instruction_data,
    };

    invoke(
        &transfer_instruction,
        &[from.clone(), to.clone(), system_program.clone()],
    )
}
