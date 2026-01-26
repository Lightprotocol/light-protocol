use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use light_compressible::rent::RentConfig;
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_cpi::invoke;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_sysvar::Sysvar;

use crate::{instruction::PackedAccounts, AnchorDeserialize, AnchorSerialize, ProgramError};

/// Replace 32-byte Pubkeys with 1-byte indices to save space.
/// If your type has no Pubkeys, just return self.
pub trait Pack {
    type Packed: AnchorSerialize + Clone + std::fmt::Debug;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Result<Self::Packed, ProgramError>;
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
            .ok_or(crate::error::LightSdkError::MissingCompressionInfo.into())
    }

    fn compression_info_mut(&mut self) -> Result<&mut CompressionInfo, ProgramError> {
        self.compression_info_field_mut()
            .as_mut()
            .ok_or(crate::error::LightSdkError::MissingCompressionInfo.into())
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
        + crate::account::Size
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
    pub fn new_from_config(cfg: &crate::interface::LightConfig, current_slot: u64) -> Self {
        Self {
            last_claimed_slot: current_slot,
            lamports_per_write: cfg.write_top_up,
            config_version: cfg.version as u16,
            state: CompressionState::Decompressed,
            _padding: 0,
            rent_config: cfg.rent_config,
        }
    }

    /// Backward-compat constructor used by older call sites; initializes minimal fields.
    /// Rent will flow to config's rent_sponsor upon compression.
    pub fn new_decompressed() -> Result<Self, crate::ProgramError> {
        Ok(Self {
            last_claimed_slot: Clock::get()?.slot,
            lamports_per_write: 0,
            config_version: 0,
            state: CompressionState::Decompressed,
            _padding: 0,
            rent_config: RentConfig::default(),
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

/// Trait for Pod types with a compression_info field at a fixed byte offset.
///
/// Unlike `CompressionInfoField` which works with `Option<CompressionInfo>` (Borsh),
/// this trait works with non-optional `CompressionInfo` at a known byte offset.
///
/// For Pod types, the compression state is indicated by the `state` field:
/// - `state == CompressionState::Uninitialized` means uninitialized
/// - `state == CompressionState::Decompressed` means initialized/decompressed
/// - `state == CompressionState::Compressed` means compressed
///
/// # Safety
/// Implementors must ensure that:
/// 1. The struct is `#[repr(C)]` for predictable field layout
/// 2. The `COMPRESSION_INFO_OFFSET` matches the actual byte offset of the field
/// 3. The struct implements `bytemuck::Pod` and `bytemuck::Zeroable`
/// 4. The `compression_info` field uses SDK `CompressionInfo` (24 bytes)
pub trait PodCompressionInfoField: bytemuck::Pod {
    /// Byte offset of the compression_info field from the start of the struct.
    /// Use `core::mem::offset_of!(Self, compression_info)` to compute this at compile time.
    const COMPRESSION_INFO_OFFSET: usize;

    /// Strip CompressionInfo bytes from Pod data.
    ///
    /// Returns a Vec containing: `pod_bytes[..offset] ++ pod_bytes[offset+24..]`
    ///
    /// This saves 24 bytes per Pod account in instruction data while maintaining
    /// hash consistency (the stripped bytes are what get hashed for the Merkle tree).
    ///
    /// # Arguments
    /// * `pod` - Reference to the Pod struct
    ///
    /// # Returns
    /// A Vec<u8> with CompressionInfo bytes removed
    fn pack_stripped(pod: &Self) -> Vec<u8> {
        let bytes = bytemuck::bytes_of(pod);
        let offset = Self::COMPRESSION_INFO_OFFSET;
        let mut result = Vec::with_capacity(bytes.len() - COMPRESSION_INFO_SIZE);
        result.extend_from_slice(&bytes[..offset]);
        result.extend_from_slice(&bytes[offset + COMPRESSION_INFO_SIZE..]);
        result
    }

    /// Reconstruct Pod from stripped data by inserting canonical compressed CompressionInfo.
    ///
    /// The canonical `CompressionInfo::compressed()` bytes are inserted at the offset.
    /// This ensures hash consistency: compression hashes full bytes with canonical
    /// compressed CompressionInfo, decompression reconstructs the same bytes for verification.
    ///
    /// After verification, `write_decompressed_info_to_slice_pod` patches to Decompressed state.
    ///
    /// # Arguments
    /// * `stripped_bytes` - Byte slice with CompressionInfo bytes removed
    ///
    /// # Returns
    /// * `Ok(Self)` - Reconstructed Pod with canonical compressed CompressionInfo
    /// * `Err` if stripped_bytes length doesn't match expected size
    fn unpack_stripped(stripped_bytes: &[u8]) -> Result<Self, ProgramError> {
        let full_size = core::mem::size_of::<Self>();
        let offset = Self::COMPRESSION_INFO_OFFSET;

        if stripped_bytes.len() != full_size - COMPRESSION_INFO_SIZE {
            return Err(ProgramError::InvalidAccountData);
        }

        // Insert canonical compressed CompressionInfo bytes for hash consistency
        let compressed_info = CompressionInfo::compressed();
        let compressed_info_bytes = bytemuck::bytes_of(&compressed_info);

        let mut full_bytes = vec![0u8; full_size];
        full_bytes[..offset].copy_from_slice(&stripped_bytes[..offset]);
        full_bytes[offset..offset + COMPRESSION_INFO_SIZE].copy_from_slice(compressed_info_bytes);
        full_bytes[offset + COMPRESSION_INFO_SIZE..].copy_from_slice(&stripped_bytes[offset..]);

        Ok(*bytemuck::from_bytes(&full_bytes))
    }

    /// Size of stripped data for this Pod type.
    ///
    /// # Returns
    /// `size_of::<Self>() - COMPRESSION_INFO_SIZE` (i.e., full size minus 24 bytes)
    fn stripped_size() -> usize {
        core::mem::size_of::<Self>() - COMPRESSION_INFO_SIZE
    }

    /// Write decompressed compression_info directly to a byte slice at the correct offset.
    ///
    /// This writes the SDK `CompressionInfo` (24 bytes) with `state = Decompressed`
    /// and default rent parameters.
    ///
    /// # Arguments
    /// * `data` - Mutable slice of the serialized account data (WITHOUT discriminator prefix)
    /// * `current_slot` - Current slot for initializing `last_claimed_slot`
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err` if data slice is too small
    fn write_decompressed_info_to_slice_pod(
        data: &mut [u8],
        current_slot: u64,
    ) -> Result<(), ProgramError> {
        // Use SDK CompressionInfo (24 bytes) - state=Decompressed indicates initialized
        let info = CompressionInfo {
            last_claimed_slot: current_slot,
            lamports_per_write: 0,
            config_version: 1, // 1 = initialized
            state: CompressionState::Decompressed,
            _padding: 0,
            rent_config: RentConfig::default(),
        };

        let info_bytes = bytemuck::bytes_of(&info);
        let offset = Self::COMPRESSION_INFO_OFFSET;
        let end = offset + core::mem::size_of::<CompressionInfo>();

        if data.len() < end {
            return Err(ProgramError::AccountDataTooSmall);
        }

        data[offset..end].copy_from_slice(info_bytes);
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test struct to validate PodCompressionInfoField derive macro behavior.
    /// This struct mimics what a zero-copy account would look like with SDK CompressionInfo.
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    #[repr(C)]
    struct TestPodAccount {
        pub owner: [u8; 32],
        pub counter: u64,
        pub compression_info: CompressionInfo, // SDK version (24 bytes)
    }

    // Manual impl of PodCompressionInfoField since we can't use the derive macro in unit tests
    impl PodCompressionInfoField for TestPodAccount {
        const COMPRESSION_INFO_OFFSET: usize =
            core::mem::offset_of!(TestPodAccount, compression_info);
    }

    #[test]
    fn test_compression_info_size() {
        // Verify CompressionInfo is exactly 24 bytes
        assert_eq!(
            core::mem::size_of::<CompressionInfo>(),
            24,
            "CompressionInfo should be exactly 24 bytes"
        );
    }

    #[test]
    fn test_compression_state_size() {
        // Verify CompressionState is exactly 1 byte
        assert_eq!(
            core::mem::size_of::<CompressionState>(),
            1,
            "CompressionState should be exactly 1 byte"
        );
    }

    #[test]
    fn test_pod_compression_info_offset() {
        // Verify offset_of! works correctly
        let expected_offset = 32 + 8; // owner (32) + counter (8)
        assert_eq!(
            TestPodAccount::COMPRESSION_INFO_OFFSET,
            expected_offset,
            "compression_info offset should be after owner and counter"
        );
    }

    #[test]
    fn test_write_decompressed_info_to_slice_pod() {
        // Create a buffer large enough for TestPodAccount
        let account_size = core::mem::size_of::<TestPodAccount>();
        let mut data = vec![0u8; account_size];

        // Write decompressed info at the correct offset
        let current_slot = 12345u64;
        TestPodAccount::write_decompressed_info_to_slice_pod(&mut data, current_slot)
            .expect("write should succeed");

        // Verify the compression_info was written correctly
        let offset = TestPodAccount::COMPRESSION_INFO_OFFSET;
        let info_size = core::mem::size_of::<CompressionInfo>();
        let info_bytes = &data[offset..offset + info_size];
        let info: &CompressionInfo = bytemuck::from_bytes(info_bytes);

        // Verify decompressed state using SDK CompressionInfo fields
        assert_eq!(
            info.config_version, 1,
            "config_version should be 1 (initialized)"
        );
        assert_eq!(
            info.last_claimed_slot, current_slot,
            "last_claimed_slot should match current_slot"
        );
        assert_eq!(
            info.state,
            CompressionState::Decompressed,
            "state should be Decompressed"
        );
        assert_eq!(info.lamports_per_write, 0, "lamports_per_write should be 0");
    }

    #[test]
    fn test_write_decompressed_info_to_slice_pod_too_small() {
        // Buffer too small to hold the compression_info
        let mut data = vec![0u8; TestPodAccount::COMPRESSION_INFO_OFFSET - 1];

        let result = TestPodAccount::write_decompressed_info_to_slice_pod(&mut data, 0);
        assert!(result.is_err(), "write should fail for buffer too small");
    }

    #[test]
    fn test_pack_stripped() {
        // Create a test account with known values
        let account = TestPodAccount {
            owner: [1u8; 32],
            counter: 42,
            compression_info: CompressionInfo {
                last_claimed_slot: 100,
                lamports_per_write: 200,
                config_version: 1,
                state: CompressionState::Compressed,
                _padding: 0,
                rent_config: RentConfig::default(),
            },
        };

        let stripped = TestPodAccount::pack_stripped(&account);

        // Stripped size should be full size minus COMPRESSION_INFO_SIZE (24 bytes)
        let full_size = core::mem::size_of::<TestPodAccount>();
        assert_eq!(
            stripped.len(),
            full_size - COMPRESSION_INFO_SIZE,
            "stripped size should be {} bytes (full {} - compression_info {})",
            full_size - COMPRESSION_INFO_SIZE,
            full_size,
            COMPRESSION_INFO_SIZE
        );

        // Verify owner bytes are preserved at the start
        assert_eq!(&stripped[..32], &[1u8; 32], "owner should be preserved");

        // Verify counter bytes are preserved after owner
        let counter_bytes = &stripped[32..40];
        assert_eq!(
            u64::from_le_bytes(counter_bytes.try_into().unwrap()),
            42,
            "counter should be preserved"
        );

        // Verify stripped_size() matches
        assert_eq!(
            TestPodAccount::stripped_size(),
            stripped.len(),
            "stripped_size() should match actual stripped length"
        );
    }

    #[test]
    fn test_unpack_stripped() {
        // Create a test account
        let original = TestPodAccount {
            owner: [2u8; 32],
            counter: 123,
            compression_info: CompressionInfo {
                last_claimed_slot: 500,
                lamports_per_write: 300,
                config_version: 2,
                state: CompressionState::Compressed,
                _padding: 0,
                rent_config: RentConfig::default(),
            },
        };

        // Strip it
        let stripped = TestPodAccount::pack_stripped(&original);

        // Unpack it
        let reconstructed =
            TestPodAccount::unpack_stripped(&stripped).expect("unpack_stripped should succeed");

        // Verify non-compression_info fields are preserved
        assert_eq!(reconstructed.owner, original.owner, "owner should match");
        assert_eq!(
            reconstructed.counter, original.counter,
            "counter should match"
        );

        // Verify compression_info has canonical compressed values (for hash consistency)
        assert_eq!(
            reconstructed.compression_info.last_claimed_slot, 0,
            "compression_info.last_claimed_slot should be 0 (canonical compressed)"
        );
        assert_eq!(
            reconstructed.compression_info.state,
            CompressionState::Compressed,
            "compression state should be Compressed (canonical compressed)"
        );
    }

    #[test]
    fn test_unpack_stripped_wrong_size() {
        // Try to unpack with wrong size
        let too_short = vec![0u8; TestPodAccount::stripped_size() - 1];
        let result = TestPodAccount::unpack_stripped(&too_short);
        assert!(result.is_err(), "unpack should fail for wrong size");

        let too_long = vec![0u8; TestPodAccount::stripped_size() + 1];
        let result = TestPodAccount::unpack_stripped(&too_long);
        assert!(result.is_err(), "unpack should fail for wrong size");
    }

    #[test]
    fn test_stripped_roundtrip() {
        // Create account, strip, unpack, verify stripping produces same bytes
        let original = TestPodAccount {
            owner: [3u8; 32],
            counter: 999,
            compression_info: CompressionInfo {
                last_claimed_slot: 1000,
                lamports_per_write: 400,
                config_version: 3,
                state: CompressionState::Compressed,
                _padding: 0,
                rent_config: RentConfig::default(),
            },
        };

        // Strip (removes CompressionInfo bytes)
        let stripped = TestPodAccount::pack_stripped(&original);

        // Unpack (reconstruct with canonical compressed CompressionInfo)
        let reconstructed =
            TestPodAccount::unpack_stripped(&stripped).expect("unpack should succeed");

        // Verify data fields are intact
        assert_eq!(reconstructed.owner, original.owner);
        assert_eq!(reconstructed.counter, original.counter);

        // Now strip the reconstructed version and verify it matches
        let re_stripped = TestPodAccount::pack_stripped(&reconstructed);
        assert_eq!(
            stripped, re_stripped,
            "re-stripping reconstructed account should produce same bytes"
        );
    }

    #[test]
    fn test_hash_consistency() {
        // Create account with canonical compressed CompressionInfo (what compression does)
        let with_canonical = TestPodAccount {
            owner: [4u8; 32],
            counter: 42,
            compression_info: CompressionInfo::compressed(),
        };

        // Get full bytes (what compression would hash)
        let compression_bytes = bytemuck::bytes_of(&with_canonical);

        // Strip and transmit (what goes over the wire)
        let stripped = TestPodAccount::pack_stripped(&with_canonical);

        // Reconstruct (what decompression does)
        let reconstructed =
            TestPodAccount::unpack_stripped(&stripped).expect("unpack should succeed");

        // Get reconstructed full bytes (what decompression would hash)
        let decompression_bytes = bytemuck::bytes_of(&reconstructed);

        // Bytes must match for Merkle tree hash verification to work
        assert_eq!(
            compression_bytes, decompression_bytes,
            "compression and decompression bytes must be identical for hash consistency"
        );
    }
}
