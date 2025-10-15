use aligned_sized::aligned_sized;
use bytemuck::{Pod, Zeroable};
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};
pub const COMPRESSION_COST: u16 = 10_000;
pub const COMPRESSION_INCENTIVE: u16 = 1000;

pub const BASE_RENT: u16 = 128;
pub const RENT_PER_BYTE: u8 = 1;
pub const SLOTS_PER_EPOCH: u64 = 6300; // 1.75h

/// Trait for accessing rent configuration parameters.
///
/// This trait allows both owned `RentConfig` and zero-copy versions
/// (`ZRentConfig`, `ZRentConfigMut`) to be used interchangeably with
/// `AccountRentState` methods.
pub trait RentConfigTrait {
    /// Get the base rent value
    fn base_rent(&self) -> u64;

    /// Get the compression cost
    fn compression_cost(&self) -> u64;

    /// Get lamports per byte per epoch
    fn lamports_per_byte_per_epoch(&self) -> u64;

    /// Get maximum funded epochs
    fn max_funded_epochs(&self) -> u64;

    /// Calculate rent per epoch for a given number of bytes
    #[inline(always)]
    fn rent_curve_per_epoch(&self, num_bytes: u64) -> u64 {
        self.base_rent() + num_bytes * self.lamports_per_byte_per_epoch()
    }

    /// Calculate total rent for given bytes and epochs
    #[inline(always)]
    fn get_rent(&self, num_bytes: u64, epochs: u64) -> u64 {
        self.rent_curve_per_epoch(num_bytes) * epochs
    }

    /// Calculate total rent including compression cost
    #[inline(always)]
    fn get_rent_with_compression_cost(&self, num_bytes: u64, epochs: u64) -> u64 {
        self.get_rent(num_bytes, epochs) + self.compression_cost()
    }
}

/// Rent function parameters,
/// used to calculate whether the account is compressible.
#[derive(
    Debug,
    Clone,
    Hash,
    Copy,
    PartialEq,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
    Pod,
    Zeroable,
)]
#[repr(C)]
#[aligned_sized]
pub struct RentConfig {
    /// Base rent constant: rent = base_rent + num_bytes * lamports_per_byte_per_epoch
    pub base_rent: u16,
    pub compression_cost: u16,
    pub lamports_per_byte_per_epoch: u8,
    pub max_funded_epochs: u8, // once the account is funded for max_funded_epochs top up per write is not executed
    pub _padding: [u8; 2],
}

impl Default for RentConfig {
    fn default() -> Self {
        Self {
            base_rent: BASE_RENT,
            compression_cost: COMPRESSION_COST + COMPRESSION_INCENTIVE,
            lamports_per_byte_per_epoch: RENT_PER_BYTE,
            max_funded_epochs: 2, // once the account is funded for max_funded_epochs top up per write is not executed
            _padding: [0; 2],
        }
    }
}

impl RentConfigTrait for RentConfig {
    #[inline(always)]
    fn base_rent(&self) -> u64 {
        self.base_rent as u64
    }

    #[inline(always)]
    fn compression_cost(&self) -> u64 {
        self.compression_cost as u64
    }

    #[inline(always)]
    fn lamports_per_byte_per_epoch(&self) -> u64 {
        self.lamports_per_byte_per_epoch as u64
    }

    #[inline(always)]
    fn max_funded_epochs(&self) -> u64 {
        self.max_funded_epochs as u64
    }
}

impl RentConfig {
    pub fn rent_curve_per_epoch(&self, num_bytes: u64) -> u64 {
        RentConfigTrait::rent_curve_per_epoch(self, num_bytes)
    }
    pub fn get_rent(&self, num_bytes: u64, epochs: u64) -> u64 {
        RentConfigTrait::get_rent(self, num_bytes, epochs)
    }
    pub fn get_rent_with_compression_cost(&self, num_bytes: u64, epochs: u64) -> u64 {
        RentConfigTrait::get_rent_with_compression_cost(self, num_bytes, epochs)
    }
}

// Implement trait for zero-copy immutable reference
impl RentConfigTrait for ZRentConfig<'_> {
    #[inline(always)]
    fn base_rent(&self) -> u64 {
        self.base_rent.into()
    }

    #[inline(always)]
    fn compression_cost(&self) -> u64 {
        self.compression_cost.into()
    }

    #[inline(always)]
    fn lamports_per_byte_per_epoch(&self) -> u64 {
        self.lamports_per_byte_per_epoch as u64
    }

    #[inline(always)]
    fn max_funded_epochs(&self) -> u64 {
        self.max_funded_epochs as u64
    }
}

// Implement trait for zero-copy mutable reference
impl RentConfigTrait for ZRentConfigMut<'_> {
    #[inline(always)]
    fn base_rent(&self) -> u64 {
        self.base_rent.into()
    }

    #[inline(always)]
    fn compression_cost(&self) -> u64 {
        self.compression_cost.into()
    }

    #[inline(always)]
    fn lamports_per_byte_per_epoch(&self) -> u64 {
        self.lamports_per_byte_per_epoch as u64
    }

    #[inline(always)]
    fn max_funded_epochs(&self) -> u64 {
        self.max_funded_epochs as u64
    }
}

impl ZRentConfigMut<'_> {
    /// Sets all fields from a RentConfig instance, handling zero-copy type conversions
    pub fn set(&mut self, config: &RentConfig) {
        self.base_rent = config.base_rent.into();
        self.compression_cost = config.compression_cost.into();
        self.lamports_per_byte_per_epoch = config.lamports_per_byte_per_epoch;
        self.max_funded_epochs = config.max_funded_epochs;
        self._padding = config._padding;
    }
}
