extern crate alloc;
use alloc::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use light_compressible::rent::RentConfig;
use light_sdk_types::instruction::PackedStateTreeInfo;
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;

use super::pack::Unpack;
use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum AccountState {
    Initialized,
    Frozen,
}

pub trait HasCompressionInfo {
    fn compression_info(&self) -> Result<&CompressionInfo, ProgramError>;
    fn compression_info_mut(&mut self) -> Result<&mut CompressionInfo, ProgramError>;
    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo>;
    fn set_compression_info_none(&mut self) -> Result<(), ProgramError>;
}

/// Simple field accessor trait for types with a `compression_info: Option<CompressionInfo>` field.
/// Implement this trait and get `HasCompressionInfo` for free via blanket impl.
pub trait CompressionInfoField {
    /// True if `compression_info` is the first field, false if last.
    /// This enables efficient serialization by skipping at a known offset.
    const COMPRESSION_INFO_FIRST: bool;

    fn compression_info_field(&self) -> &Option<CompressionInfo>;
    fn compression_info_field_mut(&mut self) -> &mut Option<CompressionInfo>;

    /// Write `Some(CompressionInfo::new_decompressed())` directly into serialized account data.
    ///
    /// This avoids re-serializing the entire account by writing only the compression_info
    /// bytes at the correct offset (first or last field position).
    ///
    /// # Arguments
    /// * `data` - Mutable slice of the serialized account data (WITHOUT discriminator prefix)
    /// * `current_slot` - Current slot for initializing `last_claimed_slot`
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err` if serialization fails or data is too small
    fn write_decompressed_info_to_slice(
        data: &mut [u8],
        current_slot: u64,
    ) -> Result<(), ProgramError> {
        use crate::AnchorSerialize;

        let info = CompressionInfo {
            last_claimed_slot: current_slot,
            lamports_per_write: 0,
            config_version: 0,
            state: CompressionState::Decompressed,
            _padding: 0,
            rent_config: light_compressible::rent::RentConfig::default(),
        };

        // Option<T> serializes as: 1 byte discriminant + T if Some
        let option_size = OPTION_COMPRESSION_INFO_SPACE;

        let offset = if Self::COMPRESSION_INFO_FIRST {
            0
        } else {
            data.len().saturating_sub(option_size)
        };

        if data.len() < offset + option_size {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let target = &mut data[offset..offset + option_size];
        // Write Some discriminant
        target[0] = 1;
        // Write CompressionInfo
        info.serialize(&mut &mut target[1..])
            .map_err(|_| ProgramError::BorshIoError("compression_info serialize failed".into()))?;

        Ok(())
    }
}

impl<T: CompressionInfoField> HasCompressionInfo for T {
    fn compression_info(&self) -> Result<&CompressionInfo, ProgramError> {
        self.compression_info_field()
            .as_ref()
            .ok_or(crate::error::LightPdaError::MissingCompressionInfo.into())
    }

    fn compression_info_mut(&mut self) -> Result<&mut CompressionInfo, ProgramError> {
        self.compression_info_field_mut()
            .as_mut()
            .ok_or(crate::error::LightPdaError::MissingCompressionInfo.into())
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        self.compression_info_field_mut()
    }

    fn set_compression_info_none(&mut self) -> Result<(), ProgramError> {
        *self.compression_info_field_mut() = None;
        Ok(())
    }
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
        + HasCompressionInfo
        + Default
        + Clone;

    fn compress_as(&self) -> Cow<'_, Self::Output>;
}

/// SDK CompressionInfo - a compact 24-byte struct for custom zero-copy PDAs.
///
/// This is the lightweight version of compression info used in the SDK.
/// CToken has its own compression handling via `light_compressible::CompressionInfo`.
///
/// # Memory Layout (24 bytes with #[repr(C)])
/// - `last_claimed_slot`: u64 @ offset 0 (8 bytes, 8-byte aligned)
/// - `lamports_per_write`: u32 @ offset 8 (4 bytes)
/// - `config_version`: u16 @ offset 12 (2 bytes)
/// - `state`: CompressionState @ offset 14 (1 byte)
/// - `_padding`: u8 @ offset 15 (1 byte)
/// - `rent_config`: RentConfig @ offset 16 (8 bytes, 2-byte aligned)
///
/// Fields are ordered for optimal alignment to achieve exactly 24 bytes.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, AnchorSerialize, AnchorDeserialize, Pod, Zeroable,
)]
#[repr(C)]
pub struct CompressionInfo {
    /// Slot when rent was last claimed (epoch boundary accounting).
    pub last_claimed_slot: u64,
    /// Lamports to top up on each write (from config, stored per-account to avoid passing config on every write)
    pub lamports_per_write: u32,
    /// Version of the compressible config used to initialize this account.
    pub config_version: u16,
    /// Account compression state.
    pub state: CompressionState,
    pub _padding: u8,
    /// Rent function parameters for determining compressibility/claims.
    pub rent_config: RentConfig,
}

/// Compression state for SDK CompressionInfo.
///
/// This enum uses #[repr(u8)] for Pod compatibility:
/// - Uninitialized = 0 (default, account not yet set up)
/// - Decompressed = 1 (account is decompressed/active on Solana)
/// - Compressed = 2 (account is compressed in Merkle tree)
#[derive(Debug, Clone, Copy, Default, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionState {
    #[default]
    Uninitialized = 0,
    Decompressed = 1,
    Compressed = 2,
}

// Safety: CompressionState is #[repr(u8)] with explicit discriminants
unsafe impl bytemuck::Pod for CompressionState {}
unsafe impl bytemuck::Zeroable for CompressionState {}

impl CompressionInfo {
    pub fn compressed() -> Self {
        Self {
            last_claimed_slot: 0,
            lamports_per_write: 0,
            config_version: 0,
            state: CompressionState::Compressed,
            _padding: 0,
            rent_config: RentConfig {
                base_rent: 0,
                compression_cost: 0,
                lamports_per_byte_per_epoch: 0,
                max_funded_epochs: 0,
                max_top_up: 0,
            },
        }
    }

    /// Create a new CompressionInfo initialized from a compressible config.
    ///
    /// Rent sponsor is always the config's rent_sponsor (not stored per-account).
    /// This means rent always flows to the protocol's rent pool upon compression,
    /// regardless of who paid for account creation.
    pub fn new_from_config(cfg: &crate::program::config::LightConfig, current_slot: u64) -> Self {
        Self {
            last_claimed_slot: current_slot,
            lamports_per_write: cfg.write_top_up,
            config_version: cfg.version as u16,
            state: CompressionState::Decompressed,
            _padding: 0,
            rent_config: cfg.rent_config,
        }
    }

    /// Backward-compat constructor; initializes minimal fields.
    /// Rent will flow to config's rent_sponsor upon compression.
    pub fn new_decompressed(current_slot: u64) -> Self {
        Self {
            last_claimed_slot: current_slot,
            lamports_per_write: 0,
            config_version: 0,
            state: CompressionState::Decompressed,
            _padding: 0,
            rent_config: RentConfig::default(),
        }
    }

    /// Update last_claimed_slot to the given slot.
    pub fn bump_last_claimed_slot(&mut self, current_slot: u64) {
        self.last_claimed_slot = current_slot;
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
    ) -> Result<(), ProgramError> {
        use solana_sysvar::{rent::Rent, Sysvar};

        let bytes = account_info.data_len() as u64;
        let current_lamports = account_info.lamports();
        let current_slot = solana_sysvar::clock::Clock::get()?.slot;
        let rent_exemption_lamports = Rent::get()?.minimum_balance(bytes as usize);

        let top_up = self.calculate_top_up_lamports(
            bytes,
            current_slot,
            current_lamports,
            rent_exemption_lamports,
        );

        if top_up > 0 {
            // Use System Program CPI to transfer lamports
            transfer_lamports_cpi(payer_info, account_info, system_program_info, top_up)?;
        }

        Ok(())
    }
}

pub trait Space {
    const INIT_SPACE: usize;
}

impl Space for CompressionInfo {
    // 8 (u64 last_claimed_slot) + 4 (u32 lamports_per_write) + 2 (u16 config_version) + 1 (CompressionState) + 1 padding + 8 (RentConfig) = 24 bytes
    const INIT_SPACE: usize = core::mem::size_of::<CompressionInfo>();
}

#[cfg(feature = "anchor")]
impl anchor_lang::Space for CompressionInfo {
    const INIT_SPACE: usize = <Self as Space>::INIT_SPACE;
}

/// Space required for Option<CompressionInfo> when Some (1 byte discriminator + INIT_SPACE).
/// Use this constant in account space calculations.
pub const OPTION_COMPRESSION_INFO_SPACE: usize = 1 + CompressionInfo::INIT_SPACE;

/// Size of SDK CompressionInfo in bytes (24 bytes).
/// Used for stripping CompressionInfo from Pod data during packing.
pub const COMPRESSION_INFO_SIZE: usize = core::mem::size_of::<CompressionInfo>();

/// Compressed account data used when decompressing.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CompressedAccountData<T> {
    pub tree_info: PackedStateTreeInfo,
    pub data: T,
}

impl Unpack for CompressedAccountData<Vec<u8>> {
    type Unpacked = Vec<u8>;

    fn unpack(&self, _remaining_accounts: &[AccountInfo]) -> Result<Self::Unpacked, ProgramError> {
        unimplemented!()
    }
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
    use solana_sysvar::{rent::Rent, Sysvar};

    let current_slot = solana_sysvar::clock::Clock::get()?.slot;
    let bytes = account_info.data_len() as u64;
    let current_lamports = account_info.lamports();
    let rent_exemption_lamports = Rent::get()
        .map_err(|_| ProgramError::Custom(0))?
        .minimum_balance(bytes as usize);

    let ci = account_data.compression_info_mut()?;
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
fn transfer_lamports_cpi<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    lamports: u64,
) -> Result<(), ProgramError> {
    use solana_cpi::invoke;
    use solana_instruction::{AccountMeta, Instruction};
    use solana_pubkey::Pubkey;

    // System Program Transfer instruction discriminator: 2 (u32 little-endian)
    let mut instruction_data = vec![2, 0, 0, 0];
    instruction_data.extend_from_slice(&lamports.to_le_bytes());

    let transfer_instruction = Instruction {
        program_id: Pubkey::default(), // System Program ID
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
