use anchor_lang::prelude::*;
use merkle_tree_program::utils::config::{
    ENCRYPTED_UTXOS_LENGTH
};
const VERIFIER_INDEX: u64 = 0;
use anchor_spl::token::{Transfer, CloseAccount};


pub fn initialize_nullifier_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    nullifier_pda: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    rent: &'b AccountInfo<'a>,
    registered_verifier_pda: &'b AccountInfo<'a>,
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
        registered_verifier_pda: registered_verifier_pda.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    let res = merkle_tree_program::cpi::initialize_nullifier(cpi_ctx, nullifier, 0u64);
    res
}


pub fn withdraw_sol_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    merkle_tree_token: &'b AccountInfo<'a>,
    recipient: &'b AccountInfo<'a>,
    registered_verifier_pda: &'b AccountInfo<'a>,
    pub_amount_checked: u64,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = merkle_tree_program::cpi::accounts::WithdrawSol {
        authority: authority.clone(),
        merkle_tree_token: merkle_tree_token.clone(),
        registered_verifier_pda: registered_verifier_pda.clone()
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
    registered_verifier_pda: &'b AccountInfo<'a>,
    pub_amount_checked: u64
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = merkle_tree_program::cpi::accounts::WithdrawSpl {
        authority:          authority.clone(),
        merkle_tree_token:  merkle_tree_token.clone(),
        token_authority:    token_authority.clone(),
        token_program:      token_program.clone(),
        registered_verifier_pda: registered_verifier_pda.clone(),
    };

    let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    cpi_ctx = cpi_ctx.with_remaining_accounts(vec![recipient.clone()]);
    let amount = pub_amount_checked.to_le_bytes().to_vec();
    merkle_tree_program::cpi::withdraw_spl(cpi_ctx, amount)
}

pub fn insert_two_leaves_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    two_leaves_pda: &'b AccountInfo<'a>,
    pre_inserted_leaves_index_account: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    rent: &'b AccountInfo<'a>,
    registered_verifier_pda: &'b AccountInfo<'a>,
    nullifier: [u8; 32],
    leaf_left: [u8; 32],
    leaf_right: [u8; 32],
    merkle_tree_tmp_account: Pubkey,
    encrypted_utxos: [u8; ENCRYPTED_UTXOS_LENGTH],
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];
    msg!("authority : {:?}", authority);
    let accounts = merkle_tree_program::cpi::accounts::InsertTwoLeaves {
        authority: authority.clone(),
        two_leaves_pda: two_leaves_pda.clone(),
        system_program: system_program.clone(),
        rent: rent.clone(),
        pre_inserted_leaves_index: pre_inserted_leaves_index_account.clone(),
        registered_verifier_pda: registered_verifier_pda.clone()
    };
    msg!("[encrypted_utxos.to_vec(), vec![0u8; 256 - encrypted_utxos.len()]].concat(): {}", [encrypted_utxos.to_vec(), vec![0u8; 256 - encrypted_utxos.len()]].concat().len());
    msg!("leaf_left {:?}", leaf_left.to_vec());
    msg!("leaf_right {:?}", leaf_right.to_vec());
    msg!("encrypted_utxos {:?}", encrypted_utxos.to_vec());
    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    merkle_tree_program::cpi::insert_two_leaves(
        cpi_ctx,
        leaf_left,
        leaf_right,
        [encrypted_utxos.to_vec(), vec![0u8; 256 - encrypted_utxos.len()]].concat().try_into().unwrap(),
        merkle_tree_tmp_account,
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
        authority:      authority.clone(),
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
