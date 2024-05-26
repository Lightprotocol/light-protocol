use anchor_lang::prelude::*;

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {
    use super::*;

    pub fn create_name(ctx: Context<CreateName>, name: String, parent_name: Option<Pubkey>) -> Result<()> {
        let name_account = &mut ctx.accounts.name_account;

        let seeds = [
            ctx.accounts.owner.key.as_ref(),
            name.as_bytes(),
            parent_name.as_ref().map_or(&[][..], |key| key.as_ref()),
        ];
        let (pda, _) = Pubkey::find_program_address(&seeds, &ctx.program_id);
    

        require!(
            name_account.to_account_info().key == &pda,
            CustomError::Unauthorized
        );

        
        name_account.owner = *ctx.accounts.owner.key;
        name_account.name = name;
        name_account.parent_name = parent_name;
        Ok(())
    }

    pub fn update_name(ctx: Context<UpdateName>, new_name: String) -> Result<()> {
        let name_account = &mut ctx.accounts.name_account;
        require!(name_account.owner == *ctx.accounts.owner.key, CustomError::Unauthorized);
        name_account.name = new_name;
        Ok(())
    }

    pub fn delete_name(ctx: Context<DeleteName>) -> Result<()> {
        let name_account = &ctx.accounts.name_account;
        require!(name_account.owner == *ctx.accounts.owner.key, CustomError::Unauthorized);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateName<'info> {
    #[account(init, payer = owner, space = 8 + 32 + 4 + 32)]
    pub name_account: Account<'info, NameRecord>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateName<'info> {
    #[account(mut, has_one = owner)]
    pub name_account: Account<'info, NameRecord>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct DeleteName<'info> {
    #[account(mut, close = owner, has_one = owner)]
    pub name_account: Account<'info, NameRecord>,
    pub owner: Signer<'info>,
}

#[account]
#[derive(Default)]
pub struct NameRecord {
    pub owner: Pubkey,
    pub name: String,
    pub parent_name: Option<Pubkey>,
}

#[error_code]
pub enum CustomError {
    #[msg("No authority to perform this action")]
    Unauthorized,
}