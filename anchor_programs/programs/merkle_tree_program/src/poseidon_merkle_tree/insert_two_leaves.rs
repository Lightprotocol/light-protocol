use anchor_lang::prelude::*;

use crate::state::TwoLeavesBytesPda;
use crate::config;
use anchor_lang::solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use crate::utils::constants::TWO_LEAVES_PDA_SIZE;
use crate::utils::create_pda::create_and_check_pda;
use crate::PreInsertedLeavesIndex;
use crate::utils::constants::UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE;

#[derive(Accounts)]
pub struct InsertTwoLeaves<'info> {
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut, address=solana_program::pubkey::Pubkey::new(&config::REGISTERED_VERIFIER_KEY_ARRAY[0]))]
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub two_leaves_pda: AccountInfo<'info>,
    #[account(mut)]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn process_insert_two_leaves(
    ctx: Context<InsertTwoLeaves>,
    leaf_left: [u8;32],
    leaf_right: [u8;32],
    encrypted_utxos: Vec<u8>,
    nullifier: [u8;32],
    next_index: u64,
    merkle_tree_pda_pubkey: [u8;32]
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
    );
    let mut leaf_pda_account_data = TwoLeavesBytesPda::unpack(&two_leaves_pda.data.borrow())?;

    leaf_pda_account_data.account_type = UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE;
    //save leaves into pda account
    leaf_pda_account_data.node_left = leaf_left.to_vec();
    leaf_pda_account_data.node_right = leaf_right.to_vec();
    //increased by 2 because we're inserting 2 leaves at once
    leaf_pda_account_data.left_leaf_index = ctx.accounts.pre_inserted_leaves_index.next_index.try_into().unwrap();
    leaf_pda_account_data.merkle_tree_pubkey = merkle_tree_pda_pubkey.to_vec();
    // anchor pads encryptedUtxos of length 222 to 254 with 32 zeros in front
    msg!("encrypted_utxos: {:?}", encrypted_utxos.to_vec());
    leaf_pda_account_data.encrypted_utxos = encrypted_utxos[0..222].to_vec();

    TwoLeavesBytesPda::pack_into_slice(
        &leaf_pda_account_data,
        &mut two_leaves_pda.data.borrow_mut(),
    );
    ctx.accounts.pre_inserted_leaves_index.next_index += 2;
    msg!("packed two_leaves_pda");
    Ok(())
}
