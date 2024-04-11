use anchor_lang::prelude::*;

use crate::{address_queue_from_bytes_zero_copy_init, state::AddressQueueAccount};

#[derive(Accounts)]
pub struct InitializeAddressQueue<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub queue: AccountLoader<'info, AddressQueueAccount>,
}

pub fn process_initialize_address_queue<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeAddressQueue<'info>>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    associated_merkle_tree: Option<Pubkey>,
    capacity_indices: u16,
    capacity_values: u16,
    sequence_threshold: u64,
) -> Result<()> {
    {
        let mut address_queue_account = ctx.accounts.queue.load_init()?;
        address_queue_account.index = index;
        address_queue_account.owner = owner;
        address_queue_account.delegate = delegate.unwrap_or(owner);
        address_queue_account.associated_merkle_tree = associated_merkle_tree.unwrap_or_default();
        drop(address_queue_account);
    }

    let _ = unsafe {
        address_queue_from_bytes_zero_copy_init(
            &mut ctx.accounts.queue.to_account_info().try_borrow_mut_data()?,
            capacity_indices as usize,
            capacity_values as usize,
            sequence_threshold as usize,
        )
        .unwrap()
    };

    Ok(())
}

#[cfg(not(target_os = "solana"))]
pub mod initialize_address_queue_sdk {
    use anchor_lang::{system_program, InstructionData};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };

    pub fn create_initialize_address_queue_instruction(
        payer: Pubkey,
        address_queue_pubkey: Pubkey,
        index: u64,
        associated_merkle_tree: Option<Pubkey>,
        capacity_indices: u16,
        capacity_values: u16,
        sequence_threshold: u64,
    ) -> Instruction {
        let instruction_data = crate::instruction::InitializeAddressQueue {
            index,
            owner: payer,
            delegate: None,
            associated_merkle_tree,
            capacity_indices,
            capacity_values,
            sequence_threshold,
        };
        Instruction {
            program_id: crate::ID,
            accounts: vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(address_queue_pubkey, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction_data.data(),
        }
    }
}
