use anchor_lang::prelude::*;

/// Per-tree reimbursement escrow account.
/// The lamport balance is the only state; the struct itself is empty
/// (Anchor discriminator only, 8 bytes).
#[account]
pub struct ReimbursementPda {}
