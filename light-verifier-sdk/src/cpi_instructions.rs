use anchor_lang::prelude::*;

pub fn insert_nullifiers_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    registered_verifier_pda: &'b AccountInfo<'a>,
    nullifiers: Vec<[u8; 32]>,
    nullifier_pdas: Vec<AccountInfo<'a>>,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];
    let accounts = merkle_tree_program::cpi::accounts::InitializeNullifiers {
        authority: authority.clone(),
        system_program: system_program.clone(),
        registered_verifier_pda: registered_verifier_pda.clone(),
    };

    let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    cpi_ctx = cpi_ctx.with_remaining_accounts(nullifier_pdas);

    merkle_tree_program::cpi::initialize_nullifiers(cpi_ctx, nullifiers)
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
        registered_verifier_pda: registered_verifier_pda.clone(),
        recipient: recipient.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    merkle_tree_program::cpi::withdraw_sol(cpi_ctx, pub_amount_checked)
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_spl_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    merkle_tree_token: &'b AccountInfo<'a>,
    recipient: &'b AccountInfo<'a>,
    token_authority: &'b AccountInfo<'a>,
    token_program: &'b AccountInfo<'a>,
    registered_verifier_pda: &'b AccountInfo<'a>,
    pub_amount_checked: u64,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = merkle_tree_program::cpi::accounts::WithdrawSpl {
        authority: authority.clone(),
        merkle_tree_token: merkle_tree_token.clone(),
        token_authority: token_authority.clone(),
        token_program: token_program.clone(),
        registered_verifier_pda: registered_verifier_pda.clone(),
        recipient: recipient.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    merkle_tree_program::cpi::withdraw_spl(cpi_ctx, pub_amount_checked)
}

#[allow(clippy::too_many_arguments)]
pub fn insert_two_leaves_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    two_leaves_pda: &'b AccountInfo<'a>,
    merkle_tree_account: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    registered_verifier_pda: &'b AccountInfo<'a>,
    leaf_left: [u8; 32],
    leaf_right: [u8; 32],
    encrypted_utxos: Vec<u8>,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = merkle_tree_program::cpi::accounts::InsertTwoLeaves {
        authority: authority.clone(),
        two_leaves_pda: two_leaves_pda.clone(),
        system_program: system_program.clone(),
        merkle_tree: merkle_tree_account.clone(),
        registered_verifier_pda: registered_verifier_pda.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    merkle_tree_program::cpi::insert_two_leaves(
        cpi_ctx,
        leaf_left,
        leaf_right,
        [
            encrypted_utxos.to_vec(),
            vec![0u8; 256 - encrypted_utxos.len()],
        ]
        .concat()
        .try_into()
        .unwrap(),
    )
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
