use account_compression::{
    errors::AccountCompressionErrorCode, utils::constants::CPI_AUTHORITY_PDA_SEED,
    AddressMerkleTreeAccount,
};
use anchor_lang::{prelude::*, Bumps, Discriminator};
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_hasher::Discriminator as LightDiscriminator;

use crate::{
    constants::CPI_AUTHORITY_PDA_BUMP,
    errors::SystemProgramError,
    invoke_cpi::verify_signer::check_program_owner_address_merkle_tree,
    sdk::{
        accounts::{InvokeAccounts, SignerAccounts},
        address::{derive_address, derive_address_legacy},
    },
    NewAddressParamsPacked,
};

pub fn derive_new_addresses(
    invoking_program_id: &Option<Pubkey>,
    new_address_params: &[NewAddressParamsPacked],
    num_input_compressed_accounts: usize,
    remaining_accounts: &[AccountInfo],
    compressed_account_addresses: &mut [Option<[u8; 32]>],
    new_addresses: &mut Vec<[u8; 32]>,
) -> Result<()> {
    let invoking_program_id_bytes = invoking_program_id
        .as_ref()
        .map(|invoking_program_id| invoking_program_id.to_bytes());

    new_address_params
        .iter()
        .enumerate()
        .try_for_each(|(i, new_address_params)| {
            let mut discriminator_bytes = [0u8; 8];
            discriminator_bytes.copy_from_slice(
                &remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
                    .try_borrow_data()?[0..8],
            );
            let address = match discriminator_bytes {
                AddressMerkleTreeAccount::DISCRIMINATOR => derive_address_legacy(
                    &remaining_accounts
                        [new_address_params.address_merkle_tree_account_index as usize]
                        .key(),
                    &new_address_params.seed,
                )
                .map_err(ProgramError::from)?,
                BatchedMerkleTreeAccount::DISCRIMINATOR => {
                    let invoking_program_id_bytes =
                        if let Some(bytes) = invoking_program_id_bytes.as_ref() {
                            Ok(bytes)
                        } else {
                            err!(SystemProgramError::DeriveAddressError)
                        }?;
                    msg!("invoking_program_id_bytes: {:?}", invoking_program_id_bytes);
                    msg!("new_address_params.seed: {:?}", new_address_params.seed);
                    msg!(
                        "merkle tree: {:?}",
                        remaining_accounts
                            [new_address_params.address_merkle_tree_account_index as usize]
                            .key()
                            .to_bytes()
                    );
                    derive_address(
                        &new_address_params.seed,
                        &remaining_accounts
                            [new_address_params.address_merkle_tree_account_index as usize]
                            .key()
                            .to_bytes(),
                        invoking_program_id_bytes,
                    )
                }
                _ => {
                    return err!(
                        AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch
                    )
                }
            };
            // We are inserting addresses into two vectors to avoid unwrapping
            // the option in following functions.
            compressed_account_addresses[i + num_input_compressed_accounts] = Some(address);
            new_addresses.push(address);
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
