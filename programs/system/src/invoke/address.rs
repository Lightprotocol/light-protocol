use anchor_lang::{prelude::*, Bumps};

use crate::{
    invoke_cpi::verify_signer::check_program_owner_address_merkle_tree,
    sdk::{
        accounts::{InvokeAccounts, SignerAccounts},
        address::derive_address,
    },
    InstructionDataInvoke, NewAddressParamsPacked,
};

// DO NOT MAKE HEAP NEUTRAL: this function allocates new heap memory
pub fn derive_new_addresses<'info, A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps>(
    inputs: &InstructionDataInvoke,
    ctx: &Context<'_, '_, '_, '_, A>,
    input_compressed_account_addresses: &mut Vec<Option<[u8; 32]>>,
    new_addresses: &mut [[u8; 32]],
) {
    inputs
        .new_address_params
        .iter()
        .enumerate()
        .for_each(|(i, new_address_params)| {
            let address = derive_address(
                &ctx.remaining_accounts
                    [new_address_params.address_merkle_tree_account_index as usize]
                    .key(),
                &new_address_params.seed,
            )
            .unwrap();
            input_compressed_account_addresses.push(Some(address));
            new_addresses[i] = address;
        });
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
) -> anchor_lang::Result<()> {
    let mut remaining_accounts = Vec::<AccountInfo>::with_capacity(addresses.len() * 2);
    new_address_params.iter().try_for_each(|params| {
        remaining_accounts
            .push(ctx.remaining_accounts[params.address_queue_account_index as usize].clone());

        remaining_accounts.push(
            ctx.remaining_accounts[params.address_merkle_tree_account_index as usize].clone(),
        );
        check_program_owner_address_merkle_tree(
            &ctx.remaining_accounts[params.address_merkle_tree_account_index as usize],
            invoking_program,
        )
    })?;

    insert_addresses_cpi(
        ctx.program_id,
        ctx.accounts.get_account_compression_program(),
        &ctx.accounts.get_fee_payer().to_account_info(),
        ctx.accounts.get_account_compression_authority(),
        &ctx.accounts.get_registered_program_pda().to_account_info(),
        &ctx.accounts.get_system_program().to_account_info(),
        remaining_accounts,
        addresses.to_vec(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn insert_addresses_cpi<'a, 'b>(
    program_id: &Pubkey,
    account_compression_program_id: &'b AccountInfo<'a>,
    fee_payer: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    registered_program_pda: &'b AccountInfo<'a>,
    system_program: &'b AccountInfo<'a>,
    remaining_accounts: Vec<AccountInfo<'a>>,
    addresses: Vec<[u8; 32]>,
) -> Result<()> {
    let (_, bump) =
        anchor_lang::prelude::Pubkey::find_program_address(&[b"cpi_authority"], program_id);
    let bump = &[bump];
    let seeds = &[&[b"cpi_authority".as_slice(), bump][..]];
    let accounts = account_compression::cpi::accounts::InsertAddresses {
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
