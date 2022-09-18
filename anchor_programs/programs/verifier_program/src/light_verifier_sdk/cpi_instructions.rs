use anchor_lang::prelude::*;
use merkle_tree_program::utils::config::{
    ENCRYPTED_UTXOS_LENGTH
};
const VERIFIER_INDEX: u64 = 0;
use solana_program;
use anchor_spl::token::{Transfer, CloseAccount};


pub fn initialize_nullifier_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    nullifier_pda: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    rent: &'b AccountInfo<'a>,
    nullifier: [u8; 32],
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];
    let accounts = merkle_tree_program::cpi::accounts::InitializeNullifier {
        authority: authority.clone(),
        nullifier_pda: nullifier_pda.clone(),
        system_program: system_program.clone(),
        rent: rent.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    let res = merkle_tree_program::cpi::initialize_nullifier(cpi_ctx, nullifier, 0u64);
    res
}

pub fn check_merkle_root_exists_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    merkle_tree: &'b AccountInfo<'a>,
    merkle_tree_index: u8,
    merkle_root: [u8; 32],
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = merkle_tree_program::cpi::accounts::CheckMerkleRootExists {
        authority: authority.clone(),
        merkle_tree: merkle_tree.clone(),
    };

    let cpi_ctx2 = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    merkle_tree_program::cpi::check_merkle_root_exists(
        cpi_ctx2,
        0u64,
        merkle_tree_index.into(),
        merkle_root,
    )
}

pub fn withdraw_sol_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    merkle_tree_token: &'b AccountInfo<'a>,
    recipient: &'b AccountInfo<'a>,
    pub_amount_checked: u64,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = merkle_tree_program::cpi::accounts::WithdrawSol {
        authority: authority.clone(),
        merkle_tree_token: merkle_tree_token.clone(),
    };

    let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    cpi_ctx = cpi_ctx.with_remaining_accounts(vec![recipient.clone()]);
    let amount = pub_amount_checked.to_le_bytes().to_vec();
    merkle_tree_program::cpi::withdraw_sol(cpi_ctx, amount, VERIFIER_INDEX, 0u64)

}

pub fn withdraw_spl_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    merkle_tree_token: &'b AccountInfo<'a>,
    recipient: &'b AccountInfo<'a>,
    token_authority: &'b AccountInfo<'a>,
    token_program: &'b AccountInfo<'a>,
    pub_amount_checked: u64,
    merkle_tree_index: u64
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = merkle_tree_program::cpi::accounts::WithdrawSpl {
        authority:          authority.clone(),
        merkle_tree_token:  merkle_tree_token.clone(),
        token_authority:    token_authority.clone(),
        token_program:      token_program.clone()
    };

    let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    cpi_ctx = cpi_ctx.with_remaining_accounts(vec![recipient.clone()]);
    let amount = pub_amount_checked.to_le_bytes().to_vec();
    merkle_tree_program::cpi::withdraw_spl(cpi_ctx, amount, VERIFIER_INDEX, merkle_tree_index)
}

pub fn insert_two_leaves_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    two_leaves_pda: &'b AccountInfo<'a>,
    pre_inserted_leaves_index_account: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    rent: &'b AccountInfo<'a>,
    nullifier: [u8; 32],
    leaf_left: [u8; 32],
    leaf_right: [u8; 32],
    merkle_tree_tmp_account_bytes: [u8; 32],
    encrypted_utxos: [u8; ENCRYPTED_UTXOS_LENGTH],
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];
    let accounts = merkle_tree_program::cpi::accounts::InsertTwoLeaves {
        authority: authority.clone(),
        two_leaves_pda: two_leaves_pda.clone(),
        system_program: system_program.clone(),
        rent: rent.clone(),
        pre_inserted_leaves_index: pre_inserted_leaves_index_account.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    merkle_tree_program::cpi::insert_two_leaves(
        cpi_ctx,
        0u64,
        leaf_left,
        leaf_right,
        [encrypted_utxos.to_vec(), vec![0u8; 34]].concat(),
        nullifier,
        merkle_tree_tmp_account_bytes,
    )
}

/// Deposits spl tokens and closes the spl escrow account.
pub fn deposit_spl_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    relayer: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    escrow_pda: &'b AccountInfo<'a>,
    merkle_tree_token_pda: &'b AccountInfo<'a>,
    token_program: &'b AccountInfo<'a>,
    amount: u64,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = Transfer {
        from:       escrow_pda.clone(),
        to:         merkle_tree_token_pda.clone(),
        authority:  authority.clone()
    };
    let cpi_ctx = CpiContext::new_with_signer(token_program.clone(), accounts, seeds);
    anchor_spl::token::transfer(cpi_ctx, amount)?;

    let accounts_close = CloseAccount {
        account:        escrow_pda.clone(),
        destination:    relayer.clone(),
        authority:      authority.clone()
    };

    let cpi_ctx_close = CpiContext::new_with_signer(token_program.clone(), accounts_close, seeds);
    anchor_spl::token::close_account(cpi_ctx_close)
}

pub fn get_seeds<'a>(
    program_id: &'a Pubkey,
    merkle_tree_program_id: &'a AccountInfo,
) -> Result<([u8; 32], u8)> {
    let (_, bump) = anchor_lang::prelude::Pubkey::find_program_address(
        &[merkle_tree_program_id.key().to_bytes().as_ref()],
        program_id,
    );
    let seed = merkle_tree_program_id.key().to_bytes();
    Ok((seed, bump))
}
