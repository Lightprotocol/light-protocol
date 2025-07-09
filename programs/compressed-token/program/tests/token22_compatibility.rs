/*
 * Token 2022 Option Types Context:
 *
 * Token 2022 uses two different option types for storing optional pubkeys:
 *
 * 1. PodCOption<T> (used in PodMint for mint_authority, freeze_authority):
 *    - Memory Layout: [option: [u8; 4], value: T]
 *    - Some state: option = [1, 0, 0, 0] (4 bytes discriminant)
 *    - None state: option = [0, 0, 0, 0] (4 bytes discriminant)
 *    - Total size for Pubkey: 4 + 32 = 36 bytes
 *    - Can handle zero values (explicit discriminant)
 *
 * 2. OptionalNonZeroPubkey (used in MetadataPointer extension):
 *    - Memory Layout: Pubkey (32 bytes)
 *    - Some state: Any non-zero pubkey
 *    - None state: Pubkey::default() (all zeros)
 *    - Total size: 32 bytes
 *    - Cannot store zero pubkeys (they're interpreted as None)
 *
 * This explains size differences in serialization:
 * - PodCOption is larger (36 bytes) but more flexible
 * - OptionalNonZeroPubkey is smaller (32 bytes) but restricts zero values
 *
 * Token22 Complete Serialized Layout Analysis (234 bytes):
 *
 * Base PodMint (82 bytes):
 * [0-3]    mint_authority.option = [1,0,0,0] (SOME discriminant)
 * [4-35]   mint_authority.value = [0,0,0,3,...] (32-byte pubkey)
 * [36-39]  freeze_authority.option = [1,0,0,0] (SOME discriminant)
 * [40-71]  freeze_authority.value = [0,0,0,4,...] (32-byte pubkey)
 * [72-79]  supply = [64,66,15,0,0,0,0,0] (1000000 as little-endian u64)
 * [80]     decimals = 6
 * [81]     is_initialized = 1 (true)
 *
 * Account Type (1 byte):
 * [82]     account_type = 1 (AccountType::Mint)
 *
 * TLV Extension Header (6 bytes):
 * [83-84]  extension_type = [18,0] (ExtensionType::MetadataPointer as u16)
 * [85-88]  extension_length = [64,0,0,0] (64 bytes as u32)
 *
 * MetadataPointer Extension Data (64 bytes):
 * [89-120] metadata_authority = [0,0,0,1,...] (OptionalNonZeroPubkey - 32 bytes)
 * [121-152] metadata_address = [0,0,0,2,...] (OptionalNonZeroPubkey - 32 bytes)
 *
 * Remaining bytes [153-233] are padding/unused space in the allocated buffer
 *
 * TLV (Type-Length-Value) Deserialization Process:
 *
 * 1. Start after base mint data + account type (byte 83)
 * 2. Read extension_type (2 bytes): [18,0] = ExtensionType::MetadataPointer
 * 3. Read extension_length (4 bytes): [64,0,0,0] = 64 bytes of extension data
 * 4. Read extension_data (64 bytes): The actual MetadataPointer struct
 * 5. If more extensions exist, repeat from step 2 at next offset
 *
 * Extension Parsing Logic:
 * - Sequential parsing through TLV entries
 * - Each entry: [Type:u16][Length:u32][Data:variable]
 * - Type identifies the extension (MetadataPointer=18, TokenMetadata=19, etc.)
 * - Length specifies how many bytes to read for this extension
 * - Data contains the actual extension struct serialized as Pod bytes
 *
 * For MetadataPointer specifically:
 * - Type=18, Length=64, Data=2×OptionalNonZeroPubkey (32 bytes each)
 * - No internal discriminants in the extension data (unlike PodCOption)
 * - Uses zero-value encoding for None (all zeros = None pubkey)
 */

#[cfg(test)]
mod tests {
    use light_compressed_token::{
        extensions::metadata_pointer::MetadataPointer, mint::state::CompressedMint,
    };
    use solana_pubkey::Pubkey;
    use spl_pod::optional_keys::OptionalNonZeroPubkey;
    use spl_pod::primitives::{PodBool, PodU64};
    use spl_token_2022::extension::{
        metadata_pointer::MetadataPointer as Token22MetadataPointer, BaseStateWithExtensionsMut,
        ExtensionType, PodStateWithExtensionsMut,
    };
    use spl_token_2022::pod::{PodCOption, PodMint};

    /// CompressedMint struct that matches Token22 serialized layout
    #[derive(Debug, Clone)]
    #[repr(C)]
    pub struct CompressedMintToken22Layout {
        // Base mint data (matches PodMint layout)
        pub mint_authority: PodCOption<Pubkey>,   // 32 bytes
        pub supply: PodU64,                       // 8 bytes
        pub decimals: u8,                         // 1 byte
        pub is_initialized: PodBool,              // 1 byte
        pub freeze_authority: PodCOption<Pubkey>, // 32 bytes

        // Account type (1 byte)
        pub account_type: u8, // 1 byte = 75 bytes total so far

