use ark_ed_on_bn254::Fq;
use solana_program;
use anchor_lang::AnchorSerialize;
use ark_ff::PrimeField;
use anchor_lang::prelude::*;
use merkle_tree_program::utils::config::ENCRYPTED_UTXOS_LENGTH;
use crate::merkle_tree_program::PreInsertedLeavesIndex;

pub fn initialize_nullifier_cpi<'a, 'b>(
    program_id:             &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority:              &'b AccountInfo<'a>,
    nullifier_pda:          &'b AccountInfo<'a>,
    system_program:         &'b AccountInfo<'a>,
    rent:                   &'b AccountInfo<'a>,
    nullifier:              [u8;32]
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
    merkle_tree_program::cpi::initialize_nullifier(
        cpi_ctx,
        nullifier
    )
}

pub fn check_root_hash_exists_cpi<'a, 'b>(
    program_id:             &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority:              &'b AccountInfo<'a>,
    merkle_tree:            &'b AccountInfo<'a>,
    merkle_tree_index:      u8,
    root_hash:              [u8;32]
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = merkle_tree_program::cpi::accounts::CheckMerkleRootExists {
        authority: authority.clone(),
        merkle_tree: merkle_tree.clone(),
    };

    let cpi_ctx2 = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    merkle_tree_program::cpi::check_root_hash_exists(
        cpi_ctx2,
        merkle_tree_index.into(),
        root_hash
    )
}

pub fn withdraw_sol_cpi<'a, 'b>(
    program_id:             &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority:              &'b AccountInfo<'a>,
    merkle_tree_token:      &'b AccountInfo<'a>,
    recipient:              &'b AccountInfo<'a>,
    pub_amount_checked:      u64,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = merkle_tree_program::cpi::accounts::WithdrawSOL {
        authority: authority.clone(),
        merkle_tree_token: merkle_tree_token.clone(),
    };

    let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    cpi_ctx = cpi_ctx.with_remaining_accounts(vec![recipient.clone()]);
    let amount = pub_amount_checked.to_le_bytes().to_vec();
    merkle_tree_program::cpi::withdraw_sol(cpi_ctx, amount)
}


pub fn insert_two_leaves_cpi<'a, 'b>(
    program_id:                         &Pubkey,
    merkle_tree_program_id:             &'b AccountInfo<'a>,
    authority:                          &'b AccountInfo<'a>,
    two_leaves_pda:                     &'b AccountInfo<'a>,
    pre_inserted_leaves_index_account:  &'b AccountInfo<'a>,
    pre_inserted_leaves_index:          &'b mut anchor_lang::prelude::Account<'a, PreInsertedLeavesIndex>,
    system_program:                     &'b AccountInfo<'a>,
    rent:                               &'b AccountInfo<'a>,
    nullifier:                          [u8;32],
    leaf_left:                          [u8;32],
    leaf_right:                         [u8;32],
    merkle_tree_tmp_account_bytes:      [u8;32],
    encrypted_utxos:                    [u8;ENCRYPTED_UTXOS_LENGTH]
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];
    let accounts = merkle_tree_program::cpi::accounts::InsertTwoLeaves {
        authority: authority.clone(),
        two_leaves_pda: two_leaves_pda.clone(),
        system_program: system_program.clone(),
        rent: rent.clone(),
        pre_inserted_leaves_index: pre_inserted_leaves_index_account.clone()
    };

    let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    pre_inserted_leaves_index.next_index+=2;
    merkle_tree_program::cpi::insert_two_leaves(
        cpi_ctx,
        leaf_left,
        leaf_right,
        [encrypted_utxos.to_vec(),vec![0u8;34]].concat(),
        nullifier,
        pre_inserted_leaves_index.next_index,
        merkle_tree_tmp_account_bytes,
    )
}

pub fn get_seeds<'a>(
    program_id: &'a Pubkey,
    merkle_tree_program_id: &'a AccountInfo
)->Result<([u8;32], u8)> {
    let (_, bump) = solana_program::pubkey::Pubkey::find_program_address(&[merkle_tree_program_id.key().to_bytes().as_ref()], program_id);
    let seed = merkle_tree_program_id.key().to_bytes();
    Ok((seed, bump))
}
