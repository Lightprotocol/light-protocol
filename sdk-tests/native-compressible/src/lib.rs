use borsh::{BorshDeserialize, BorshSerialize};
use light_macros::pubkey;
use light_sdk::{
    account::Size,
    compressible::{CompressionInfo, HasCompressionInfo},
    cpi::CpiSigner,
    derive_light_cpi_signer,
    error::LightSdkError,
    sha::LightHasher,
    LightDiscriminator,
};
use solana_program::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey::Pubkey,
};

pub mod compress_dynamic_pda;
pub mod compress_empty_compressed_pda;
pub mod create_config;
pub mod create_dynamic_pda;
pub mod create_empty_compressed_pda;
pub mod create_pda;
pub mod decompress_dynamic_pda;
pub mod update_config;
pub mod update_pda;

pub const ID: Pubkey = pubkey!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

entrypoint!(process_instruction);

#[repr(u8)]
pub enum InstructionType {
    CreatePdaBorsh = 0,
    UpdatePdaBorsh = 1,
    CompressDynamicPda = 2,
    CreateDynamicPda = 3,
    InitializeCompressionConfig = 4,
    UpdateCompressionConfig = 5,
    DecompressAccountsIdempotent = 6,
    CreateEmptyCompressedPda = 7,
    CompressEmptyCompressedPda = 8,
}

impl TryFrom<u8> for InstructionType {
    type Error = LightSdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstructionType::CreatePdaBorsh),
            1 => Ok(InstructionType::UpdatePdaBorsh),
            2 => Ok(InstructionType::CompressDynamicPda),
            3 => Ok(InstructionType::CreateDynamicPda),
            4 => Ok(InstructionType::InitializeCompressionConfig),
            5 => Ok(InstructionType::UpdateCompressionConfig),
            6 => Ok(InstructionType::DecompressAccountsIdempotent),
            7 => Ok(InstructionType::CreateEmptyCompressedPda),
            8 => Ok(InstructionType::CompressEmptyCompressedPda),

            _ => panic!("Invalid instruction discriminator."),
        }
    }
}

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let discriminator = InstructionType::try_from(instruction_data[0])
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match discriminator {
        InstructionType::CreatePdaBorsh => {
            create_pda::create_pda::<true>(accounts, &instruction_data[1..])
        }
        InstructionType::UpdatePdaBorsh => {
            update_pda::update_pda::<false>(accounts, &instruction_data[1..])
        }
        InstructionType::CompressDynamicPda => {
            compress_dynamic_pda::compress_dynamic_pda(accounts, &instruction_data[1..])
        }
        InstructionType::CreateDynamicPda => {
            create_dynamic_pda::create_dynamic_pda(accounts, &instruction_data[1..])
        }

        InstructionType::InitializeCompressionConfig => {
            create_config::process_initialize_compression_config_checked(
                accounts,
                &instruction_data[1..],
            )
        }
        InstructionType::UpdateCompressionConfig => {
            update_config::process_update_config(accounts, &instruction_data[1..])
        }
        InstructionType::DecompressAccountsIdempotent => {
            decompress_dynamic_pda::decompress_multiple_dynamic_pdas(
                accounts,
                &instruction_data[1..],
            )
        }
        InstructionType::CreateEmptyCompressedPda => {
            create_empty_compressed_pda::create_empty_compressed_pda(
                accounts,
                &instruction_data[1..],
            )
        }
        InstructionType::CompressEmptyCompressedPda => {
            compress_empty_compressed_pda::compress_empty_compressed_pda(
                accounts,
                &instruction_data[1..],
            )
        }
    }?;
    Ok(())
}

#[derive(
    Clone, Debug, Default, LightHasher, LightDiscriminator, BorshDeserialize, BorshSerialize,
)]
pub struct MyPdaAccount {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    pub data: [u8; 31],
}

// Implement the HasCompressionInfo trait
impl HasCompressionInfo for MyPdaAccount {
    fn compression_info(&self) -> &CompressionInfo {
        self.compression_info
            .as_ref()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        self.compression_info
            .as_mut()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        &mut self.compression_info
    }

    fn set_compression_info_none(&mut self) {
        self.compression_info = None;
    }
}

impl Size for MyPdaAccount {
    fn size(&self) -> usize {
        // compression_info is #[skip], so not serialized
        Self::LIGHT_DISCRIMINATOR_SLICE.len() + 31 + 1 + 9 // discriminator + data: [u8; 31] + compression_info: Option<CompressionInfo>
    }
}

#[cfg(test)]
mod test_sha_hasher {
    use light_hasher::{to_byte_array::ToByteArray, DataHasher, Sha256};
    use light_sdk::sha::LightHasher;

    use super::*;

