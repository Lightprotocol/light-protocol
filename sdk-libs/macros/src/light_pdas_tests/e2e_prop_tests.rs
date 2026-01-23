//! End-to-end property-based tests for derive_light_accounts macro.
//!
//! These tests verify correctness properties of the full macro pipeline:
//! - Never panics on syntactically valid input
//! - Output contains expected trait implementations
//! - Deterministic code generation

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use syn::{parse_quote, DeriveInput};

    // Access derive module from parent (accounts module)
    use crate::light_pdas::accounts::derive::derive_light_accounts;

    // ========================================================================
    // Constants
    // ========================================================================

    /// Rust keywords that are capitalized and could match PascalCase patterns.
    /// These should be excluded from struct/type name generation.
    const RUST_TYPE_KEYWORDS: &[&str] = &["Self"];

    // ========================================================================
    // Strategies for generating test inputs
    // ========================================================================

    /// Strategy for generating struct names (PascalCase)
    /// Excludes Rust keywords like "Self" that would fail parsing.
    fn arb_struct_name() -> impl Strategy<Value = String> {
        "[A-Z][a-z]{2,10}".prop_filter("not a Rust keyword", |s| {
            !RUST_TYPE_KEYWORDS.contains(&s.as_str())
        })
    }

    /// Strategy for generating field names
    fn arb_field_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,10}"
    }

    /// Strategy for generating param type names (PascalCase)
    /// Excludes Rust keywords like "Self" that would fail parsing.
    fn arb_type_name() -> impl Strategy<Value = String> {
        "[A-Z][a-z]{2,10}Params".prop_filter("not a Rust keyword", |s| !s.starts_with("Self"))
    }

    // ========================================================================
    // Property Tests: Basic Macro Behavior
    // ========================================================================

    proptest! {
        /// Empty struct without instruction should not panic and generate noop impls.
        #[test]
        fn prop_empty_struct_no_panic(struct_name in arb_struct_name()) {
            let input: DeriveInput = syn::parse_str(&format!(
                "pub struct {}<'info> {{ pub fee_payer: Signer<'info> }}",
                struct_name
            )).unwrap();

            let result = derive_light_accounts(&input);
            prop_assert!(
                result.is_ok(),
                "Empty struct '{}' should not cause macro to panic",
                struct_name
            );
        }

        /// Struct with instruction attribute should generate non-noop impls.
        #[test]
        fn prop_with_instruction_generates_impls(
            struct_name in arb_struct_name(),
            param_type in arb_type_name()
        ) {
            let input: DeriveInput = syn::parse_str(&format!(
                r#"#[instruction(params: {})]
                pub struct {}<'info> {{
                    pub fee_payer: Signer<'info>
                }}"#,
                param_type, struct_name
            )).unwrap();

            let result = derive_light_accounts(&input);
            prop_assert!(
                result.is_ok(),
                "Struct '{}' with instruction should generate impls",
                struct_name
            );

            let output = result.unwrap().to_string();
            prop_assert!(
                output.contains("LightPreInit"),
                "Output should contain LightPreInit trait impl"
            );
            prop_assert!(
                output.contains("LightFinalize"),
                "Output should contain LightFinalize trait impl"
            );
        }

        /// derive_light_accounts should be deterministic.
        #[test]
        fn prop_deterministic(struct_name in arb_struct_name()) {
            let input: DeriveInput = syn::parse_str(&format!(
                "pub struct {}<'info> {{ pub fee_payer: Signer<'info> }}",
                struct_name
            )).unwrap();

            let result1 = derive_light_accounts(&input);
            let result2 = derive_light_accounts(&input);

            prop_assert_eq!(
                result1.is_ok(),
                result2.is_ok(),
                "Macro should consistently succeed or fail"
            );

            if let (Ok(output1), Ok(output2)) = (result1, result2) {
                prop_assert_eq!(
                    output1.to_string(),
                    output2.to_string(),
                    "Macro output should be deterministic"
                );
            }
        }

        /// Without instruction attribute, should generate noop impls.
        #[test]
        fn prop_without_instruction_noop(struct_name in arb_struct_name()) {
            let input: DeriveInput = syn::parse_str(&format!(
                "pub struct {}<'info> {{ pub fee_payer: Signer<'info> }}",
                struct_name
            )).unwrap();

            let result = derive_light_accounts(&input);
            if let Ok(output) = result {
                let output_str = output.to_string();
                // Noop impls have Ok(false) for pre_init
                prop_assert!(
                    output_str.contains("Ok (false)") || output_str.contains("Ok(false)"),
                    "Without instruction, pre_init should return Ok(false)"
                );
            }
        }
    }

    // ========================================================================
    // Property Tests: Light Account Field Parsing
    // ========================================================================

    proptest! {
        /// Struct with light_account(init) field should generate PDA code.
        /// Uses parse_quote for more reliable struct generation.
        #[test]
        fn prop_light_account_init_generates_code(
            struct_name in arb_struct_name(),
            _field_name in arb_field_name(),
            _param_type in arb_type_name()
        ) {
            // Use parse_quote with fixed structure - property test varies struct name only
            // to avoid complex string formatting issues.
            // Includes required infrastructure fields: fee_payer, compression_config
            let struct_ident = syn::Ident::new(&struct_name, proc_macro2::Span::call_site());
            let input: DeriveInput = parse_quote! {
                #[instruction(params: TestParams)]
                pub struct #struct_ident<'info> {
                    #[account(mut)]
                    pub fee_payer: Signer<'info>,
                    #[account(
                        init,
                        payer = fee_payer,
                        space = 8 + 100,
                        seeds = [b"test"],
                        bump
                    )]
                    #[light_account(init)]
                    pub user_record: Account<'info, TestRecord>,
                    // Required infrastructure field for PDA fields
                    pub compression_config: Account<'info, CompressionConfig>
                }
            };

            let result = derive_light_accounts(&input);
            prop_assert!(
                result.is_ok(),
                "Struct with light_account(init) should parse successfully: {:?}",
                result.err()
            );

            let output = result.unwrap().to_string();
            // Should generate pre_init code for PDA
            prop_assert!(
                output.contains("LightPreInit"),
                "Should generate LightPreInit impl"
            );
        }

        /// Token account with init should generate CreateTokenAccountCpi.
        #[test]
        fn prop_token_account_generates_cpi(
            _struct_name in arb_struct_name(),
            _param_type in arb_type_name()
        ) {
            // Use parse_quote which is more reliable for complex structs
            let input: DeriveInput = parse_quote! {
                #[instruction(params: CreateParams)]
                pub struct TestStruct<'info> {
                    #[account(mut)]
                    pub fee_payer: Signer<'info>,

                    #[light_account(init, token::authority = [b"authority"], token::mint = my_mint, token::owner = fee_payer)]
                    pub vault: Account<'info, CToken>,

                    pub light_token_compressible_config: Account<'info, CompressibleConfig>,
                    pub light_token_rent_sponsor: Account<'info, RentSponsor>,
                    pub light_token_cpi_authority: AccountInfo<'info>,
                }
            };

            let result = derive_light_accounts(&input);
            prop_assert!(
                result.is_ok(),
                "Token account struct should parse successfully"
            );

            let output = result.unwrap().to_string();
            prop_assert!(
                output.contains("CreateTokenAccountCpi"),
                "Token account with init should generate CreateTokenAccountCpi"
            );
        }

        /// ATA with init should generate CreateTokenAtaCpi.
        #[test]
        fn prop_ata_generates_cpi(
            _struct_name in arb_struct_name(),
            _param_type in arb_type_name()
        ) {
            let input: DeriveInput = parse_quote! {
                #[instruction(params: CreateParams)]
                pub struct TestAta<'info> {
                    #[account(mut)]
                    pub fee_payer: Signer<'info>,

                    #[light_account(init, associated_token::authority = wallet, associated_token::mint = my_mint)]
                    pub user_ata: Account<'info, CToken>,

                    pub wallet: AccountInfo<'info>,
                    pub my_mint: AccountInfo<'info>,
                    pub light_token_compressible_config: Account<'info, CompressibleConfig>,
                    pub light_token_rent_sponsor: Account<'info, RentSponsor>,
                }
            };

            let result = derive_light_accounts(&input);
            prop_assert!(
                result.is_ok(),
                "ATA struct should parse successfully"
            );

            let output = result.unwrap().to_string();
            prop_assert!(
                output.contains("CreateTokenAtaCpi"),
                "ATA with init should generate CreateTokenAtaCpi"
            );
        }
    }

    // ========================================================================
    // Property Tests: Error Handling
    // ========================================================================

    proptest! {
        /// light_account without instruction attribute should fail.
        #[test]
        fn prop_light_account_requires_instruction(
            struct_name in arb_struct_name(),
            field_name in arb_field_name()
        ) {
            let input_str = format!(
                r#"pub struct {}<'info> {{
                    #[account(mut)]
                    pub fee_payer: Signer<'info>,
                    #[account(
                        init,
                        payer = fee_payer,
                        space = 8 + 100,
                        seeds = [b"test"],
                        bump
                    )]
                    #[light_account(init)]
                    pub {}: Account<'info, TestRecord>
                }}"#,
                struct_name, field_name
            );

            if let Ok(input) = syn::parse_str::<DeriveInput>(&input_str) {
                let result = derive_light_accounts(&input);
                // Should fail because light_account fields require instruction attribute
                prop_assert!(
                    result.is_err(),
                    "light_account without instruction should fail"
                );
            }
        }

        /// Invalid struct (not a struct) should fail gracefully.
        #[test]
        fn prop_non_struct_fails_gracefully(_seed in 0u32..1000) {
            // Try to derive on an enum (should fail)
            let input: DeriveInput = parse_quote! {
                pub enum NotAStruct {
                    VariantA,
                    VariantB,
                }
            };

            let result = derive_light_accounts(&input);
            prop_assert!(
                result.is_err(),
                "Enum should fail gracefully"
            );
        }
    }

    // ========================================================================
    // Property Tests: Output Structure
    // ========================================================================

    proptest! {
        /// Output should always contain both trait implementations.
        #[test]
        fn prop_always_produces_both_traits(struct_name in arb_struct_name()) {
            let input: DeriveInput = syn::parse_str(&format!(
                "pub struct {}<'info> {{ pub fee_payer: Signer<'info> }}",
                struct_name
            )).unwrap();

            let result = derive_light_accounts(&input);
            if let Ok(output) = result {
                let output_str = output.to_string();
                prop_assert!(
                    output_str.contains("LightPreInit"),
                    "Output should always contain LightPreInit"
                );
                prop_assert!(
                    output_str.contains("LightFinalize"),
                    "Output should always contain LightFinalize"
                );
            }
        }

        /// Generated code should compile as valid Rust tokens.
        #[test]
        fn prop_output_is_valid_tokens(struct_name in arb_struct_name()) {
            let input: DeriveInput = syn::parse_str(&format!(
                "pub struct {}<'info> {{ pub fee_payer: Signer<'info> }}",
                struct_name
            )).unwrap();

            let result = derive_light_accounts(&input);
            if let Ok(output) = result {
                // The output should be parseable as valid token stream
                // (it already is a TokenStream, so this is a sanity check)
                let output_str = output.to_string();
                prop_assert!(
                    !output_str.is_empty(),
                    "Output should not be empty"
                );
                // Check it's balanced braces (basic syntax check)
                let open_braces = output_str.matches('{').count();
                let close_braces = output_str.matches('}').count();
                prop_assert_eq!(
                    open_braces, close_braces,
                    "Braces should be balanced in output"
                );
            }
        }
    }
}
