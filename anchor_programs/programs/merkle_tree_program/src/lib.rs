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

pub mod authority_config;
pub mod constant;
pub mod instructions;
pub mod poseidon_merkle_tree;
pub mod processor;
pub mod state;
pub mod utils;
pub mod wrapped_state;

pub mod registry;
pub use registry::*;

use crate::config::MERKLE_TREE_TMP_PDA_SIZE;
use crate::config::STORAGE_SEED;
use crate::config::{ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_INIT_AUTHORITY};
pub use crate::constant::*;
use crate::instructions::create_and_try_initialize_tmp_storage_pda;
use crate::poseidon_merkle_tree::processor::MerkleTreeProcessor;
use crate::utils::config;

use crate::config::NF_SEED;
use crate::state::MerkleTreeTmpPda;
use anchor_lang::system_program;

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
            MerkleTreeProcessor::new(None, Some(&merkle_tree_storage_acc))?;
        merkle_tree_processor
            .initialize_new_merkle_tree_from_bytes(&config::INIT_BYTES_MERKLE_TREE_18[..])?;
        Ok(())
    }

    pub fn initialize_merkle_tree_update_state(
        ctx: Context<InitializeMerkleTreeUpdateState>,
        data: Vec<u8>,
    ) -> Result<()> {
        // we don't need this check
        // let derived_pubkey =
        //     Pubkey::find_program_address(&[&data[0..32], b"storage"], ctx.program_id);

        // if derived_pubkey.0 != *ctx.accounts.merkle_tree_tmp_storage.key {
        //     msg!("Passed-in pda pubkey != on-chain derived pda pubkey.");
        //     msg!("On-chain derived pda pubkey {:?}", derived_pubkey);
        //     msg!(
        //         "Passed-in pda pubkey {:?}",
        //         ctx.accounts.merkle_tree_tmp_storage.key
        //     );
        //     msg!("Instruction data seed  {:?}", data);
        //     return err!(ErrorCode::MtTmpPdaInitFailed);
        // }
        create_and_try_initialize_tmp_storage_pda(
            ctx.program_id,
            &[
                ctx.accounts.authority.to_account_info(),
                // ctx.accounts.verifier_tmp.to_account_info(),
                ctx.accounts.merkle_tree_tmp_storage.to_account_info(),
                // ctx.accounts.system_program.to_account_info(),
                // ctx.accounts.rent.to_account_info(),
            ][..],
            &data.as_slice()[32..],
        )?;
        Ok(())
    }

    pub fn update_merkle_tree<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, UpdateMerkleTree<'info>>,
        data: Vec<u8>,
    ) -> Result<()> {
        msg!("update_merkle_tree");
        let tmp_storage_pda = ctx.accounts.merkle_tree_tmp_storage.to_account_info();
        let mut tmp_storage_pda_data = MerkleTreeTmpPda::unpack(&tmp_storage_pda.data.borrow())?;
        processor::process_instruction(
            ctx.program_id,
            &[
                vec![
                    ctx.accounts.authority.to_account_info(),
                    ctx.accounts.merkle_tree_tmp_storage.to_account_info(),
                    ctx.accounts.merkle_tree.to_account_info(),
                ],
                ctx.remaining_accounts.to_vec(),
            ]
            .concat()
            .as_slice(),
            &mut tmp_storage_pda_data,
            &data.as_slice(),
        )?;
        Ok(())
    }
    /*pub fn deposit_sol(ctx: Context<DepositSOL>, data: Vec<u8>) -> Result<()> {
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
    }*/
    pub fn withdraw_sol<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, WithdrawSOL<'info>>,
        data: Vec<u8>,
    ) -> Result<()> {
        let mut new_data = data.clone();
        new_data.insert(0, 2);

        let mut accounts = ctx.remaining_accounts.to_vec();
        accounts.insert(0, ctx.accounts.authority.to_account_info());
        accounts.insert(1, ctx.accounts.merkle_tree_token.to_account_info());

        processor::process_sol_transfer(
            ctx.program_id,
            &accounts.as_slice(),
            &new_data.as_slice(),
        )?;
        Ok(())
    }

    pub fn create_authority_config(ctx: Context<CreateAuthorityConfig>) -> Result<()> {
        ctx.accounts
            .handle(*ctx.bumps.get("authority_config").unwrap())
    }
    pub fn update_authority_config(
        ctx: Context<UpdateAuthorityConfig>,
        new_authority: Pubkey,
    ) -> Result<()> {
        ctx.accounts.handle(new_authority)
    }

    pub fn register_new_id(ctx: Context<RegisterNewId>) -> Result<()> {
        ctx.accounts.handle(*ctx.bumps.get("registry").unwrap())
    }
    pub fn initialize_nullifier(
        _ctx: Context<InitializeNullifier>,
        _nullifier: [u8; 32],
    ) -> anchor_lang::Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeNewMerkleTree<'info> {
    #[account(address = Pubkey::new(&MERKLE_TREE_INIT_AUTHORITY))]
    pub authority: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: it should be unpacked internally
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(data: Vec<u8>)]
pub struct InitializeMerkleTreeUpdateState<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    // pub verifier_tmp: AccountInfo<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(
        init,
        payer = authority,
        seeds = [&(data.as_slice()[0..32]), STORAGE_SEED.as_ref()],
        bump,
        space = MERKLE_TREE_TMP_PDA_SIZE,
    )]
    pub merkle_tree_tmp_storage: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateMerkleTree<'info> {
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub merkle_tree_tmp_storage: AccountInfo<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
}
/*
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

#[derive(Accounts)]
pub struct WithdrawSOL<'info> {
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut, owner= Pubkey::new(b"2c54pLrGpQdGxJWUAoME6CReBrtDbsx5Tqx4nLZZo6av"))]
    pub merkle_tree_token: AccountInfo<'info>,
    // recipients are specified in additional accounts and checked in the verifier
}

#[derive(Accounts)]
#[instruction(nullifier: [u8;32])]
pub struct InitializeNullifier<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&(nullifier.as_slice()[0..32]), NF_SEED.as_ref()],
        bump,
        space = 8,
    )]
    pub nullifier_pda: Account<'info, Nullifier>,
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// Nullfier pdas are derived from the nullifier
// existence of a nullifier is the check to
// prevent double spends.
#[account]
pub struct Nullifier {}

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

#[account(zero_copy)]
pub struct LeavesPda {
    pub leaf_right: [u8; 32],
    pub leaf_left: [u8; 32],
    pub merkle_tree_pubkey: Pubkey,
    pub encrypted_utxos: [u8; 222],
    pub left_leaf_index: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Merkle tree tmp account init failed wrong pda.")]
    MtTmpPdaInitFailed,
}
