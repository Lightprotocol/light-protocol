use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

pub const POOL_SEED: &[u8] = b"pool";
pub const TOKEN_SEED: &[u8] = b"token";

#[derive(Debug)]
#[aligned_sized(anchor)]
#[account]
pub struct TokenPool {
    pub enable_decompress: bool,
}

/// creates a token pool account which is owned by the token authority pda
#[derive(Accounts)]
pub struct CreateTokenPoolInstruction<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(
        init,
        seeds = [
            POOL_SEED, &mint.key().to_bytes(),
        ],
        bump,
        space = TokenPool::LEN,
        payer = fee_payer,
    )]
    pub token_pool_pda: Account<'info, TokenPool>,
    #[account(
        init,
        seeds = [
            TOKEN_SEED, &mint.key().to_bytes(),
        ],
        bump,
        payer = fee_payer,
        token::mint = mint,
        token::authority = cpi_authority_pda,
    )]
    pub token_pda: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    /// CHECK:
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
}

pub fn process_create_token_pool<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateTokenPoolInstruction<'info>>,
) -> Result<()> {
    ctx.accounts.token_pool_pda.enable_decompress = false;
    Ok(())
}

#[derive(Accounts)]
pub struct TokenPoolSetEnableDecompress<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, owner = authority.key())]
    pub token_pool_pda: Account<'info, TokenPool>,
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
}

pub fn process_token_pool_set_enable_decompress<'info>(
    ctx: Context<'_, '_, '_, 'info, TokenPoolSetEnableDecompress<'info>>,
    enable: bool,
) -> Result<()> {
    ctx.accounts.token_pool_pda.enable_decompress = enable;
    Ok(())
}

#[cfg(not(target_os = "solana"))]
pub mod pool_sdk {
    use anchor_lang::{system_program, InstructionData, ToAccountMetas};
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::{get_token_pda, get_token_pool_pda, process_transfer::get_cpi_authority_pda};

    pub fn create_create_token_pool_instruction(fee_payer: &Pubkey, mint: &Pubkey) -> Instruction {
        let token_pool_pda = get_token_pool_pda(mint);
        let token_pda = get_token_pda(mint);
        let instruction_data = crate::instruction::CreateTokenPool {};

        let accounts = crate::accounts::CreateTokenPoolInstruction {
            fee_payer: *fee_payer,
            token_pool_pda,
            token_pda,
            system_program: system_program::ID,
            mint: *mint,
            token_program: anchor_spl::token::ID,
            cpi_authority_pda: get_cpi_authority_pda().0,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }

    pub fn create_token_pool_set_enable_decompress(
        authority: &Pubkey,
        mint: &Pubkey,
        enable: bool,
    ) -> Instruction {
        let token_pool_pda = get_token_pool_pda(mint);
        let instruction_data = crate::instruction::TokenPoolSetEnableDecompress { enable };

        let accounts = crate::accounts::TokenPoolSetEnableDecompress {
            authority: *authority,
            token_pool_pda,
            cpi_authority_pda: get_cpi_authority_pda().0,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }
}
