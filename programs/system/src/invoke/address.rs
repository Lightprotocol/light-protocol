use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::{prelude::*, Bumps};

use crate::{
    constants::CPI_AUTHORITY_PDA_BUMP,
    invoke_cpi::verify_signer::check_program_owner_address_merkle_tree,
    sdk::{
        accounts::{InvokeAccounts, SignerAccounts},
        address::derive_address,
    },
    NewAddressParamsPacked,
};

pub fn derive_new_addresses(
    new_address_params: &[NewAddressParamsPacked],
    num_input_compressed_accounts: usize,
    remaining_accounts: &[AccountInfo],
    compressed_account_addresses: &mut [Option<[u8; 32]>],
    new_addresses: &mut [[u8; 32]],
) -> Result<()> {
    new_address_params
        .iter()
        .enumerate()
        .try_for_each(|(i, new_address_params)| {
            let address = derive_address(
                &remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
                    .key(),
                &new_address_params.seed,
            )
            .map_err(ProgramError::from)?;
            // We are inserting addresses into two vectors to avoid unwrapping
            // the option in following functions.
            compressed_account_addresses[i + num_input_compressed_accounts] = Some(address);
            new_addresses[i] = address;
            Ok(())
        })
}

pub fn insert_addresses_into_address_merkle_tree_queue<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    addresses: &'a [[u8; 32]],
    new_address_params: &'a [NewAddressParamsPacked],
    invoking_program: &Option<Pubkey>,
) -> anchor_lang::Result<Option<(u8, u64)>> {
    let mut remaining_accounts = Vec::<AccountInfo>::with_capacity(new_address_params.len() * 2);
    let mut network_fee_bundle = None;

    new_address_params.iter().try_for_each(|params| {
        remaining_accounts
            .push(ctx.remaining_accounts[params.address_queue_account_index as usize].clone());

        remaining_accounts.push(
            ctx.remaining_accounts[params.address_merkle_tree_account_index as usize].clone(),
        );
        // If at least one new address is created an address network fee is
        // paid.The network fee is paid once per transaction, defined in the
        // state Merkle tree and transferred to the nullifier queue because the
        // nullifier queue is mutable. The network fee field in the queue is not
        // used.
        let network_fee = check_program_owner_address_merkle_tree(
            &ctx.remaining_accounts[params.address_merkle_tree_account_index as usize],
            invoking_program,
        )?;
        // We select the first network fee we find. All Merkle trees are
        // initialized with the same network fee.
        if network_fee_bundle.is_none() && network_fee.is_some() {
            network_fee_bundle = Some((params.address_queue_account_index, network_fee.unwrap()));
        }
        anchor_lang::Result::Ok(())
    })?;

    insert_addresses_cpi(
        ctx.accounts.get_account_compression_program(),
        &ctx.accounts.get_fee_payer().to_account_info(),
        ctx.accounts.get_account_compression_authority(),
        &ctx.accounts.get_registered_program_pda().to_account_info(),
        &ctx.accounts.get_system_program().to_account_info(),
        remaining_accounts,
        addresses.to_vec(),
    )?;
    Ok(network_fee_bundle)
}

#[allow(clippy::too_many_arguments)]
pub fn insert_addresses_cpi<'a, 'b>(
    account_compression_program_id: &'b AccountInfo<'a>,
    fee_payer: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    registered_program_pda: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    remaining_accounts: Vec<AccountInfo<'a>>,
    addresses: Vec<[u8; 32]>,
) -> Result<()> {
    let bump = &[CPI_AUTHORITY_PDA_BUMP];
    let seeds = &[&[CPI_AUTHORITY_PDA_SEED, bump][..]];
    let accounts = account_compression::cpi::accounts::InsertIntoQueues {
        fee_payer: fee_payer.to_account_info(),
        authority: authority.to_account_info(),
        registered_program_pda: Some(registered_program_pda.to_account_info()),
        system_program: system_program.to_account_info(),
    };

    let mut cpi_ctx =
        CpiContext::new_with_signer(account_compression_program_id.clone(), accounts, seeds);
    cpi_ctx.remaining_accounts.extend(remaining_accounts);

    account_compression::cpi::insert_addresses(cpi_ctx, addresses)
}
