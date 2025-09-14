#[cfg(test)]
mod test {
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn test_compressible_derive() {
        let input: syn::DeriveInput = parse_quote! {
            pub struct UserRecord {
                pub compression_info: Option<CompressionInfo>,
                pub owner: Pubkey,
                pub name: String,
                pub score: u64,
            }
        };

        let result = crate::compressible_derive::derive_compressible(input);
        assert!(result.is_ok(), "Compressible derive should succeed");

        let output = result.unwrap();
        let output_str = output.to_string();

        println!("Generated output:\n{}", output_str);

        // Check that all required trait implementations are generated
        assert!(output_str.contains("impl light_sdk :: compressible :: HasCompressionInfo"));
        assert!(output_str.contains("impl light_sdk :: account :: Size"));
        assert!(output_str.contains("impl light_sdk :: compressible :: CompressAs"));
        assert!(output_str.contains("compression_info : None"));
    }

    #[test]
    fn test_compressible_pack_derive() {
        let input: syn::DeriveInput = parse_quote! {
            pub struct UserRecord {
                pub compression_info: Option<CompressionInfo>,
                pub owner: Pubkey,
                pub name: String,
                pub score: u64,
            }
        };

        let result = crate::pack_unpack::derive_compressible_pack(input);
        assert!(result.is_ok(), "CompressiblePack derive should succeed");

        let output = result.unwrap();
        let output_str = output.to_string();

        println!("Pack derive output:\n{}", output_str);

        // Check that PackedUserRecord struct is generated
        assert!(output_str.contains("pub struct PackedUserRecord"));
        assert!(output_str.contains("pub owner : u8")); // Pubkey packed as u8
        assert!(output_str.contains("pub name : String")); // String kept as-is

        // Check that Pack/Unpack implementations are generated
        assert!(output_str.contains("impl light_sdk :: compressible :: Pack for UserRecord"));
        assert!(output_str.contains("impl light_sdk :: compressible :: Unpack for UserRecord"));
        assert!(output_str.contains("impl light_sdk :: compressible :: Pack for PackedUserRecord"));
        assert!(
            output_str.contains("impl light_sdk :: compressible :: Unpack for PackedUserRecord")
        );
    }

    #[test]
    fn test_compressed_account_variant_macro() {
        let input = quote! { UserRecord, GameSession };

        let result = crate::variant_enum::compressed_account_variant(input);
        assert!(
            result.is_ok(),
            "compressed_account_variant macro should succeed"
        );

        let output = result.unwrap();
        let output_str = output.to_string();

        println!("Variant enum output:\n{}", output_str);

        // Check that enum is generated with all variants
        assert!(output_str.contains("pub enum CompressedAccountVariant"));
        assert!(output_str.contains("UserRecord (UserRecord)"));
        assert!(output_str.contains("PackedUserRecord (PackedUserRecord)"));
        assert!(output_str.contains("GameSession (GameSession)"));
        assert!(output_str.contains("CompressibleTokenAccountPacked"));
        assert!(output_str.contains("CompressibleTokenData"));

        // Check that all trait implementations are generated
        assert!(output_str.contains("impl Default for CompressedAccountVariant"));
        assert!(output_str.contains("impl light_hasher :: DataHasher for CompressedAccountVariant"));
        assert!(output_str
            .contains("impl light_sdk :: LightDiscriminator for CompressedAccountVariant"));
        assert!(output_str.contains(
            "impl light_sdk :: compressible :: HasCompressionInfo for CompressedAccountVariant"
        ));
        assert!(
            output_str.contains("impl light_sdk :: account :: Size for CompressedAccountVariant")
        );
        assert!(output_str
            .contains("impl light_sdk :: compressible :: Pack for CompressedAccountVariant"));
        assert!(output_str
            .contains("impl light_sdk :: compressible :: Unpack for CompressedAccountVariant"));

        // Check that CompressedAccountData struct is generated
        assert!(output_str.contains("pub struct CompressedAccountData"));
    }

    #[test]
    fn test_custom_compression_with_compress_as_attribute() {
        let input: syn::DeriveInput = parse_quote! {
            #[compress_as(start_time = 0, score = 100)]
            pub struct GameSession {
                pub compression_info: Option<CompressionInfo>,
                pub session_id: u64,
                pub player: Pubkey,
                pub start_time: u64,
                pub score: u64,
            }
        };

        let result = crate::compressible_derive::derive_compressible(input);
        assert!(
            result.is_ok(),
            "Compressible derive with compress_as should succeed"
        );

        let output = result.unwrap();
        let output_str = output.to_string();

        println!("Custom compression output:\n{}", output_str);

        // Check that custom field values are used
        assert!(output_str.contains("start_time : 0"));
        assert!(output_str.contains("score : 100"));
        // Check that non-overridden fields use original values
        assert!(output_str.contains("session_id : self . session_id"));
        assert!(output_str.contains("player : self . player"));
    }

    #[test]
    fn test_derive_seeds_macro() {
        let input: syn::DeriveInput = parse_quote! {
            #[seeds("user_record", owner)]
            pub struct UserRecord {
                pub owner: Pubkey,
                pub name: String,
                pub score: u64,
            }
        };

        let result = crate::derive_seeds::derive_seeds(input);
        assert!(result.is_ok(), "DeriveSeeds should succeed");

        let output = result.unwrap();
        let output_str = output.to_string();

        println!("DeriveSeeds output:\n{}", output_str);

        // Check that function is generated
        assert!(output_str.contains("pub fn get_user_record_seeds"));
        assert!(output_str.contains("owner : & Pubkey"));
        assert!(output_str.contains("find_program_address"));
    }
}
