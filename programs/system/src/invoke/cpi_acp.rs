use account_compression::{
    append_nullify_create_address::AppendNullifyCreateAddressInputs,
    utils::{constants::CPI_AUTHORITY_PDA_SEED, transfer_lamports::transfer_lamports_cpi},
};
use anchor_lang::{prelude::*, Bumps};

use crate::{
    constants::CPI_AUTHORITY_PDA_BUMP,
    sdk::accounts::{InvokeAccounts, SignerAccounts},
};

// TODO:
// 1. only one iteration per inputs, addresses, read-only, and outputs.
// -> do all the checks in one place and collect data in bytes for cpi.
pub struct CpiData<'info> {
    pub account_indices: Vec<u8>,
    pub accounts: Vec<AccountMeta>,
    // Would be better to store references.
    pub account_infos: Vec<AccountInfo<'info>>,
    // TODO: switch to store account indices once we have new context.
    pub hashed_pubkeys: Vec<(Pubkey, [u8; 32])>,
    // Addresses for deduplication.
    // Try to find a way without storing the addresses.
    pub addresses: Vec<Option<[u8; 32]>>,
    // Index of account and fee to be paid.
    pub rollover_fee_payments: Vec<(u8, u64)>,
}

// TODO: remove event to expose all output account data we need to copy the
// entire accounts including vectors.
// we can probably just put this data at the end of the vec so that
// we can easily skip it the account compression program.
// Maybe I can even just get the pointer to the data
// and put that into a separate cpi.
pub fn create_cpi_data<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    num_leaves: u8,
    num_nullifiers: u8,
    num_new_addresses: u8,
    hashed_pubkeys_capacity: usize,
) -> Result<(CpiData<'info>, Vec<u8>)> {
    let account_infos = vec![
        ctx.accounts.get_fee_payer().to_account_info(),
        ctx.accounts
            .get_account_compression_authority()
            .to_account_info(),
        ctx.accounts.get_registered_program_pda().to_account_info(),
        ctx.accounts.get_system_program().to_account_info(),
    ];
    let accounts = vec![
        AccountMeta {
            pubkey: account_infos[0].key(),
            is_signer: true,
            is_writable: true,
        },
        AccountMeta::new_readonly(account_infos[1].key(), true),
        AccountMeta::new_readonly(account_infos[2].key(), false),
        AccountMeta::new_readonly(account_infos[3].key(), false),
    ];
    let account_indices =
        Vec::<u8>::with_capacity((num_nullifiers + num_leaves + num_new_addresses) as usize);
    let bytes_size = AppendNullifyCreateAddressInputs::required_size_for_capacity(
        num_leaves,
        num_nullifiers,
        num_new_addresses,
    );
    msg!("num_leaves: {}", num_leaves);
    msg!("num_nullifiers: {}", num_nullifiers);
    msg!("num_new_addresses: {}", num_new_addresses);
    msg!("bytes_size: {}", bytes_size);
    let bytes = vec![0u8; bytes_size];
    Ok((
        CpiData {
            account_indices,
            accounts,
            account_infos,
            hashed_pubkeys: Vec::with_capacity(hashed_pubkeys_capacity),
            // TODO: init with capacity.
            addresses: Vec::new(),
            rollover_fee_payments: Vec::new(),
        },
        bytes,
    ))
}
impl<'info> CpiData<'info> {
    pub fn get_index_or_insert(
        &mut self,
        ix_data_index: u8,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> u8 {
        msg!("ix_data_index: {}", ix_data_index);
        msg!("self.account_indices: {:?}", self.account_indices);
        let queue_index = self
            .account_indices
            .iter()
            .position(|a| *a == ix_data_index);
        msg!("queue_index: {:?}", queue_index);
        let queue_index = match queue_index {
            Some(index) => index as u8,
            None => {
                msg!("pushing to account_indices");
                msg!(
                    "key: {:?}",
                    remaining_accounts[ix_data_index as usize].key()
                );
                self.account_indices.push(ix_data_index);
                let account_info = &remaining_accounts[ix_data_index as usize];
                self.accounts.push(AccountMeta {
                    pubkey: account_info.key(),
                    is_signer: false,
                    is_writable: true,
                });
                self.account_infos.push(account_info.clone());
                self.account_indices.len() as u8 - 1
            }
        };
        queue_index
    }

    pub fn set_rollover_fee(&mut self, ix_data_index: u8, fee: u64) {
        let payment = self
            .rollover_fee_payments
            .iter_mut()
            .find(|a| a.0 == ix_data_index);
        match payment {
            Some(payment) => payment.1 += fee,
            None => self.rollover_fee_payments.push((ix_data_index, fee)),
        };
    }

    pub fn transfer_rollover_fees(
        &self,
        accounts: &[AccountInfo<'info>],
        fee_payer: &AccountInfo<'info>,
    ) -> Result<()> {
        // TODO: if len is 1 don't do a cpi mutate lamports.
        for (i, fee) in self.rollover_fee_payments.iter() {
            transfer_lamports_cpi(fee_payer, &accounts[*i as usize], *fee)?;
        }
        Ok(())
    }
}
use anchor_lang::{InstructionData, Result};

pub fn cpi_account_compression_program(cpi_context: CpiData, bytes: Vec<u8>) -> Result<()> {
    let CpiData {
        accounts,
        account_infos,
        ..
    } = cpi_context;
    let instruction_data = account_compression::instruction::NullifyAppendCreateAddress { bytes };

    let data = instruction_data.data();
    light_heap::bench_sbf_end!("cpda_instruction_data");
    let bump = &[CPI_AUTHORITY_PDA_BUMP];
    let seeds = &[&[CPI_AUTHORITY_PDA_SEED, bump][..]];
    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: account_compression::ID,
        accounts,
        data,
    };
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        account_infos.as_slice(),
        seeds,
    )?;
    Ok(())
}
