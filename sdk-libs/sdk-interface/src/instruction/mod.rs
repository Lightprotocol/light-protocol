// Only available off-chain (client-side) - contains sorting code that exceeds BPF stack limits
#[cfg(not(target_os = "solana"))]
mod pack_accounts;

// Stub type for on-chain compilation - allows trait signatures to compile
// The actual pack methods are never called on-chain
#[cfg(target_os = "solana")]
mod pack_accounts_stub {
    use solana_pubkey::Pubkey;

    /// Stub type for on-chain compilation. The actual implementation with sorting
    /// is only available off-chain. This allows trait signatures that reference
    /// PackedAccounts to compile on Solana.
    pub struct PackedAccounts {
        _phantom: core::marker::PhantomData<()>,
    }

    impl PackedAccounts {
        pub fn insert_or_get(&mut self, _pubkey: Pubkey) -> u8 {
            panic!("PackedAccounts::insert_or_get is not available on-chain")
        }

        pub fn insert_or_get_read_only(&mut self, _pubkey: Pubkey) -> u8 {
            panic!("PackedAccounts::insert_or_get_read_only is not available on-chain")
        }
    }
}

/// Re-exports from light-sdk-types instruction types.
pub use light_sdk_types::instruction::*;
#[cfg(not(target_os = "solana"))]
pub use pack_accounts::*;
#[cfg(target_os = "solana")]
pub use pack_accounts_stub::PackedAccounts;
