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


// can use for all. acc validation needs be manual. 
#[derive(Accounts)]
pub struct CreateName<'info> {
    #[account(mut)]
    pub signer: Signer<'info>, // this the owner
    /// CHECK:
    #[account(seeds = [b"Light Name Service".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub name_account: AccountInfo<'info>,
    pub light_system_program: Program<'info, light_system_program::program::LightSystemProgram>,
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    /// CHECK:
    pub account_compression_authority: AccountInfo<'info>,
    /// CHECK:
    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK:
    pub noop_program: AccountInfo<'info>,
    pub self_program: Program<'info, crate::program::NameService>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    #[account(mut)]
    pub cpi_context_account: AccountInfo<'info>,
    #[account(init, payer = owner, space = 8 + 32 + 4 + 32)]
    pub name_account: Account<'info, NameRecord>,
}