
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod verifier_program {
     use super::*;

     pub fn create_bar(ctx: Context<CreateBar>) -> Result<()> {
         let bar = &mut ctx.accounts.bar.load_init()?;
         bar.authority = ctx.accounts.authority.key();
         // msg!("data: {:?}" ,data);
         // bar.data = data;
         Ok(())
     }

     pub fn update_bar(ctx: Context<UpdateBar>, data: [u8;32])-> Result<()> {
         for i in 0..32 {
             (*ctx.accounts.bar.load_mut()?).data[i] = data[i];
         }
         Ok(())
     }
}

#[account(zero_copy)]
// #[derive(Default)]
pub struct Bar {
    authority: Pubkey,
    data: [u8;10000]
}
use anchor_lang::solana_program::system_program;
#[derive(Accounts)]
pub struct CreateBar<'info> {
    #[account(init, seeds = [b"data_holder_v0", authority.key().as_ref()], bump, payer=authority, space= 10 * 1024 as usize)]
    pub bar: AccountLoader<'info, Bar>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(address = system_program::ID)]
        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}

#[account(zero_copy)]
// #[derive(Default)]
pub struct Bar1 {
    authority: Pubkey,
    data: [u8;1000]
}

#[derive(Accounts)]
pub struct UpdateBar<'info> {
    #[account(
        mut,
        has_one = authority,
    )]
    pub bar: AccountLoader<'info, Bar>,
    pub authority: Signer<'info>,
}

/*
#[derive(Accounts)]
pub struct Initialize<'info> {
    //#[account(init, payer = user/*, space = 8 + 8 + 32*/)]
    //#[account(init,payer = user, space= 3240 as usize)]
    // #[account(seeds = [b"data_holder_v0", user.key().as_ref()], bump, payer=user, space= 10 * 1024 as usize)]
    #[account(zero)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}
use anchor_lang::AccountSerialize;
//#[assert_size(3240)]
#[account(zero_copy)]
#[derive(Default)]
#[repr(packed)]
pub struct UserAccount {
    // pub merkle_tree_tmp
    //pub number: u64,
    pub i_1_range: [u8;32],
    // pub x_1_range: [u8;32],
    // pub i_2_range: Vec<u8>,
    // pub x_2_range: Vec<u8>,
    // pub i_3_range: Vec<u8>,
    // pub x_3_range: Vec<u8>,
    // pub i_4_range: Vec<u8>,
    // pub x_4_range: Vec<u8>,
    // pub i_5_range: Vec<u8>,
    // pub x_5_range: Vec<u8>,
    // pub i_6_range: Vec<u8>,
    // pub x_6_range: Vec<u8>,
    // pub i_7_range: Vec<u8>,
    // pub x_7_range: Vec<u8>,
    //
    // pub res_x_range: Vec<u8>,
    // pub res_y_range: Vec<u8>,
    // pub res_z_range: Vec<u8>,
    // pub g_ic_x_range: Vec<u8>,
    // pub g_ic_y_range: Vec<u8>,
    // pub g_ic_z_range: Vec<u8>,
    // pub current_instruction_index: usize,
}
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RpcUserAccount {
    pub data: [u8;32],
}

impl From<RpcUserAccount> for UserAccount {
    fn from(e: RpcUserAccount) -> UserAccount {
        UserAccount {
            i_1_range: e.data,
        }
    }
}*/
