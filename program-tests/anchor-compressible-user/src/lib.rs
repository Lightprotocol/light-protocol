use anchor_lang::prelude::*;

declare_id!("CompUser11111111111111111111111111111111111");
pub const ADDRESS_SPACE: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const RENT_RECIPIENT: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

// Simple anchor program retrofitted with compressible accounts.
#[program]
pub mod anchor_compressible_user {
    use super::*;

    /// Creates a new user record
    pub fn create_record(
        ctx: Context<CreateRecord>,
        name: String,
        proof: ValidityProof,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        user_record.owner = ctx.accounts.user.key();
        user_record.name = name;
        user_record.score = 0;

        let cpi_accounts = CpiAccounts::new_with_config(
            &ctx.accounts.user, // fee_payer
            &ctx.remaining_accounts[..],
            CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER),
        );
        let new_address_params =
            address_tree_info.into_new_address_params_packed(user_record.key.to_bytes());

        compress_pda_new::<MyPdaAccount>(
            &user_record,
            compressed_address,
            new_address_params,
            output_state_tree_index,
            proof,
            cpi_accounts,
            &crate::ID,
            &ctx.accounts.rent_recipient,
            &ADDRESS_SPACE,
        )?;
        Ok(())
    }

    /// Can be the same because the PDA will be decompressed in a separate instruction.
    /// Updates an existing user record
    pub fn update_record(ctx: Context<UpdateRecord>, name: String, score: u64) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        user_record.name = name;
        user_record.score = score;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 4 + 32 + 8, // discriminator + owner + string len + name + score
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    pub system_program: Program<'info, System>,
    #[account(address = RENT_RECIPIENT)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UpdateRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
        constraint = user_record.owner == user.key()
    )]
    pub user_record: Account<'info, UserRecord>,
}

#[account]
pub struct UserRecord {
    pub owner: Pubkey,
    pub name: String,
    pub score: u64,
}
