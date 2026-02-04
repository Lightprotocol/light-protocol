use core::fmt::Debug;

/// Trait abstracting over AccountMeta implementations (solana vs pinocchio).
///
/// The associated `Pubkey` type allows callers to pass native pubkey types
/// (e.g. `solana_pubkey::Pubkey` or `[u8; 32]`) without manual conversion.
pub trait AccountMetaTrait: Clone + Debug {
    /// The native pubkey type for this account meta implementation.
    /// - `solana_pubkey::Pubkey` for `solana_instruction::AccountMeta`
    /// - `[u8; 32]` for pinocchio's `OwnedAccountMeta`
    type Pubkey: Copy;

    fn new(pubkey: Self::Pubkey, is_signer: bool, is_writable: bool) -> Self;
    fn pubkey_to_bytes(pubkey: Self::Pubkey) -> [u8; 32];
    fn pubkey_from_bytes(bytes: [u8; 32]) -> Self::Pubkey;
    fn pubkey_bytes(&self) -> [u8; 32];
    fn is_signer(&self) -> bool;
    fn is_writable(&self) -> bool;
    fn set_is_signer(&mut self, val: bool);
    fn set_is_writable(&mut self, val: bool);
}
