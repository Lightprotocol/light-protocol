

/*
/*
pub fn create_authority_config(ctx: Context<CreateAuthorityConfig>) -> Result<()>{
    ctx.accounts
        .handle(*ctx.bumps.get("authority_config").unwrap())
}
pub fn update_authority_config(
    ctx: Context<UpdateAuthorityConfig>,
    new_authority: Pubkey,
) -> Result<()>{
    ctx.accounts.handle(new_authority)
}

pub fn register_new_id(ctx: Context<RegisterNewId>) -> Result<()>{
    ctx.accounts.handle(*ctx.bumps.get("registry").unwrap())
}
*/

// deposits are currently implemented in the verifier program
#[derive(Accounts)]
pub struct DepositSOL<'info> {
    #[account(address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY))]
    pub authority: Signer<'info>,

    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub tmp_storage: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub merkle_tree_token: AccountInfo<'info>,

    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub user_escrow: AccountInfo<'info>,
}*/







/*
// not used right now because already inited merkle tree would not be compatible
#[derive(Accounts)]
#[instruction(nullifier: [u8;32])]
pub struct InitializeLeavesPda<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&(nullifier.as_slice()[0..32]), NF_SEED.as_ref()],
        bump,
        space = 8,
    )]
    pub nullifier_pda: Account<'info, Nullifier>,
    /// CHECKS should be, address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(address=system_program::ID)]
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// not used right now because already inited merkle tree would not be compatible
#[account(zero_copy)]
pub struct LeavesPda {
    pub leaf_right: [u8; 32],
    pub leaf_left: [u8; 32],
    pub merkle_tree_pubkey: Pubkey,
    pub encrypted_utxos: [u8; 222],
    pub left_leaf_index: u64,
}
*/
