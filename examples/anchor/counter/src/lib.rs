use anchor_lang::{prelude::*, Discriminator};
use borsh::BorshDeserialize;
use light_sdk::{
    account::LightAccount,
    address::v1::derive_address,
    cpi::{
        accounts::CompressionCpiAccounts,
        verify::{verify_compression_instruction, CompressionInstruction},
    },
    instruction::{
        account_meta::CompressedAccountMeta, merkle_context::PackedAddressMerkleContext,
    },
    verifier::CompressedProof,
    LightDiscriminator, LightHasher, NewAddressParamsPacked, ValidityProof,
};

declare_id!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[program]
pub mod counter {

    use super::*;

    pub fn create_counter<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericAnchorAccounts<'info>>,
        proof: CompressedProof,
        address_merkle_context: PackedAddressMerkleContext,
        output_merkle_tree_index: u8,
    ) -> Result<()> {
        let program_id = crate::ID.into();
        // LightAccount::new_init will create an account with empty output state (no input state).
        // Modifying the account will modify the output state that when converted to_account_info()
        // is hashed with poseidon hashes, serialized with borsh
        // and created with verify_compression_instruction by invoking the light-system-program.
        // The hashing scheme is the account structure derived with LightHasher.
        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::ID,
        )
        .map_err(ProgramError::from)?;

        let (address, address_seed) = derive_address(
            &[b"counter", ctx.accounts.signer.key().as_ref()],
            &light_cpi_accounts.tree_accounts()
                [address_merkle_context.address_merkle_tree_pubkey_index as usize]
                .key(),
            &crate::ID,
        );

        let new_address_params = NewAddressParamsPacked {
            seed: address_seed,
            address_queue_account_index: address_merkle_context.address_queue_pubkey_index,
            address_merkle_tree_root_index: address_merkle_context.root_index,
            address_merkle_tree_account_index: address_merkle_context
                .address_merkle_tree_pubkey_index,
        };

        let mut counter = LightAccount::<'_, CounterAccount>::new_init(
            &program_id,
            Some(address),
            output_merkle_tree_index,
        );

        counter.owner = ctx.accounts.signer.key();
        counter.value = 0;

        let instruction = CompressionInstruction::new_with_address(
            proof,
            vec![counter.to_account_info().map_err(ProgramError::from)?],
            vec![new_address_params],
        );

        verify_compression_instruction(&light_cpi_accounts, instruction)
            .map_err(ProgramError::from)?;

        Ok(())
    }

    pub fn increment_counter<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericAnchorAccounts<'info>>,
        proof: ValidityProof,
        counter_value: u64,
        account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let program_id = crate::ID.into();
        // LightAccount::new_mut will create an account with input state and output state.
        // The input state is hashed immediately when calling new_mut().
        // Modifying the account will modify the output state that when converted to_account_info()
        // is hashed with poseidon hashes, serialized with borsh
        // and created with verify_compression_instruction by invoking the light-system-program.
        // The hashing scheme is the account structure derived with LightHasher.
        let mut counter = LightAccount::<'_, CounterAccount>::new_mut(
            &program_id,
            &account_meta,
            CounterAccount {
                owner: ctx.accounts.signer.key(),
                value: counter_value,
            },
        )
        .map_err(ProgramError::from)?;

        counter.value = counter.value.checked_add(1).ok_or(CustomError::Overflow)?;

        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::ID,
        )
        .map_err(ProgramError::from)?;

        let instruction = CompressionInstruction::new(
            proof,
            vec![counter.to_account_info().map_err(ProgramError::from)?],
        );

        verify_compression_instruction(&light_cpi_accounts, instruction)
            .map_err(ProgramError::from)?;

        Ok(())
    }

    pub fn decrement_counter<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericAnchorAccounts<'info>>,
        proof: ValidityProof,
        counter_value: u64,
        account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let program_id = crate::ID.into();
        let mut counter = LightAccount::<'_, CounterAccount>::new_mut(
            &program_id,
            &account_meta,
            CounterAccount {
                owner: ctx.accounts.signer.key(),
                value: counter_value,
            },
        )
        .map_err(ProgramError::from)?;

        if counter.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }

        counter.value = counter.value.checked_sub(1).ok_or(CustomError::Underflow)?;

        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::ID,
        )
        .map_err(ProgramError::from)?;

        let instruction = CompressionInstruction::new(
            proof,
            vec![counter.to_account_info().map_err(ProgramError::from)?],
        );

        verify_compression_instruction(&light_cpi_accounts, instruction)
            .map_err(ProgramError::from)?;

        Ok(())
    }

    pub fn reset_counter<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericAnchorAccounts<'info>>,
        proof: ValidityProof,
        counter_value: u64,
        account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let program_id = crate::ID.into();
        let mut counter = LightAccount::<'_, CounterAccount>::new_mut(
            &program_id,
            &account_meta,
            CounterAccount {
                owner: ctx.accounts.signer.key(),
                value: counter_value,
            },
        )
        .map_err(ProgramError::from)?;

        counter.value = 0;

        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::ID,
        )
        .map_err(ProgramError::from)?;
        let instruction = CompressionInstruction::new(
            proof,
            vec![counter.to_account_info().map_err(ProgramError::from)?],
        );

        verify_compression_instruction(&light_cpi_accounts, instruction)
            .map_err(ProgramError::from)?;

        Ok(())
    }

    pub fn close_counter<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericAnchorAccounts<'info>>,
        proof: ValidityProof,
        counter_value: u64,
        account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let program_id = crate::ID.into();
        // LightAccount::new_close() will create an account with only input state and no output state.
        // By providing no output state the account is closed after the instruction.
        // The address of a closed account cannot be reused.
        let counter = LightAccount::<'_, CounterAccount>::new_close(
            &program_id,
            &account_meta,
            CounterAccount {
                owner: ctx.accounts.signer.key(),
                value: counter_value,
            },
        )
        .map_err(ProgramError::from)?;

        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::ID,
        )
        .map_err(ProgramError::from)?;

        let instruction = CompressionInstruction::new(
            proof,
            vec![counter.to_account_info().map_err(ProgramError::from)?],
        );

        verify_compression_instruction(&light_cpi_accounts, instruction)
            .map_err(ProgramError::from)?;

        Ok(())
    }
}

#[error_code]
pub enum CustomError {
    #[msg("No authority to perform this action")]
    Unauthorized,
    #[msg("Counter overflow")]
    Overflow,
    #[msg("Counter underflow")]
    Underflow,
}

#[derive(Accounts)]
pub struct GenericAnchorAccounts<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(
    Clone, Debug, Default, AnchorDeserialize, AnchorSerialize, LightDiscriminator, LightHasher,
)]
pub struct CounterAccount {
    #[hash]
    pub owner: Pubkey,
    pub value: u64,
}
