use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{
    transaction_merkle_tree::state::TransactionMerkleTree,
    utils::constants::TRANSACTION_MERKLE_TREE_SEED, RegisteredVerifier,
};

#[derive(Accounts)]
pub struct InsertTwoLeaves<'info> {
    /// CHECK: should only be accessed by a registered verifier.
    #[account(mut, seeds=[__program_id.to_bytes().as_ref()],bump,seeds::program=registered_verifier_pda.pubkey)]
    pub authority: Signer<'info>,
    #[account(mut, seeds = [
        TRANSACTION_MERKLE_TREE_SEED,
        transaction_merkle_tree.load().unwrap().merkle_tree_nr.to_le_bytes().as_ref()
    ], bump)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    pub system_program: Program<'info, System>,
    #[account(seeds=[&registered_verifier_pda.pubkey.to_bytes()],  bump)]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
}

#[cfg(feature = "atomic-transactions")]
pub fn process_insert_two_leaves<'info, 'a>(
    ctx: Context<'a, '_, '_, 'info, InsertTwoLeaves<'info>>,
    leaves: &'a Vec<[u8; 32]>,
) -> Result<()> {
    let merkle_tree = &mut ctx.accounts.transaction_merkle_tree.load_mut()?;

    // Iterate over the leaves in pairs
    for i in (0..leaves.len()).step_by(2) {
        // Get the left leaf
        let leaf_left = &leaves[i];

        // Check if there is a right leaf; use a default value if not
        let leaf_right = if i + 1 < leaves.len() {
            &leaves[i + 1]
        } else {
            return err!(crate::errors::ErrorCode::UnevenNumberOfLeaves);
        };

        // Insert the pair into the merkle tree
        merkle_tree.merkle_tree.insert(*leaf_left, *leaf_right)?;

        // Increase next index by 2 because we're inserting 2 leaves at once
        merkle_tree.next_queued_index += 2;
    }

    Ok(())
}

#[cfg(not(feature = "atomic-transactions"))]
pub fn process_insert_two_leaves<'info, 'a>(
    ctx: Context<'_, '_, '_, 'info, InsertTwoLeaves<'info>>,
    leaves: &'a Vec<[u8; 32]>,
) -> Result<()> {
    use anchor_lang::solana_program::sysvar;
    use light_utils::change_endianness;

    use crate::{
        transaction_merkle_tree::state::TwoLeavesBytesPda,
        utils::{accounts::create_and_check_pda, constants::LEAVES_SEED},
    };

    let account_size = 8 + 3 * 32 + 8 + 8;
    let rent = <Rent as sysvar::Sysvar>::get()?;
    let mut j = 0;
    for i in (0..leaves.len()).step_by(2) {
        create_and_check_pda(
            ctx.program_id,
            &ctx.accounts.authority.to_account_info(),
            &ctx.remaining_accounts[i].to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &rent,
            &leaves[i].as_slice(),
            LEAVES_SEED,
            account_size, //bytes
            0,            //lamports
            true,         //rent_exempt
        )
        .unwrap();
        // Save leaves into PDA.
        let two_leaves_bytes_struct = TwoLeavesBytesPda {
            node_left: change_endianness::<32>(&leaves[i]),
            node_right: change_endianness::<32>(&leaves[i + 1]),
            left_leaf_index: 0,
            merkle_tree_pubkey: ctx.accounts.transaction_merkle_tree.key(),
        };
        let mut account_data = Vec::with_capacity(account_size as usize);

        AccountSerialize::try_serialize(&two_leaves_bytes_struct, &mut account_data)?;
        for (index, byte) in account_data.iter().enumerate() {
            ctx.remaining_accounts[j]
                .to_account_info()
                .data
                .borrow_mut()[index] = *byte;
        }
        j += 1;
        let mut merkle_tree = ctx.accounts.transaction_merkle_tree.load_mut()?;
        // Increase next index by 2 because we're inserting 2 leaves at once.
        merkle_tree.next_queued_index += 2;
    }
    Ok(())
}
