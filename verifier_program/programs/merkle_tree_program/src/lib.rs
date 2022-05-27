use anchor_lang::prelude::*;

declare_id!("2c54pLrGpQdGxJWUAoME6CReBrtDbsx5Tqx4nLZZo6av");
use solana_program::program_pack::Pack;
use solana_security_txt::security_txt;

security_txt! {
    name: "light_protocol_merkle_tree",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-program/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-program/program_merkle_tree"
}

pub mod instructions;
pub mod poseidon_merkle_tree;
pub mod processor;
pub mod state;
pub mod utils;
pub mod constant;
pub mod authority_config;

use crate::utils::config;
use crate::config::{ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_INIT_AUTHORITY, TREE_ROOT_SEED};
use crate::poseidon_merkle_tree::processor::MerkleTreeProcessor;
pub use crate::constant::*;
use crate::instructions::create_and_try_initialize_tmp_storage_pda;

use crate::state::MerkleTreeTmpPda;
use crate::poseidon_merkle_tree::state::MerkleTree;

pub use authority_config::*;

#[program]
pub mod merkle_tree_program {
    use super::*;

    pub fn initialize_new_merkle_tree(ctx: Context<InitializeNewMerkleTree>) -> Result<()> {
        let merkle_tree_storage_acc = ctx.accounts.merkle_tree.to_account_info();
        let rent = Rent::get()?;

        if !rent.is_exempt(
            **merkle_tree_storage_acc.lamports.borrow(),
            merkle_tree_storage_acc.data.borrow().len(),
        ) {
            msg!("Account is not rent exempt.");
            return Err(ProgramError::AccountNotRentExempt.try_into().unwrap());
        }
        let mut merkle_tree_processor =
            MerkleTreeProcessor::new(None, Some(&merkle_tree_storage_acc), crate::ID)?;
        merkle_tree_processor
            .initialize_new_merkle_tree_from_bytes(&config::INIT_BYTES_MERKLE_TREE_18[..])?;
        Ok(())
    }
    pub fn initialize_tmp_merkle_tree(ctx: Context<InitializeTmpMerkleTree>, data: Vec<u8>) -> Result<()> {
        create_and_try_initialize_tmp_storage_pda(
            ctx.program_id,
            &[
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.verifier_tmp.to_account_info(),
                ctx.accounts.merkle_tree_tmp.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ][..],
            &data.as_slice()
        )?;
        Ok(())
    }

    pub fn update_merkle_tree(ctx: Context<UpdateMerkleTree>, data: Vec<u8>) -> Result<()> {
        let tmp_storage_pda = ctx.accounts.merkle_tree.to_account_info();
        let mut tmp_storage_pda_data = MerkleTreeTmpPda::unpack(&tmp_storage_pda.data.borrow())?;
        processor::process_instruction(
            ctx.program_id, 
            &[
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.merkle_tree.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ], 
            &mut tmp_storage_pda_data,
            &data.as_slice()
        )?;
        Ok(())
    }
    pub fn deposit_sol(ctx: Context<DepositSOL>, data: Vec<u8>) -> Result<()> {
        let mut new_data = data.clone();
        new_data.insert(0, 1);
        processor::process_sol_transfer(
            ctx.program_id,
            &[
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.tmp_storage.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.merkle_tree_token.to_account_info(),
                ctx.accounts.user_escrow.to_account_info(),
            ], 
            &new_data.as_slice()
        )?;
        Ok(())
    }
    pub fn withdraw_sol<'a, 'b, 'c, 'info>(ctx: Context<'a, 'b, 'c, 'info, WithdrawSOL<'info>>, data: Vec<u8>) -> Result<()> {
        
        let mut new_data = data.clone();
        new_data.insert(0, 2);

        let mut accounts = ctx.remaining_accounts.to_vec();
        accounts.insert(0, ctx.accounts.authority.to_account_info());
        accounts.insert(1, ctx.accounts.tmp_storage.to_account_info());
        accounts.insert(2, ctx.accounts.merkle_tree_token.to_account_info());
        
        processor::process_sol_transfer(
            ctx.program_id,
            &accounts.as_slice(), 
            &new_data.as_slice()
        )?;
        Ok(())
    }

    pub fn create_authority_config(ctx: Context<CreateAuthorityConfig>) -> Result<()> {
        ctx.accounts.handle(*ctx.bumps.get("authority_config").unwrap())
    }
    pub fn update_authority_config(ctx: Context<UpdateAuthorityConfig>, new_authority: Pubkey) -> Result<()> {
        ctx.accounts.handle(new_authority)
    }
}

#[derive(Accounts)]
pub struct InitializeNewMerkleTree<'info> {
    #[account(address = Pubkey::new(&MERKLE_TREE_INIT_AUTHORITY))]
    pub authority: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,

    /// CHECK: it should be unpacked internally
    #[account(
        mut,
        // init,
        // payer = authority,
        // seeds = [TREE_ROOT_SEED.as_ref()],
        // bump,
        // space = MerkleTree::LEN,
    )]
    pub merkle_tree: AccountInfo<'info>,

}

#[derive(Accounts)]
pub struct InitializeTmpMerkleTree<'info> {
    #[account(address = Pubkey::new(&MERKLE_TREE_INIT_AUTHORITY))]
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub verifier_tmp: AccountInfo<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub merkle_tree_tmp: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateMerkleTree<'info> {
    #[account(address = Pubkey::new(&MERKLE_TREE_INIT_AUTHORITY))]
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct DepositSOL<'info> {
    #[account(mut)]
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
}


#[derive(Accounts)]
pub struct WithdrawSOL<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub tmp_storage: AccountInfo<'info>,
    
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub merkle_tree_token: AccountInfo<'info>,
}

