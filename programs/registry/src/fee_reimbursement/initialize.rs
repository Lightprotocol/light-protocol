use anchor_lang::prelude::*;
use light_account_checks::discriminator::Discriminator as LightDiscriminator;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_merkle_tree_metadata::STATE_MERKLE_TREE_TYPE_V2;

use crate::errors::RegistryError;
use crate::fee_reimbursement::state::ReimbursementPda;

pub const REIMBURSEMENT_PDA_SEED: &[u8] = b"reimbursement";

#[derive(Accounts)]
pub struct InitReimbursementPda<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [REIMBURSEMENT_PDA_SEED, tree.key().as_ref()],
        bump,
    )]
    pub reimbursement_pda: Account<'info, ReimbursementPda>,
    /// The state tree account. Must be owned by account-compression and have
    /// a valid state tree discriminator (V1 StateMerkleTreeAccount or V2 StateV2).
    /// CHECK: Validated in instruction logic.
    pub tree: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

/// Validates that `tree` is a state tree owned by account-compression and
/// creates the reimbursement PDA.
pub fn process_init_reimbursement_pda(ctx: &Context<InitReimbursementPda>) -> Result<()> {
    let tree = &ctx.accounts.tree;

    // 1. Check owner is account-compression program.
    if tree.owner != &account_compression::ID {
        return err!(RegistryError::InvalidTreeForReimbursementPda);
    }

    // 2. Read discriminator from account data.
    let data = tree.try_borrow_data()?;
    if data.len() < 8 {
        return err!(RegistryError::InvalidTreeForReimbursementPda);
    }
    let discriminator: [u8; 8] = data[..8].try_into().unwrap();

    // Accept V1 state tree discriminator (Anchor-generated).
    if discriminator == account_compression::StateMerkleTreeAccount::DISCRIMINATOR {
        return Ok(());
    }

    // Accept V2 batched merkle tree discriminator, but only if tree_type is StateV2.
    if discriminator == <BatchedMerkleTreeAccount as LightDiscriminator>::LIGHT_DISCRIMINATOR {
        // tree_type is the first field of BatchedMerkleTreeMetadata, at offset 8, u64 LE.
        if data.len() < 16 {
            return err!(RegistryError::InvalidTreeForReimbursementPda);
        }
        let tree_type = u64::from_le_bytes(data[8..16].try_into().unwrap());
        if tree_type == STATE_MERKLE_TREE_TYPE_V2 {
            return Ok(());
        }
    }

    err!(RegistryError::InvalidTreeForReimbursementPda)
}
