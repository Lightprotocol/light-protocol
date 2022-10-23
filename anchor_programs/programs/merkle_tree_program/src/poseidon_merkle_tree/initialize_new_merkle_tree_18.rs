use crate::errors::ErrorCode;
use anchor_lang::prelude::*;

use crate::state::MerkleTree;
use crate::MerkleTreeAuthority;
use anchor_lang::solana_program::{
    msg, pubkey::Pubkey,
};
use std::cell::RefMut;
use crate::utils::constants::MERKLE_TREE_AUTHORITY_SEED;

#[derive(Accounts)]
pub struct InitializeNewMerkleTree<'info> {
    #[account(mut, address=merkle_tree_authority_pda.pubkey @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    /// CHECK: it should be unpacked internally
    #[account(
        init,
        seeds = [&program_id.to_bytes()[..]//, &[0u8;8][..]
        ],
        bump,
        payer = authority,
        space = 8880 //10240 //1698
    )]
    pub merkle_tree: AccountLoader<'info, MerkleTree>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    #[account(
        init,
        payer = authority,
        seeds = [&merkle_tree.key().to_bytes()],
        bump,
        space = 16,
    )]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    #[account(seeds = [&MERKLE_TREE_AUTHORITY_SEED[..]], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

// keeps track of leaves which have been queued but not inserted into the merkle tree yet
#[account]
pub struct MerkleTreePdaToken {}

// keeps track of leaves which have been queued but not inserted into the merkle tree yet
#[account]
pub struct PreInsertedLeavesIndex {
    pub next_index: u64,
}

// TODO: print zero bytes [[u8;32];32]
pub fn process_initialize_new_merkle_tree_18(
    merkle_tree_state_data: &mut RefMut<'_, MerkleTree>,
    height: u64,
    zero_bytes: Vec<[u8; 32]>,
    mt_index: u64,
) {
    for i in 0..height as usize {
        merkle_tree_state_data.filled_subtrees[i] = zero_bytes[i];
    }
    merkle_tree_state_data.height = merkle_tree_state_data
        .filled_subtrees
        .len()
        .try_into()
        .unwrap();
    merkle_tree_state_data.merkle_tree_nr = mt_index;
    merkle_tree_state_data.roots[0] = zero_bytes[height as usize];
    msg!(
        "merkle_tree_state_data.roots[0]: {:?}",
        merkle_tree_state_data.roots[0]
    );
}


#[test]
fn test_init_merkle_tree() {
    let zero_value = vec![
        108, 175, 153, 72, 237, 133, 150, 36, 226, 65, 231, 118, 15, 52, 27, 130, 180, 93, 161,
        235, 182, 53, 58, 52, 243, 171, 172, 211, 96, 76, 229, 47,
    ];
    let mut mt = MerkleTree {
        filled_subtrees: [[0u8; 32]; 18],
        current_root_index: 0u64,
        next_index: 0u64,
        roots: [[0u8; 32]; MERKLE_TREE_HISTORY_SIZE as usize],
        pubkey_locked: Pubkey::new(&[0u8; 32]),
        time_locked: 0u64,
        height: 0u64,
        merkle_tree_nr: 0u64,
    };
    let height = 18;
    let mt_index = 0;
    let binding = &mut RefCell::new(mt);
    let mut ref_mt = binding.borrow_mut();
    process_initialize_new_merkle_tree_18(
        &mut ref_mt,
        height,
        ZERO_BYTES_MERKLE_TREE_18.to_vec(),
        mt_index,
    );

    assert_eq!(ref_mt.height, 18, "height inited wrong");
    assert_eq!(ref_mt.merkle_tree_nr, 0, "merkle_tree_nr inited wrong");
    assert_eq!(
        ref_mt.pubkey_locked,
        Pubkey::new(&[0u8; 32]),
        "pubkey_locked inited wrong"
    );
    assert_eq!(ref_mt.next_index, 0, "next_index inited wrong");
    assert_eq!(
        ref_mt.current_root_index, 0,
        "current_root_index inited wrong"
    );
}
