use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::{
    prelude::*, solana_program::pubkey::Pubkey, system_program::System, AnchorDeserialize,
    AnchorSerialize,
};

use super::account::CpiContextAccount;
use crate::{
    invoke::{processor::CompressedProof, sol_compression::SOL_POOL_PDA_SEED},
    sdk::{
        accounts::{InvokeAccounts, SignerAccounts},
        compressed_account::PackedCompressedAccountWithMerkleContext,
        CompressedCpiContext,
    },
    NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
};

#[derive(Accounts)]
pub struct InvokeCpiInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(
    seeds = [&crate::ID.to_bytes()], bump, seeds::program = &account_compression::ID,
    )]
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: checked in emit_event.rs.
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK:
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub account_compression_authority: UncheckedAccount<'info>,
    /// CHECK:
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: checked in cpi_signer_check.
    pub invoking_program: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [SOL_POOL_PDA_SEED], bump
    )]
    pub sol_pool_pda: Option<UncheckedAccount<'info>>,
    #[account(mut)]
    pub decompression_recipient: Option<UncheckedAccount<'info>>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub cpi_context_account: Option<Account<'info, CpiContextAccount>>,
}

impl<'info> SignerAccounts<'info> for InvokeCpiInstruction<'info> {
    fn get_fee_payer(&self) -> &Signer<'info> {
        &self.fee_payer
    }

    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
}

impl<'info> InvokeAccounts<'info> for InvokeCpiInstruction<'info> {
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

    fn get_sol_pool_pda(&self) -> Option<&UncheckedAccount<'info>> {
        self.sol_pool_pda.as_ref()
    }

    fn get_decompression_recipient(&self) -> Option<&UncheckedAccount<'info>> {
        self.decompression_recipient.as_ref()
    }

    fn get_system_program(&self) -> &Program<'info, System> {
        &self.system_program
    }
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InstructionDataInvokeCpi {
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub signer_seeds: Vec<Vec<u8>>,
    pub cpi_context: Option<CompressedCpiContext>,
}

impl InstructionDataInvokeCpi {
    pub fn combine(&mut self, other: &[InstructionDataInvokeCpi]) {
        for other in other {
            self.new_address_params
                .extend_from_slice(&other.new_address_params);
            self.input_compressed_accounts_with_merkle_context
                .extend_from_slice(&other.input_compressed_accounts_with_merkle_context);
            self.output_compressed_accounts
                .extend_from_slice(&other.output_compressed_accounts);
        }
    }
}
#[cfg(test)]
mod tests {
    use std::vec;

    use crate::{
        invoke::processor::CompressedProof,
        sdk::compressed_account::PackedCompressedAccountWithMerkleContext,
        InstructionDataInvokeCpi, NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
    };

    // test combine instruction data transfer
    #[test]
    fn test_combine_instruction_data_transfer() {
        let mut instruction_data_transfer = InstructionDataInvokeCpi {
            proof: Some(CompressedProof {
                a: [0; 32],
                b: [0; 64],
                c: [0; 32],
            }),
            new_address_params: vec![NewAddressParamsPacked::default()],
            input_compressed_accounts_with_merkle_context: vec![
                PackedCompressedAccountWithMerkleContext::default(),
            ],
            output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext::default()],
            relay_fee: Some(1),
            compress_or_decompress_lamports: Some(1),
            is_compress: true,
            signer_seeds: vec![vec![0; 32], vec![1; 32]],
            cpi_context: None,
        };
        let other = InstructionDataInvokeCpi {
            proof: Some(CompressedProof {
                a: [0; 32],
                b: [0; 64],
                c: [0; 32],
            }),
            input_compressed_accounts_with_merkle_context: vec![
                PackedCompressedAccountWithMerkleContext::default(),
            ],
            output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext::default()],
            relay_fee: Some(1),
            compress_or_decompress_lamports: Some(1),
            is_compress: true,
            new_address_params: vec![NewAddressParamsPacked::default()],
            signer_seeds: vec![],
            cpi_context: None,
        };
        instruction_data_transfer.combine(&[other]);
        assert_eq!(instruction_data_transfer.new_address_params.len(), 2);
        assert_eq!(
            instruction_data_transfer
                .input_compressed_accounts_with_merkle_context
                .len(),
            2
        );
        assert_eq!(
            instruction_data_transfer.output_compressed_accounts.len(),
            2
        );
    }
}
