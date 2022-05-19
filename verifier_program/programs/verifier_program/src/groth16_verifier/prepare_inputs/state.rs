use anchor_lang::solana_program::system_program;
use anchor_lang::prelude::*;



#[account(zero_copy)]
pub struct PrepareInputsState {
    pub current_instruction_index: u64,
    pub signing_address: Pubkey, // is relayer address
    pub merkle_tree_tmp_account: Pubkey,
    pub relayer_fee: u64,
    pub recipient: Pubkey,
    pub amount: [u8;32],
    pub nullifier_hash: [u8;32],
    pub root_hash: [u8;32],
    pub tx_integrity_hash: [u8;32], // is calculated on-chain from recipient, amount, signing_address,
    pub proof_a_b_c: [u8;256],
    pub ext_amount: [u8;8],
    pub fee: [u8;8],
    pub leaf_left: [u8;32],
    pub leaf_right: [u8;32],
    pub nullifier0: [u8; 32],
    pub nullifier1: [u8;32],

    pub i_1_range: [u8;32],
    pub x_1_range: [u8;64],
    pub i_2_range: [u8;32],
    pub x_2_range: [u8;64],
    pub i_3_range: [u8;32],
    pub x_3_range: [u8;64],
    pub i_4_range: [u8;32],
    pub x_4_range: [u8;64],
    pub i_5_range: [u8;32],
    pub x_5_range: [u8;64],
    pub i_6_range: [u8;32],
    pub x_6_range: [u8;64],
    pub i_7_range: [u8;32],
    pub x_7_range: [u8;64],

    pub res_x_range: [u8;32],
    pub res_y_range: [u8;32],
    pub res_z_range: [u8;32],

    pub g_ic_x_range: [u8;32],
    pub g_ic_y_range: [u8;32],
    pub g_ic_z_range: [u8;32],
    pub current_index: u64,
    pub merkle_tree_index: u8,
    pub found_root: u8,
}

#[derive(Accounts)]
pub struct CreatePrepareInputsState<'info> {
    #[account(init, seeds = [b"data_holder_v0", signing_address.key().as_ref()], bump, payer=signing_address, space= 2048 as usize)]
    pub prepare_inputs_state: AccountLoader<'info, PrepareInputsState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(address = system_program::ID)]
        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}
use anchor_lang::accounts::loader::Loader;
#[derive(Accounts)]
pub struct PrepareInputs<'info> {
    #[account(mut)]
    pub prepare_inputs_state: AccountLoader<'info, PrepareInputsState>,
    pub signing_address: Signer<'info>,
}
