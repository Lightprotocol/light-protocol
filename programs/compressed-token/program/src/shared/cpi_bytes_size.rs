use anchor_lang::Discriminator;
use light_compressed_account::{
    compressed_account::{CompressedAccountConfig, CompressedAccountDataConfig},
    instruction_data::{
        data::OutputCompressedAccountWithPackedContextConfig,
        with_readonly::{
            InAccountConfig, InstructionDataInvokeCpiWithReadOnly,
            InstructionDataInvokeCpiWithReadOnlyConfig,
        },
    },
};
use light_ctoken_types::state::CompressedMint;
use light_program_profiler::profile;
use light_zero_copy::ZeroCopyNew;
use pinocchio::program_error::ProgramError;
use tinyvec::ArrayVec;

pub const MAX_INPUT_ACCOUNTS: usize = 8;
const MAX_OUTPUT_ACCOUNTS: usize = 35;

/// Calculate data length for a compressed mint account
#[profile]
#[inline(always)]
pub fn mint_data_len(config: &light_ctoken_types::state::CompressedMintConfig) -> u32 {
    CompressedMint::byte_len(config).unwrap() as u32
}

/// Calculate data length for a compressed token account
#[inline(always)]
pub fn compressed_token_data_len(has_delegate: bool) -> u32 {
    if has_delegate {
        107
    } else {
        75
    }
}

#[derive(Debug, Clone)]
pub struct CpiConfigInput {
    pub input_accounts: ArrayVec<[bool; MAX_INPUT_ACCOUNTS]>, // true = has address (mint), false = no address (token)
    pub output_accounts: ArrayVec<[(bool, u32); MAX_OUTPUT_ACCOUNTS]>, // (has_address, data_len)
    pub has_proof: bool,
    pub new_address_params: usize, // Number of new addresses to create
}

impl CpiConfigInput {
    /// Helper to create config for mint_to_compressed with no delegates
    #[profile]
    pub fn mint_to_compressed(
        num_recipients: usize,
        has_proof: bool,
        output_mint_config: &light_ctoken_types::state::CompressedMintConfig,
    ) -> Self {
        let mut outputs = ArrayVec::new();

        // First output is always the mint account
        outputs.push((true, mint_data_len(output_mint_config)));

        // Add token accounts for recipients
        for _ in 0..num_recipients {
            outputs.push((false, compressed_token_data_len(false))); // No delegates for simple mint
        }

        Self {
            input_accounts: ArrayVec::new(), // No input accounts for mint_to_compressed
            output_accounts: outputs,
            has_proof,
            new_address_params: 0, // No new addresses for mint_to_compressed
        }
    }

    /// Helper to create config for update_mint
    #[profile]
    pub fn update_mint(
        has_proof: bool,
        output_mint_config: &light_ctoken_types::state::CompressedMintConfig,
    ) -> Self {
        let mut inputs = ArrayVec::new();
        inputs.push(true); // Input mint has address

        let mut outputs = ArrayVec::new();
        outputs.push((true, mint_data_len(output_mint_config))); // Output mint has address

        Self {
            input_accounts: inputs,
            output_accounts: outputs,
            has_proof,
            new_address_params: 0, // No new addresses for update_mint
        }
    }
}

// TODO: generalize and move the light-compressed-account
// TODO: add version of this function with hardcoded values that just calculates the cpi_byte_size, with a randomized test vs this function
#[profile]
#[inline(always)]
pub fn cpi_bytes_config(input: CpiConfigInput) -> InstructionDataInvokeCpiWithReadOnlyConfig {
    let input_compressed_accounts = {
        let mut input_compressed_accounts = Vec::with_capacity(input.input_accounts.len());

        // Process input accounts in order
        for has_address in input.input_accounts {
            input_compressed_accounts.push(InAccountConfig {
                merkle_context: (),
                address: (has_address, ()),
            });
        }

        input_compressed_accounts
    };

    let output_compressed_accounts = {
        let mut outputs = Vec::with_capacity(input.output_accounts.len());
        // Process output accounts in order
        for (has_address, data_len) in input.output_accounts {
            outputs.push(OutputCompressedAccountWithPackedContextConfig {
                compressed_account: CompressedAccountConfig {
                    address: (has_address, ()),
                    data: (true, CompressedAccountDataConfig { data: data_len }),
                },
            });
        }
        outputs
    };
    let new_address_params = vec![(); input.new_address_params];
    InstructionDataInvokeCpiWithReadOnlyConfig {
        cpi_context: (),
        proof: (input.has_proof, ()),
        new_address_params, // Create required number of new address params
        input_compressed_accounts,
        output_compressed_accounts,
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    }
}

/// Allocate CPI instruction bytes with discriminator and length prefix
#[profile]
#[inline(always)]
pub fn allocate_invoke_with_read_only_cpi_bytes(
    config: &InstructionDataInvokeCpiWithReadOnlyConfig,
) -> Result<Vec<u8>, ProgramError> {
    let vec_len = InstructionDataInvokeCpiWithReadOnly::byte_len(config)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let mut cpi_bytes = vec![0u8; vec_len + 8];
    cpi_bytes[0..8]
        .copy_from_slice(light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR);
    Ok(cpi_bytes)
}
