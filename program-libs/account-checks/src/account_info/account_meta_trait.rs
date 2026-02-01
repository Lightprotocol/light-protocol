use core::fmt::Debug;

/// Trait abstracting over AccountMeta implementations (solana vs pinocchio).
///
/// Uses `[u8; 32]` for pubkeys (same pattern as AccountInfoTrait::key()).
/// Implementations convert to/from native types internally.
pub trait AccountMetaTrait: Clone + Debug {
    fn new(pubkey: [u8; 32], is_signer: bool, is_writable: bool) -> Self;
    fn pubkey_bytes(&self) -> [u8; 32];
    fn is_signer(&self) -> bool;
    fn is_writable(&self) -> bool;
    fn set_is_signer(&mut self, val: bool);
    fn set_is_writable(&mut self, val: bool);
}
