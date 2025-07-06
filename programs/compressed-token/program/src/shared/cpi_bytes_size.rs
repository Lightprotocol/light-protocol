use anchor_lang::Discriminator;
use arrayvec::ArrayVec;
use light_compressed_account::{
    compressed_account::{
        CompressedAccountConfig, CompressedAccountDataConfig, PackedMerkleContextConfig,
    },
    instruction_data::{
        compressed_proof::CompressedProofConfig,
        cpi_context::CompressedCpiContextConfig,
        data::OutputCompressedAccountWithPackedContextConfig,
        with_readonly::{
            InAccountConfig, InstructionDataInvokeCpiWithReadOnly,
            InstructionDataInvokeCpiWithReadOnlyConfig,
        },
    },
};
use light_zero_copy::ZeroCopyNew;

const MAX_INPUT_ACCOUNTS: usize = 8;
const MAX_OUTPUT_ACCOUNTS: usize = 35;

#[derive(Debug, Clone)]
pub struct CpiConfigInput {
    pub input_accounts: ArrayVec<bool, MAX_INPUT_ACCOUNTS>, // Per-input account delegate flag
    pub output_accounts: ArrayVec<bool, MAX_OUTPUT_ACCOUNTS>, // Per-output account delegate flag
    pub has_proof: bool,
    pub compressed_mint: bool,
    pub compressed_mint_with_freeze_authority: bool,
}

impl CpiConfigInput {
    /// Helper to create config for mint_to_compressed with no delegates
    pub fn mint_to_compressed(
        num_recipients: usize,
        has_compressed_mint: bool,
        compressed_mint_with_freeze_authority: bool,
    ) -> Self {
        let mut output_delegates = ArrayVec::new();
        for _ in 0..num_recipients {
            output_delegates.push(false); // No delegates for simple mint
        }

        Self {
            input_accounts: ArrayVec::new(), // No input accounts for mint_to_compressed
            output_accounts: output_delegates,
            has_proof: has_compressed_mint,
            compressed_mint: true,
            compressed_mint_with_freeze_authority,
        }
    }
}

// TODO: add version of this function with hardcoded values that just calculates the cpi_byte_size, with a randomized test vs this function
pub fn cpi_bytes_config(input: CpiConfigInput) -> InstructionDataInvokeCpiWithReadOnlyConfig {
    let input_compressed_accounts = {
        let mut inputs_capacity = input.input_accounts.len();
        if input.compressed_mint {
            inputs_capacity += 1;
        }
        let mut input_compressed_accounts = Vec::with_capacity(inputs_capacity);

        // Add regular input accounts (token accounts)
        for _ in input.input_accounts {
            input_compressed_accounts.push(InAccountConfig {
                merkle_context: PackedMerkleContextConfig {}, // Default merkle context
                address: (false, ()),                         // Token accounts don't have addresses
            });
        }

        // Add compressed mint input account if needed
        if input.compressed_mint {
            input_compressed_accounts.push(InAccountConfig {
                merkle_context: PackedMerkleContextConfig {}, // Default merkle context
                address: (true, ()),
            });
        }

        input_compressed_accounts
    };

    let output_compressed_accounts = {
        {
            let total_outputs = input.output_accounts.len() + if input.has_proof { 1 } else { 0 };
            let mut outputs = Vec::with_capacity(total_outputs);
            for has_delegate in input.output_accounts {
                let token_data_size = if has_delegate { 107 } else { 75 }; // 75 + 32 (delegate) = 107

                outputs.push(OutputCompressedAccountWithPackedContextConfig {
                    compressed_account: CompressedAccountConfig {
                        address: (false, ()), // Token accounts don't have addresses
                        data: (
                            true,
                            CompressedAccountDataConfig {
                                data: token_data_size, // Size depends on delegate: 75 without, 107 with
                            },
                        ),
                    },
                });
            }

            // Add compressed mint update if needed (last output account)
            if input.compressed_mint {
                use crate::mint::state::{CompressedMint, CompressedMintConfig};
                let mint_size_config = CompressedMintConfig {
                    mint_authority: (true, ()),
                    freeze_authority: (input.compressed_mint_with_freeze_authority, ()),
                };
                outputs.push(OutputCompressedAccountWithPackedContextConfig {
                    compressed_account: CompressedAccountConfig {
                        address: (true, ()), // Compressed mint has an address
                        data: (
                            true,
                            CompressedAccountDataConfig {
                                data: CompressedMint::byte_len(&mint_size_config) as u32,
                            },
                        ),
                    },
                });
            }
            outputs
        }
    };
    InstructionDataInvokeCpiWithReadOnlyConfig {
        cpi_context: CompressedCpiContextConfig {},
        proof: (input.has_proof, CompressedProofConfig {}),
        new_address_params: vec![], // No new addresses for mint_to_compressed
        input_compressed_accounts,
        output_compressed_accounts,
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    }
}

/// Allocate CPI instruction bytes with discriminator and length prefix
pub fn allocate_invoke_with_read_only_cpi_bytes(
    config: &InstructionDataInvokeCpiWithReadOnlyConfig,
) -> Vec<u8> {
    let vec_len = InstructionDataInvokeCpiWithReadOnly::byte_len(config);
    let mut cpi_bytes = vec![0u8; vec_len + 8];
    cpi_bytes[0..8]
        .copy_from_slice(light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR);
    cpi_bytes
}
