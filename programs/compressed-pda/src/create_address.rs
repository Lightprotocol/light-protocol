use account_compression::AddressQueueAccount;
use anchor_lang::prelude::*;

use crate::{
    append_state::get_seeds,
    instructions::{InstructionDataTransfer, TransferInstruction},
};

pub fn insert_addresses_into_address_merkle_tree_queue<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    addresses: &'a [[u8; 32]],
) -> anchor_lang::Result<()> {
    let address_merkle_tree_account_infos = inputs
        .address_merkle_tree_account_indices
        .iter()
        .map(|index| ctx.remaining_accounts[*index as usize].clone())
        .collect::<Vec<AccountInfo<'info>>>();
    let mut indexed_array_account_infos = Vec::<AccountInfo>::new();
    for index in inputs.address_queue_account_indices.iter() {
        indexed_array_account_infos.push(ctx.remaining_accounts[*index as usize].clone());
        let unpacked_queue_account = AccountLoader::<AddressQueueAccount>::try_from(
            &ctx.remaining_accounts[*index as usize],
        )
        .unwrap();
        let array_account = unpacked_queue_account.load()?;
        let account_is_associated_with_address_merkle_tree = address_merkle_tree_account_infos
            .iter()
            .any(|x| x.key() == array_account.associated_merkle_tree);
        if !account_is_associated_with_address_merkle_tree {
            msg!(
                "Address queue account {:?} is not associated with any address Merkle tree. Provided address Merkle trees {:?}",
                ctx.remaining_accounts[*index as usize].key(), address_merkle_tree_account_infos);
            return Err(crate::ErrorCode::InvalidAddressQueue.into());
        }
    }
    insert_addresses_cpi(
        ctx.program_id,
        &ctx.accounts.account_compression_program,
        &ctx.accounts.psp_account_compression_authority,
        &ctx.accounts.registered_program_pda.to_account_info(),
        indexed_array_account_infos,
        address_merkle_tree_account_infos,
        addresses.to_vec(),
    )
}

pub fn insert_addresses_cpi<'a, 'b>(
    program_id: &Pubkey,
    account_compression_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    registered_program_pda: &'b AccountInfo<'a>,
    adddress_queue_account_infos: Vec<AccountInfo<'a>>,
    address_merkle_tree_account_infos: Vec<AccountInfo<'a>>,
    addresses: Vec<[u8; 32]>,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, &authority.key())?;
    let bump = &[bump];
    let seeds = &[&[b"cpi_authority", seed.as_slice(), bump][..]];
    let accounts = account_compression::cpi::accounts::InsertAddresses {
        authority: authority.to_account_info(),
        registered_program_pda: Some(registered_program_pda.to_account_info()),
    };

    let mut cpi_ctx =
        CpiContext::new_with_signer(account_compression_program_id.clone(), accounts, seeds);
    cpi_ctx
        .remaining_accounts
        .extend(adddress_queue_account_infos);
    cpi_ctx
        .remaining_accounts
        .extend(address_merkle_tree_account_infos);

    account_compression::cpi::insert_addresses(cpi_ctx, addresses)
}
