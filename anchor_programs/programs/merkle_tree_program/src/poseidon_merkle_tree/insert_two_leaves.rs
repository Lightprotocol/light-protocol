use anchor_lang::prelude::*;

use crate::config;
use crate::state::TwoLeavesBytesPda;
use crate::utils::constants::TWO_LEAVES_PDA_SIZE;
use crate::utils::constants::{LEAVES_SEED, UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE};
use crate::utils::create_pda::create_and_check_pda;
use crate::PreInsertedLeavesIndex;
use anchor_lang::solana_program::{msg, program_pack::Pack};

#[derive(Accounts)]
#[instruction(
    _index: u64,
    leaf_left: [u8;32],
    leaf_right: [u8;32],
    encrypted_utxos: Vec<u8>,
    nullifier: [u8;32],
    merkle_tree_pda_pubkey: [u8;32]
)]
pub struct InsertTwoLeaves<'info> {
    /// CHECK:` should only be accessed by a registered verifier
    /// for every registered verifier a pda is saved in config::REGISTERED_VERIFIER_KEY_ARRAY
    ///
    #[account(mut, address=anchor_lang::prelude::Pubkey::new(&config::REGISTERED_VERIFIER_KEY_ARRAY[usize::try_from(_index).unwrap()]))]
    pub authority: Signer<'info>,
    /// CHECK:` Leaves account should be checked by invoking verifier.
    #[account(mut, seeds= [&nullifier, LEAVES_SEED], bump)]
    pub two_leaves_pda: AccountInfo<'info>,
    #[account(mut, seeds= [&merkle_tree_pda_pubkey], bump)]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn process_insert_two_leaves(
    ctx: Context<InsertTwoLeaves>,
    leaf_left: [u8; 32],
    leaf_right: [u8; 32],
    encrypted_utxos: Vec<u8>,
    nullifier: [u8; 32],
    merkle_tree_pda_pubkey: [u8; 32],
) -> Result<()> {
    msg!("insert_two_leaves");

    let rent = &Rent::from_account_info(&ctx.accounts.rent.to_account_info())?;
    let two_leaves_pda = ctx.accounts.two_leaves_pda.to_account_info();

    msg!("Creating two_leaves_pda.");
    create_and_check_pda(
        &ctx.program_id,
        &ctx.accounts.authority.to_account_info(),
        &two_leaves_pda.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        rent,
        &nullifier,
        &b"leaves"[..],
        TWO_LEAVES_PDA_SIZE, //bytes
        0,                   //lamports
        true,                //rent_exempt
    )?;
    let mut leaf_pda_account_data = TwoLeavesBytesPda::unpack(&two_leaves_pda.data.borrow())?;

    leaf_pda_account_data.account_type = UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE;
    //save leaves into pda account
    leaf_pda_account_data.node_left = leaf_left.to_vec();
    leaf_pda_account_data.node_right = leaf_right.to_vec();
    leaf_pda_account_data.left_leaf_index = ctx
        .accounts
        .pre_inserted_leaves_index
        .next_index
        .try_into()
        .unwrap();
    leaf_pda_account_data.merkle_tree_pubkey = merkle_tree_pda_pubkey.to_vec();
    // Padded encryptedUtxos of length 222 to length 256 for anchor uses serde which is
    // not implemented for [u8;222].
    leaf_pda_account_data.encrypted_utxos = encrypted_utxos[0..222].to_vec();

    TwoLeavesBytesPda::pack_into_slice(
        &leaf_pda_account_data,
        &mut two_leaves_pda.data.borrow_mut(),
    );
    // Increase next index by 2 because we're inserting 2 leaves at once.
    ctx.accounts.pre_inserted_leaves_index.next_index += 2;
    msg!("packed two_leaves_pda");
    Ok(())
}