        // TLV Extensions
        pub extension_type: u16, // 2 bytes (ExtensionType::MetadataPointer = 18)
        pub extension_length: u32, // 4 bytes (64 bytes for MetadataPointer)

        // MetadataPointer extension data
        pub metadata_authority: OptionalNonZeroPubkey, // 32 bytes
        pub metadata_address: OptionalNonZeroPubkey,   // 32 bytes

                                                       // Fields from original CompressedMint that don't fit Token22 layout:
                                                       // - spl_mint: Pubkey (this becomes the account address, not stored in data)
                                                       // - is_decompressed: bool (compressed-specific, not in Token22)
                                                       // - version: u8 (compressed-specific versioning)
                                                       // - extension_hash: [u8; 32] (compressed-specific hash)
    }

    // #[test]
    // fn test_serialization_compatibility() {
    //     let authority = Pubkey::new_unique();
    //     let metadata_address = Pubkey::new_unique();
    //     let mint_authority = Pubkey::new_unique();
    //     let freeze_authority = Pubkey::new_unique();

    //     let compressed_metadata_pointer = MetadataPointer {
    //         authority: Some(authority.into()),
    //         metadata_address: Some(metadata_address.into()),
    //     };

    //     let token22_metadata_pointer = Token22MetadataPointer {
    //         authority: OptionalNonZeroPubkey::try_from(Some(authority)).unwrap(),
    //         metadata_address: OptionalNonZeroPubkey::try_from(Some(metadata_address)).unwrap(),
    //     };

    //     let compressed_mint = CompressedMint {
    //         spl_mint: mint_authority.into(),
    //         supply: 1000000,
    //         decimals: 6,
    //         is_decompressed: false,
    //         mint_authority: Some(mint_authority.into()),
    //         freeze_authority: None,
    //         version: 0,
    //         extension_hash: [0; 32],
    //     };

    //     // Create Token22 mint account with metadata pointer extension
    //     let account_size =
    //         ExtensionType::try_calculate_account_len::<PodMint>(&[ExtensionType::MetadataPointer])
    //             .unwrap();
    //     let mut token22_account_data = vec![0u8; account_size];

    //     // Unpack uninitialized buffer
    //     let mut token22_state =
    //         PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut token22_account_data)
    //             .unwrap();

    //     // Initialize base mint data
    //     *token22_state.base = PodMint {
    //         mint_authority: PodCOption::some(mint_authority.into()),
    //         supply: PodU64::from_primitive(1000000),
    //         decimals: 6,
    //         is_initialized: PodBool::from_bool(true),
    //         freeze_authority: PodCOption::some(freeze_authority.into()),
    //     };

    //     // Initialize account type
    //     token22_state.init_account_type().unwrap();

    //     // Initialize metadata pointer extension
    //     let metadata_pointer_ext = token22_state
    //         .init_extension::<Token22MetadataPointer>(false)
    //         .unwrap();
    //     *metadata_pointer_ext = token22_metadata_pointer;

    //     let compressed_mint_serialized = borsh::to_vec(&compressed_mint).unwrap();
    //     let token22_complete_serialized = token22_account_data.clone();

    //     // Create CompressedMint with Token22 layout
    //     let compressed_mint_token22_layout = CompressedMintToken22Layout {
    //         mint_authority: PodCOption::some(mint_authority.into()),
    //         supply: PodU64::from_primitive(1000000),
    //         decimals: 6,
    //         is_initialized: PodBool::from_bool(true),
    //         freeze_authority: PodCOption::some(freeze_authority.into()),
    //         account_type: spl_token_2022::extension::AccountType::Mint as u8,
    //         extension_type: ExtensionType::MetadataPointer as u16,
    //         extension_length: 64u32, // size of MetadataPointer
    //         metadata_authority: OptionalNonZeroPubkey::try_from(Some(authority)).unwrap(),
    //         metadata_address: OptionalNonZeroPubkey::try_from(Some(metadata_address)).unwrap(),
    //     };

    //     // Token22 mint serialization: [Base Mint: 82 bytes][Account Type: 1 byte][TLV Extensions...]
    //     // TLV: [Type: 2 bytes][Length: 4 bytes][MetadataPointer: 64 bytes]
    //     println!(
    //         "CompressedMint size: {} bytes",
    //         compressed_mint_serialized.len()
    //     );
    //     println!(
    //         "Token22 complete size: {} bytes",
    //         token22_complete_serialized.len()
    //     );
    //     println!(
    //         "CompressedMintToken22Layout size: {} bytes",
    //         std::mem::size_of::<CompressedMintToken22Layout>()
    //     );
    //     println!("CompressedMint bytes: {:?}", compressed_mint_serialized);
    //     println!("Token22 complete bytes: {:?}", token22_complete_serialized);

    //     // Show the layout struct size matches expected Token22 size
    //     let expected_size = 32 + 8 + 1 + 1 + 32 + 1 + 2 + 4 + 32 + 32; // 145 bytes
    //     println!("Expected Token22 layout size: {} bytes", expected_size);
    //     println!(
    //         "Actual CompressedMintToken22Layout size: {} bytes",
    //         std::mem::size_of::<CompressedMintToken22Layout>()
    //     );
    // }
}
