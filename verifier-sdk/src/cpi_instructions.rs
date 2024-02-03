use std::str::FromStr;

use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};

use crate::errors::VerifierSdkError;

#[inline(never)]
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

    let accounts = light_merkle_tree_program::cpi::accounts::InitializeNullifiers {
        authority: authority.clone(),
        system_program: system_program.clone(),
        registered_verifier_pda: registered_verifier_pda.clone(),
    };

    let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    cpi_ctx = cpi_ctx.with_remaining_accounts(nullifier_pdas);

    light_merkle_tree_program::cpi::initialize_nullifiers(cpi_ctx, nullifiers)
}

pub fn decompress_sol_cpi<'a, 'b>(
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

    let accounts = light_merkle_tree_program::cpi::accounts::DecompressSol {
        authority: authority.clone(),
        merkle_tree_token: merkle_tree_token.clone(),
        registered_verifier_pda: registered_verifier_pda.clone(),
        recipient: recipient.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    light_merkle_tree_program::cpi::decompress_sol(cpi_ctx, pub_amount_checked)
}

#[allow(clippy::too_many_arguments)]
pub fn decompress_spl_cpi<'a, 'b>(
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
    let accounts = light_merkle_tree_program::cpi::accounts::DecompressSpl {
        authority: authority.clone(),
        merkle_tree_token: merkle_tree_token.clone(),
        token_authority: token_authority.clone(),
        token_program: token_program.clone(),
        registered_verifier_pda: registered_verifier_pda.clone(),
        recipient: recipient.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    light_merkle_tree_program::cpi::decompress_spl(cpi_ctx, pub_amount_checked)
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
#[inline(never)]
pub fn insert_two_leaves_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    merkle_tree_set: &'b AccountInfo<'a>,
    registered_verifier_pda: &'b AccountInfo<'a>,
    leaves: Vec<[u8; 32]>,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];
    let accounts = light_merkle_tree_program::cpi::accounts::InsertTwoLeaves {
        authority: authority.to_account_info(),
        merkle_tree_set: merkle_tree_set.to_account_info(),
        registered_verifier_pda: registered_verifier_pda.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    light_merkle_tree_program::cpi::insert_two_leaves(cpi_ctx, leaves)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
#[inline(never)]
pub fn insert_two_leaves_parallel_cpi<'a, 'b>(
    program_id: &Pubkey,
    psp_account_compression_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    registered_verifier_pda: &'b AccountInfo<'a>,
    leaves: Vec<[u8; 32]>,
    transaction_merkle_tree_accounts: Vec<AccountInfo<'a>>,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, psp_account_compression_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = psp_account_compression::cpi::accounts::InsertTwoLeavesParallel {
        authority: authority.to_account_info(),
        registered_verifier_pda: None,
    };

    let mut cpi_ctx =
        CpiContext::new_with_signer(psp_account_compression_program_id.clone(), accounts, seeds);
    cpi_ctx.remaining_accounts = transaction_merkle_tree_accounts;
    psp_account_compression::cpi::insert_leaves_into_merkle_trees(cpi_ctx, leaves)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
#[inline(never)]
pub fn insert_public_nullifier_into_indexed_array_cpi<'a, 'b>(
    program_id: &Pubkey,
    psp_account_compression_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    registered_verifier_pda: &'b AccountInfo<'a>,
    in_utxos: Vec<[u8; 32]>,
    low_element_indexes: Vec<u16>,
    indexed_array_accounts: Vec<AccountInfo<'a>>,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, psp_account_compression_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = psp_account_compression::cpi::accounts::InsertIntoIndexedArrays {
        authority: authority.to_account_info(),
        registered_verifier_pda: None,
    };

    let mut cpi_ctx =
        CpiContext::new_with_signer(psp_account_compression_program_id.clone(), accounts, seeds);
    cpi_ctx.remaining_accounts = indexed_array_accounts;
    psp_account_compression::cpi::insert_into_indexed_arrays(
        cpi_ctx,
        in_utxos,
        low_element_indexes,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub fn insert_two_leaves_event_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    merkle_tree_set: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    registered_verifier: &'b AccountInfo<'a>,
    leaf_left: &'b [u8; 32],
    leaf_right: &'b [u8; 32],
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    let accounts = light_merkle_tree_program::cpi::accounts::InsertTwoLeavesEvent {
        authority: authority.clone(),
        merkle_tree_set: merkle_tree_set.clone(),
        system_program: system_program.clone(),
        registered_verifier: registered_verifier.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    light_merkle_tree_program::cpi::insert_two_leaves_event(
        cpi_ctx,
        leaf_left.to_owned(),
        leaf_right.to_owned(),
    )?;
    Ok(())
}

#[inline(never)]
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

#[inline(never)]
pub fn invoke_indexer_transaction_event<'info, T>(
    event: &T,
    noop_program: &AccountInfo<'info>,
) -> Result<()>
where
    T: AnchorSerialize,
{
    if noop_program.key()
        != Pubkey::from_str("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV").unwrap()
    {
        return err!(VerifierSdkError::InvalidNoopPubkey);
    }
    let instruction = Instruction {
        program_id: noop_program.key(),
        accounts: vec![],
        data: event.try_to_vec()?,
    };
    invoke(&instruction, &[noop_program.to_account_info()])?;
    Ok(())
}
