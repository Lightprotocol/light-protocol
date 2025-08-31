use light_compressed_account::{
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        traits::{AccountOptions, InputAccount, InstructionData, NewAddress, OutputAccount},
        zero_copy::{ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount},
    },
    pubkey::Pubkey,
    CompressedAccountError,
};
use zerocopy::Ref;

use super::state::ZCpiContextAccount2;

impl<'a> InstructionData<'a> for ZCpiContextAccount2<'a> {
    fn owner(&self) -> Pubkey {
        // CPI context accounts don't have a single owner, they aggregate multiple programs
        // Return the fee payer as the primary owner
        *self.fee_payer
    }

    fn new_addresses(&self) -> &[impl NewAddress<'a>] {
        self.new_addresses.as_slice()
    }

    fn input_accounts(&self) -> &[impl InputAccount<'a>] {
        self.in_accounts.as_slice()
    }

    fn output_accounts(&self) -> &[impl OutputAccount<'a>] {
        self.out_accounts.as_slice()
    }

    fn read_only_accounts(&self) -> Option<&[ZPackedReadOnlyCompressedAccount]> {
        if self.readonly_accounts.is_empty() {
            None
        } else {
            Some(self.readonly_accounts.as_slice())
        }
    }

    fn read_only_addresses(&self) -> Option<&[ZPackedReadOnlyAddress]> {
        if self.readonly_addresses.is_empty() {
            None
        } else {
            Some(self.readonly_addresses.as_slice())
        }
    }

    fn is_compress(&self) -> bool {
        false
    }

    fn compress_or_decompress_lamports(&self) -> Option<u64> {
        // CPI context accounts don't directly handle lamport compression/decompression
        // This is handled by individual instructions within the context
        None
    }

    fn proof(&self) -> Option<Ref<&'a [u8], CompressedProof>> {
        // CPI context accounts don't contain proofs directly
        // Proofs are provided by the instructions that use the context
        None
    }

    fn cpi_context(&self) -> Option<CompressedCpiContext> {
        None
    }

    fn bump(&self) -> Option<u8> {
        // CPI context accounts don't have a PDA bump
        None
    }

    fn account_option_config(&self) -> Result<AccountOptions, CompressedAccountError> {
        Ok(AccountOptions {
            sol_pool_pda: false,
            decompression_recipient: false,
            cpi_context_account: true,
            write_to_cpi_context: true,
        })
    }

    fn with_transaction_hash(&self) -> bool {
        // CPI context accounts typically don't require transaction hashes
        false
    }
}
