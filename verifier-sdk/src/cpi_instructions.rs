use std::str::FromStr;

use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};

use crate::{errors::VerifierSdkError, state::TransactionIndexerEvent};

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

pub fn unshield_sol_cpi<'a, 'b>(
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

    let accounts = light_merkle_tree_program::cpi::accounts::UnshieldSol {
        authority: authority.clone(),
        merkle_tree_token: merkle_tree_token.clone(),
        registered_verifier_pda: registered_verifier_pda.clone(),
        recipient: recipient.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    light_merkle_tree_program::cpi::unshield_sol(cpi_ctx, pub_amount_checked)
}

#[allow(clippy::too_many_arguments)]
pub fn unshield_spl_cpi<'a, 'b>(
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

    let accounts = light_merkle_tree_program::cpi::accounts::UnshieldSpl {
        authority: authority.clone(),
        merkle_tree_token: merkle_tree_token.clone(),
        token_authority: token_authority.clone(),
        token_program: token_program.clone(),
        registered_verifier_pda: registered_verifier_pda.clone(),
        recipient: recipient.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    light_merkle_tree_program::cpi::unshield_spl(cpi_ctx, pub_amount_checked)
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
pub fn insert_two_leaves_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    two_leaves_pda: &'b AccountInfo<'a>,
    transaction_merkle_tree_account: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    registered_verifier_pda: &'b AccountInfo<'a>,
    leaf_left: [u8; 32],
    leaf_right: [u8; 32],
    encrypted_utxos: Vec<u8>,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, merkle_tree_program_id)?;
    let bump = &[bump];
    let seeds = &[&[seed.as_slice(), bump][..]];

    #[cfg(feature = "atomic-transactions")]
    let two_leaves_pda = None;
    #[cfg(not(feature = "atomic-transactions"))]
    let two_leaves_pda = Some(two_leaves_pda.clone());

    let accounts = light_merkle_tree_program::cpi::accounts::InsertTwoLeaves {
        authority: authority.clone(),
        two_leaves_pda,
        system_program: system_program.clone(),
        transaction_merkle_tree: transaction_merkle_tree_account.clone(),
        registered_verifier_pda: registered_verifier_pda.clone(),
    };

    let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);

    light_merkle_tree_program::cpi::insert_two_leaves(
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
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn insert_two_leaves_event_cpi<'a, 'b>(
    program_id: &Pubkey,
    merkle_tree_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    event_merkle_tree: &'b AccountInfo<'a>,
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
        event_merkle_tree: event_merkle_tree.clone(),
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

pub fn invoke_indexer_transaction_event<'info>(
    event: &TransactionIndexerEvent,
    noop_program: &AccountInfo<'info>,
    signer: &AccountInfo<'info>,
) -> Result<()> {
    if noop_program.key()
        != Pubkey::from_str("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV").unwrap()
    {
        return err!(VerifierSdkError::InvalidNoopPubkey);
    }
    let instruction =
        Instruction {
            program_id: noop_program.key(),
            accounts: vec![],
            data: event.try_to_vec()?,
        };
    invoke(
        &instruction,
        &[noop_program.to_account_info(), signer.to_account_info()],
    )?;
    Ok(())
}