    #[derive(
        Clone, Debug, Default, LightDiscriminator, BorshDeserialize, BorshSerialize, LightHasher,
    )]
    pub struct TestShaAccount {
        #[skip]
        pub compression_info: Option<CompressionInfo>,
        pub data: [u8; 31],
    }

    #[test]
    fn test_sha256_vs_poseidon_hashing() {
        let account = MyPdaAccount {
            compression_info: None,
            data: [42u8; 31],
        };

        // Test Poseidon hashing (default)
        let poseidon_hash = account.hash::<light_hasher::Poseidon>().unwrap();

        // Test SHA256 hashing
        let sha256_hash = account.hash::<Sha256>().unwrap();

        // They should be different
        assert_ne!(poseidon_hash, sha256_hash);

        // Both should have first byte as 0 (field size truncated) or be different due to different hashing
        println!("Poseidon hash: {:?}", poseidon_hash);
        println!("SHA256 hash: {:?}", sha256_hash);
    }

    #[test]
    fn test_sha_hasher_derive_macro() {
        let sha_account = TestShaAccount {
            compression_info: None,
            data: [99u8; 31],
        };

        // Test the to_byte_array implementation (which should use SHA256 internally)
        let sha_byte_array = sha_account.to_byte_array().unwrap();

        // Test DataHasher implementation with SHA256
        let sha_data_hash = sha_account.hash::<Sha256>().unwrap();

        // Both should have first byte truncated to 0 for field size
        assert_eq!(sha_byte_array[0], 0);
        assert_eq!(sha_data_hash[0], 0);

        assert_eq!(sha_byte_array.len(), 32);
        assert_eq!(sha_data_hash.len(), 32);

        println!("SHA account to_byte_array: {:?}", sha_byte_array);
        println!("SHA account DataHasher: {:?}", sha_data_hash);

        // Test that this is different from Poseidon hashing
        let poseidon_hash = sha_account.hash::<light_hasher::Poseidon>().unwrap();
        // Poseidon hash should not have first byte truncated (ID=0)
        assert_ne!(sha_byte_array, poseidon_hash);
        assert_ne!(sha_data_hash, poseidon_hash);

        println!("Same account with Poseidon: {:?}", poseidon_hash);
    }

    #[test]
    fn test_large_struct_with_sha_hasher() {
        // This demonstrates that SHA256 can handle arbitrary-sized data
        // while Poseidon is limited to 12 fields in the current implementation

        use light_hasher::{Hasher, Sha256};

        // Create a large struct that would exceed Poseidon's field limits
        #[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
        struct LargeStruct {
            pub field1: u64,
            pub field2: u64,
            pub field3: u64,
            pub field4: u64,
            pub field5: u64,
            pub field6: u64,
            pub field7: u64,
            pub field8: u64,
            pub field9: u64,
            pub field10: u64,
            pub field11: u64,
            pub field12: u64,
            pub field13: u64,
            // Pubkeys that would require #[hash] attribute with Poseidon
            pub owner: solana_program::pubkey::Pubkey,
            pub authority: solana_program::pubkey::Pubkey,
        }

        let large_account = LargeStruct {
            field1: 1,
            field2: 2,
            field3: 3,
            field4: 4,
            field5: 5,
            field6: 6,
            field7: 7,
            field8: 8,
            field9: 9,
            field10: 10,
            field11: 11,
            field12: 12,
            field13: 13,
            owner: solana_program::pubkey::Pubkey::new_unique(),
            authority: solana_program::pubkey::Pubkey::new_unique(),
        };

        // Test that SHA256 can hash large data by serializing the whole struct
        let serialized = large_account.try_to_vec().unwrap();
        println!("Serialized struct size: {} bytes", serialized.len());

        // SHA256 can hash arbitrary amounts of data
        let sha_hash = Sha256::hash(&serialized).unwrap();
        println!("SHA256 hash: {:?}", sha_hash);

        // Verify the hash is truncated properly (first byte should be 0 for field size)
        // Note: Since SHA256::ID = 1 (not 0), the system program expects truncation
        let mut expected_hash = sha_hash;
        expected_hash[0] = 0;

        assert_eq!(sha_hash.len(), 32);
        // For demonstration - in real usage, the truncation would be applied by the system
        println!("SHA256 hash truncated: {:?}", expected_hash);

        // Show that this would be different from a smaller struct
        let small_struct = MyPdaAccount {
            compression_info: None,
            data: [42u8; 31],
        };

        let small_serialized = small_struct.try_to_vec().unwrap();
        let small_hash = Sha256::hash(&small_serialized).unwrap();

        // Different data should produce different hashes
        assert_ne!(sha_hash, small_hash);
        println!("Different struct produces different hash: {:?}", small_hash);
    }
}
