use account_compression::{config_accounts::GroupAuthority, program::AccountCompression};
use anchor_lang::prelude::*;
#[cfg(not(target_os = "solana"))]
pub mod sdk;

declare_id!("5WzvRtu7LABotw1SUEpguJiKU27LRGsiCnF5FH6VV7yP");

#[error_code]
pub enum ErrorCode {
    #[msg("Sum check failed")]
    SumCheckFailed,
}

#[constant]
pub const AUTHORITY_PDA_SEED: &[u8] = b"authority";

#[constant]
pub const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";

#[program]
pub mod light {

    use anchor_lang::solana_program::pubkey::Pubkey;

    use super::*;

    pub fn initialize_governance_authority(
        ctx: Context<InitializeAuthority>,
        authority: Pubkey,
        rewards: Vec<u64>,
        bump: u8,
    ) -> Result<()> {
        ctx.accounts.authority_pda.authority = authority;
        ctx.accounts.authority_pda.bump = bump;
        ctx.accounts.authority_pda.rewards = rewards;
        Ok(())
    }

    // TODO: add test
    pub fn update_governance_authority_reward(
        ctx: Context<UpdateAuthority>,
        reward: u64,
        index: u64,
    ) -> Result<()> {
        if ctx.accounts.authority_pda.rewards.len() <= index as usize {
            ctx.accounts.authority_pda.rewards.push(reward);
        } else {
            ctx.accounts.authority_pda.rewards[index as usize] = reward;
        }
        Ok(())
    }

    pub fn update_governance_authority(
        ctx: Context<UpdateAuthority>,
        bump: u8,
        new_authority: Pubkey,
    ) -> Result<()> {
        ctx.accounts.authority_pda.authority = new_authority;
        ctx.accounts.authority_pda.bump = bump;
        Ok(())
    }

    pub fn register_system_program(
        ctx: Context<RegisteredProgram>,
        bump: u8,
        program_id: Pubkey,
    ) -> Result<()> {
        let program_id_seed = ctx.program_id.to_bytes();
        let bump = &[bump];
        let seeds = [CPI_AUTHORITY_PDA_SEED, program_id_seed.as_slice(), bump];
        let signer_seeds = &[&seeds[..]];

        let accounts = account_compression::cpi::accounts::RegisterProgramToGroup {
            authority: ctx.accounts.cpi_authority.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
            group_authority_pda: ctx.accounts.group_pda.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        account_compression::cpi::register_program_to_group(cpi_ctx, program_id)
    }

    // TODO: update rewards field
    // signer is light governance authority

    // TODO: sync rewards
    // signer is registered relayer
    // sync rewards field with Light Governance Authority rewards field

    // TODO: add register relayer
    // signer is light governance authority
    // creates a registered relayer pda which is derived from the relayer pubkey,
    // with fields: signer_pubkey, points_counter, rewards: Vec<u64>, last_rewards_sync

    // TODO: deregister relayer
    // signer is light governance authority

    // TODO: update registered relayer
    // signer is registered relayer
    // update the relayer signer pubkey in the pda

    // TODO: add rollover Merkle tree with rewards
    // signer is registered relayer
    // cpi to account compression program rollover Merkle tree
    // increment points in registered relayer account

    // TODO: add rollover lookup table with rewards
    // signer is registered relayer
    // cpi to account compression program rollover lookup table
    // increment points in registered relayer account

    // TODO: add nullify utxo with rewards
    // signer is registered relayer
    // cpi to account compression program nullify utxo
    // increment points in registered relayer account
}

#[account]
pub struct LightGovernanceAuthority {
    pub authority: Pubkey,
    pub bump: u8,
    pub _padding: [u8; 7],
    pub rewards: Vec<u64>, // initing with storage for 8 u64s TODO: add instruction to resize
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializeAuthority<'info> {
    // TODO: add check that this is upgrade authority
    #[account(mut)]
    authority: Signer<'info>,
    /// CHECK:
    #[account(init, seeds = [AUTHORITY_PDA_SEED, __program_id.to_bytes().as_slice()], bump, space = 8 + 32 + 8 + 8 * 8, payer = authority)]
    authority_pda: Account<'info, LightGovernanceAuthority>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct UpdateAuthority<'info> {
    #[account(mut, constraint = authority.key() == authority_pda.authority)]
    authority: Signer<'info>,
    /// CHECK:
    #[account(mut, seeds = [AUTHORITY_PDA_SEED, __program_id.to_bytes().as_slice()], bump)]
    authority_pda: Account<'info, LightGovernanceAuthority>,
}

#[derive(Accounts)]
pub struct RegisteredProgram<'info> {
    #[account(mut, constraint = authority.key() == authority_pda.authority)]
    authority: Signer<'info>,
    /// CHECK:
    #[account(mut, seeds = [AUTHORITY_PDA_SEED, __program_id.to_bytes().as_slice()], bump)]
    authority_pda: Account<'info, LightGovernanceAuthority>,
    /// CHECK: this is
    #[account(mut, seeds = [CPI_AUTHORITY_PDA_SEED, __program_id.to_bytes().as_slice()], bump)]
    cpi_authority: AccountInfo<'info>,
    #[account(mut)]
    group_pda: Account<'info, GroupAuthority>,
    account_compression_program: Program<'info, AccountCompression>,
    system_program: Program<'info, System>,
    /// CHECK:
    registered_program_pda: AccountInfo<'info>,
}
