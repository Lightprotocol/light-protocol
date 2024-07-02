use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;

use super::processor::CompressedProof;
use crate::{
    invoke::sol_compression::SOL_POOL_PDA_SEED,
    sdk::{
        accounts::{InvokeAccounts, SignerAccounts},
        compressed_account::{CompressedAccount, PackedCompressedAccountWithMerkleContext},
    },
};

/// These are the base accounts additionally Merkle tree and queue accounts are required.
/// These additional accounts are passed as remaining accounts.
/// 1 Merkle tree for each input compressed account one queue and Merkle tree account each for each output compressed account.
#[derive(Accounts)]
pub struct InvokeInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK: this account
    #[account(
    seeds = [&crate::ID.to_bytes()], bump, seeds::program = &account_compression::ID,
    )]
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: is checked when emitting the event.
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in account compression program.
    /// This pda is used to invoke the account compression program.
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: Account compression program is used to update state and address
    /// Merkle trees.
    pub account_compression_program: Program<'info, AccountCompression>,
    /// Sol pool pda is used to store the native sol that has been compressed.
    /// It's only required when compressing or decompressing sol.
    #[account(
        mut,
        seeds = [SOL_POOL_PDA_SEED], bump
    )]
    pub sol_pool_pda: Option<UncheckedAccount<'info>>,
    /// Only needs to be provided for decompression as a recipient for the
    /// decompressed sol.
    /// Compressed sol originate from authority.
    #[account(mut)]
    pub decompression_recipient: Option<UncheckedAccount<'info>>,
    pub system_program: Program<'info, System>,
}

impl<'info> SignerAccounts<'info> for InvokeInstruction<'info> {
    fn get_fee_payer(&self) -> &Signer<'info> {
        &self.fee_payer
    }

    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
}

impl<'info> InvokeAccounts<'info> for InvokeInstruction<'info> {
    fn get_registered_program_pda(&self) -> &AccountInfo<'info> {
        &self.registered_program_pda
    }

    fn get_noop_program(&self) -> &UncheckedAccount<'info> {
        &self.noop_program
    }

    fn get_account_compression_authority(&self) -> &UncheckedAccount<'info> {
        &self.account_compression_authority
    }

    fn get_account_compression_program(&self) -> &Program<'info, AccountCompression> {
        &self.account_compression_program
    }

    fn get_system_program(&self) -> &Program<'info, System> {
        &self.system_program
    }

    fn get_sol_pool_pda(&self) -> Option<&UncheckedAccount<'info>> {
        self.sol_pool_pda.as_ref()
    }

    fn get_decompression_recipient(&self) -> Option<&UncheckedAccount<'info>> {
        self.decompression_recipient.as_ref()
    }
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InstructionDataInvoke {
    pub proof: Option<CompressedProof>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct OutputCompressedAccountWithContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree: Pubkey,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct OutputCompressedAccountWithPackedContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree_index: u8,
}

#[derive(Debug, PartialEq, Default, Clone, Copy, AnchorSerialize, AnchorDeserialize)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct NewAddressParams {
    pub seed: [u8; 32],
    pub address_queue_pubkey: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}
