use anchor_lang::prelude::Pubkey;
use anchor_lang::prelude::*;
// use merkle_tree_program::{self, program::MerkleTreeProgram};
// use solana_program;
// use merkle_tree_program::PreInsertedLeavesIndex;
declare_id!("3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL");
// use merkle_tree_program::utils::constants::NF_SEED;
// use verifier_program::last_transaction::cpi_instructions::{
//     check_merkle_root_exists_cpi,
//     // insert_two_leaves_cpi,
//     // withdraw_sol_cpi
// };
#[program]
pub mod attacker_program {
    use super::*;
    /*
    pub fn test_nullifier_insert(
        ctx: Context<TestNullifierInsert>,
        nullifer: [u8; 32]
    ) -> Result<()> {
        let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();

        let (address, bump) = solana_program::pubkey::Pubkey::find_program_address(
            &[merkle_tree_program_id.key().to_bytes().as_ref()],
            ctx.program_id,
        );
        msg!("find_program_address: {:?}", address);
        msg!("ctx.accounts.authority: {:?}", ctx.accounts.authority.key());

        let bump = &[bump][..];
        let seed = &merkle_tree_program_id.key().to_bytes()[..];
        let seeds = &[&[seed, bump][..]];
        msg!("seeds: {:?}", seeds);
        // authority.is_signer = true;
        // msg!("authority1 {:?}", authority);
        let accounts = merkle_tree_program::cpi::accounts::InitializeNullifier {
            authority: ctx.accounts.authority.to_account_info(),
            nullifier_pda: ctx.accounts.nullifier0_pda.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
        merkle_tree_program::cpi::initialize_nullifier(cpi_ctx, nullifer, 0u64).unwrap();
        Ok(())
    }

    pub fn test_check_merkle_root_exists(
        ctx: Context<TestNullifierInsert>, _nullifier0: [u8;32]
    ) -> Result<()> {
        let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
        let program_id = ctx.program_id;
        // check_merkle_root_exists_cpi(
        //     &ctx.program_id,
        //     &merkle_tree_program_id,
        //     &ctx.accounts.authority.to_account_info(),
        //     &ctx.accounts.authority.to_account_info(),
        //     0u8,
        //     [0u8;32],
        // )?;
        let merkle_root = [
            2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176,
            253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
        ];//[1u8;32];
        let merkle_tree_index = 0u64;
        let (seed, bump) = get_seeds(program_id, &merkle_tree_program_id)?;
        let bump = &[bump];
        let seeds = &[&[seed.as_slice(), bump][..]];
        msg!("ctx.accounts.authority.to_account_info(): {:?}", ctx.accounts.authority.to_account_info());
        msg!("ctx.accounts.merkle_tree.to_account_info(): {:?}", ctx.accounts.merkle_tree.to_account_info());
        msg!("seeds: {:?}", seeds);

        let accounts = merkle_tree_program::cpi::accounts::CheckMerkleRootExists {
            authority: ctx.accounts.authority.to_account_info(),
            merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        };
        msg!("here");

        let cpi_ctx2 = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
        merkle_tree_program::cpi::check_merkle_root_exists(
            cpi_ctx2,
            0u64,
            merkle_tree_index,
            merkle_root,
        )
    }

    pub fn test_insert_two_leaves(
        ctx: Context<TestNullifierInsert>, _nullifier0: [u8;32]
    ) -> Result<()> {
        let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
        let program_id = ctx.program_id;
        // insert_two_leaves_cpi(
        //     &ctx.program_id,
        //     &merkle_tree_program_id,
        //     &ctx.accounts.authority.to_account_info(),
        //     &ctx.accounts.authority.to_account_info(),
        //     &ctx.accounts.authority.to_account_info(),
        //     &ctx.accounts.system_program.to_account_info(),
        //     &ctx.accounts.rent.to_account_info(),
        //     [0u8;32], //verifier_state.nullifier0
        //     [0u8;32], //verifier_state.leaf_left,
        //     [0u8;32], //verifier_state.leaf_right,
        //     [0u8;32], //ctx.accounts.authority.to_account_info(),
        //     [0u8;222] //verifier_state.encrypted_utxos,
        // )
        let (seed, bump) = get_seeds(program_id, &merkle_tree_program_id)?;
        let bump = &[bump];
        let seeds = &[&[seed.as_slice(), bump][..]];
        let accounts = merkle_tree_program::cpi::accounts::InsertTwoLeaves {
            authority: ctx.accounts.authority.to_account_info(),
            two_leaves_pda: ctx.accounts.nullifier0_pda.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
            pre_inserted_leaves_index: ctx.accounts.pre_inserted_leaves_index.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
        merkle_tree_program::cpi::insert_two_leaves(
            cpi_ctx,
            0u64,
            [1u8;32],
            [2u8;32],
            vec![1u8; 256],
            [3u8;32],
            [4u8;32],
        )
    }
    /// Should fail because of the pda cannot be used from another contract
    pub fn test_withdraw_sol(
        ctx: Context<TestNullifierInsert>, _nullifier0: [u8;32]
    ) -> Result<()> {
        let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
        let program_id = ctx.program_id;

        let (seed, bump) = get_seeds(program_id, &merkle_tree_program_id)?;
        let bump = &[bump];
        let seeds = &[&[seed.as_slice(), bump][..]];

        let accounts = merkle_tree_program::cpi::accounts::WithdrawSol {
            authority: ctx.accounts.authority.to_account_info(),
            merkle_tree_token: ctx.accounts.merkle_tree.to_account_info(),
        };

        let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
        cpi_ctx = cpi_ctx.with_remaining_accounts(vec![ctx.accounts.signing_address.to_account_info()]);
        // let amount = pub_amount_checked;
        merkle_tree_program::cpi::withdraw_sol(cpi_ctx, (1_000_000_000u64).to_le_bytes().to_vec(), 0u64, 0u64)
    }
    /// Should fail because of the pda cannot be used from another contract
    pub fn test_withdraw_sol_signer(
        ctx: Context<TestNullifierInsert>,
    ) -> Result<()> {
        let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
        let program_id = ctx.program_id;

        let (seed, bump) = get_seeds(program_id, &merkle_tree_program_id)?;
        let bump = &[bump];
        let _seeds = &[&[seed.as_slice(), bump][..]];

        let accounts = merkle_tree_program::cpi::accounts::WithdrawSol {
            authority: ctx.accounts.signing_address.to_account_info(),
            merkle_tree_token: ctx.accounts.merkle_tree.to_account_info(),
        };

        let mut cpi_ctx = CpiContext::new(merkle_tree_program_id.clone(), accounts);
        cpi_ctx = cpi_ctx.with_remaining_accounts(vec![ctx.accounts.signing_address.to_account_info()]);
        // let amount = pub_amount_checked;
        merkle_tree_program::cpi::withdraw_sol(cpi_ctx, (1_000_000_000u64).to_le_bytes().to_vec(), 0u64, 0u64)
    }
    */
    pub fn testr(
        ctx: Context<Test>,
    ) -> Result<()> {
        let merkle_tree_program_id = ctx.accounts.nullifier0_pda.to_account_info();
        let program_id = ctx.program_id;

        let (seed, bump) = get_seeds(program_id, &merkle_tree_program_id)?;
        let bump = &[bump];
        let _seeds = &[&[seed.as_slice(), bump][..]];

        // let amount = pub_amount_checked;
        Ok(())
    }
}
/*
#[derive(Accounts)]
#[instruction(nullifier: [u8;32])]
pub struct TestNullifierInsert<'info> {
    #[account(
        mut,
        seeds = [nullifier.as_ref(), NF_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub nullifier0_pda: UncheckedAccount<'info>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    /// CHECK:` should be a pda
    // #[account(init, payer = signing_address, space=0)]
    #[account(mut)]
    pub authority: UncheckedAccount<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    // #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    #[account(mut)]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>
}

#[error_code]
pub enum ErrorCode {
    #[msg("Incompatible Verifying Key")]
    IncompatibleVerifyingKey,
    #[msg("WrongPubAmount")]
    WrongPubAmount,
    #[msg("PrepareInputsDidNotFinish")]
    PrepareInputsDidNotFinish,
    #[msg("NotLastTransactionState")]
    NotLastTransactionState,
    #[msg("Tx is not a deposit")]
    NotDeposit,
    #[msg("WrongTxIntegrityHash")]
    WrongTxIntegrityHash,
}
*/
pub fn get_seeds<'a>(
    program_id: &'a Pubkey,
    merkle_tree_program_id: &'a AccountInfo,
) -> Result<([u8; 32], u8)> {
    let (_, bump) = Pubkey::find_program_address(
        &[merkle_tree_program_id.key().to_bytes().as_ref()],
        program_id,
    );
    let seed = merkle_tree_program_id.key().to_bytes();
    Ok((seed, bump))
}



#[derive(Accounts)]
pub struct Test<'info> {
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub nullifier0_pda: UncheckedAccount<'info>,
    pub signing_address: Signer<'info>,
}
