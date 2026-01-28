//! LightAccount derive macro - generates unified LightAccount trait implementation.
//!
//! This macro generates:
//! - `LightHasherSha` (SHA256 hashing via DataHasher + ToByteArray)
//! - `LightDiscriminator` (unique 8-byte discriminator)
//! - `impl LightAccount for T` (unified trait with pack/unpack, compression_info accessors)
//! - `PackedXxx` struct (Pubkeys -> u8 indices, excludes compression_info)
//!
//! The `LightAccount` trait requires `Discriminator` and `DataHasher` supertraits.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{punctuated::Punctuated, DeriveInput, Field, Fields, Ident, ItemStruct, Result, Token};

use super::traits::{parse_compress_as_overrides, CompressAsFields};
use crate::{
    discriminator::discriminator,
    hasher::derive_light_hasher_sha,
    light_pdas::account::utils::{extract_fields_from_derive_input, is_copy_type, is_pubkey_type},
};

/// Checks if the struct has `#[account(zero_copy)]` attribute, indicating a zero-copy (Pod) type.
/// We check for `zero_copy` inside `#[account(...)]` to distinguish from regular `#[account]`
/// + `#[repr(C)]` structs (which already get AnchorSerialize from the `#[account]` macro).
fn is_zero_copy(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("account") {
            return false;
        }
        if let syn::Meta::List(meta_list) = &attr.meta {
            return meta_list.tokens.to_string().contains("zero_copy");
        }
        false
    })
}

/// Derives all required traits for a compressible account.
///
/// This generates:
/// - `LightHasherSha` - SHA256-based DataHasher and ToByteArray implementations
/// - `LightDiscriminator` - Unique 8-byte discriminator for the account type
/// - `impl LightAccount for T` - Unified trait with:
///   - `const ACCOUNT_TYPE: AccountType = AccountType::Pda`
///   - `type Packed = PackedT`
///   - `const INIT_SPACE: usize`
///   - `fn compression_info(&self)` / `fn compression_info_mut(&mut self)`
///   - `fn set_decompressed(&mut self, config, slot)`
///   - `fn pack(&self, accounts)` / `fn unpack(packed, accounts)`
/// - `PackedT` struct - Pubkeys -> u8 indices, compression_info excluded
///
/// # Example
///
/// ```ignore
/// use light_sdk_macros::{LightAccount, LightDiscriminator, LightHasherSha};
/// use light_sdk::compressible::CompressionInfo;
/// use solana_pubkey::Pubkey;
///
/// #[derive(Default, Debug, InitSpace, LightAccount, LightDiscriminator, LightHasherSha)]
/// #[account]
/// pub struct UserRecord {
///     pub compression_info: CompressionInfo,  // Non-Option, first or last field
///     pub owner: Pubkey,
///     #[max_len(32)]
///     pub name: String,
///     pub score: u64,
/// }
/// ```
///
/// ## Notes
///
/// - The `compression_info` field must be non-Option `CompressionInfo` type
/// - The `compression_info` field must be first or last field in the struct
/// - SHA256 hashing serializes the entire struct (no `#[hash]` needed)
/// - Use `#[compress_as(field = value)]` to override field values during compression
/// - Use `#[skip]` to exclude fields from compression entirely
pub fn derive_light_account(input: DeriveInput) -> Result<TokenStream> {
    // Convert DeriveInput to ItemStruct for macros that need it
    let item_struct = derive_input_to_item_struct(&input)?;

    // Generate LightHasherSha implementation
    let hasher_impl = derive_light_hasher_sha(item_struct.clone())?;

    // Generate LightDiscriminator implementation
    let discriminator_impl = discriminator(item_struct)?;

    // Generate unified LightAccount implementation (includes PackedXxx struct)
    let light_account_impl = generate_light_account_impl(&input)?;

    // For zero-copy (Pod) types, generate AnchorSerialize/AnchorDeserialize impls
    // using fully-qualified anchor_lang:: paths. This is necessary because the workspace
    // borsh dependency resolves to a different crate instance than anchor_lang's borsh
    // (due to proc-macro boundary causing crate duplication).
    let anchor_serde_impls = if is_zero_copy(&input.attrs) {
        generate_anchor_serde_for_zero_copy(&input)?
    } else {
        quote! {}
    };

    // Combine all implementations
    Ok(quote! {
        #hasher_impl
        #discriminator_impl
        #light_account_impl
        #anchor_serde_impls
    })
}

/// Converts a DeriveInput to an ItemStruct.
fn derive_input_to_item_struct(input: &DeriveInput) -> Result<ItemStruct> {
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "LightAccount can only be derived for structs",
            ))
        }
    };

    let fields = match &data.fields {
        Fields::Named(fields) => Fields::Named(fields.clone()),
        Fields::Unnamed(fields) => Fields::Unnamed(fields.clone()),
        Fields::Unit => Fields::Unit,
    };

    Ok(ItemStruct {
        attrs: input.attrs.clone(),
        vis: input.vis.clone(),
        struct_token: data.struct_token,
        ident: input.ident.clone(),
        generics: input.generics.clone(),
        fields,
        semi_token: data.semi_token,
    })
}

/// Validates that the struct has a `compression_info` field as first or last field.
/// Returns `Ok(true)` if first, `Ok(false)` if last, `Err` if missing or in middle.
fn validate_compression_info_field(
    fields: &Punctuated<Field, Token![,]>,
    struct_name: &Ident,
) -> Result<bool> {
    let field_count = fields.len();
    if field_count == 0 {
        return Err(syn::Error::new_spanned(
            struct_name,
            "Struct must have at least one field",
        ));
    }

    let first_is_compression_info = fields
        .first()
        .and_then(|f| f.ident.as_ref())
        .is_some_and(|name| name == "compression_info");

    let last_is_compression_info = fields
        .last()
        .and_then(|f| f.ident.as_ref())
        .is_some_and(|name| name == "compression_info");

    if first_is_compression_info {
        Ok(true)
    } else if last_is_compression_info {
        Ok(false)
    } else {
        Err(syn::Error::new_spanned(
            struct_name,
            "Field 'compression_info: CompressionInfo' must be the first or last field in the struct \
             for efficient serialization. Move it to the beginning or end of your struct definition.",
        ))
    }
}

/// Generates `AnchorSerialize` and `AnchorDeserialize` impls for zero-copy (Pod) types.
///
/// This is needed because the workspace `borsh` dependency and `anchor_lang`'s borsh
/// resolve to different crate instances (proc-macro boundary causes duplication).
/// Using `#[derive(BorshSerialize)]` would generate impls for the wrong borsh instance.
/// By generating field-by-field impls with fully-qualified `anchor_lang::` paths,
/// we ensure the impls satisfy `anchor_lang::AnchorSerialize` bounds.
fn generate_anchor_serde_for_zero_copy(input: &DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = extract_fields_from_derive_input(input)?;

    let serialize_fields: Vec<_> = fields
        .iter()
        .filter_map(|f| {
            let name = f.ident.as_ref()?;
            Some(quote! {
                anchor_lang::AnchorSerialize::serialize(&self.#name, writer)?;
            })
        })
        .collect();

    let deserialize_fields: Vec<_> = fields
        .iter()
        .filter_map(|f| {
            let name = f.ident.as_ref()?;
            Some(quote! {
                #name: anchor_lang::AnchorDeserialize::deserialize_reader(reader)?
            })
        })
        .collect();

    Ok(quote! {
        impl anchor_lang::AnchorSerialize for #struct_name {
            fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                #(#serialize_fields)*
                Ok(())
            }
        }

        impl anchor_lang::AnchorDeserialize for #struct_name {
            fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
                Ok(Self {
                    #(#deserialize_fields,)*
                })
            }
        }
    })
}

/// Generates the unified LightAccount trait implementation.
fn generate_light_account_impl(input: &DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let packed_struct_name = format_ident!("Packed{}", struct_name);
    let fields = extract_fields_from_derive_input(input)?;

    // Detect zero-copy (Pod) types via #[repr(C)]
    let is_zero_copy = is_zero_copy(&input.attrs);

    // Validate compression_info field position
    let _compression_info_first = validate_compression_info_field(fields, struct_name)?;

    // Parse compress_as overrides
    let compress_as_fields = parse_compress_as_overrides(&input.attrs)?;

    // Check if we have Pubkey fields (determines if we need a separate Packed struct)
    let has_pubkey_fields = fields
        .iter()
        .filter(|f| {
            f.ident
                .as_ref()
                .is_none_or(|name| name != "compression_info")
        })
        .any(|f| is_pubkey_type(&f.ty));

    // Generate the packed struct (excludes compression_info)
    let packed_struct = generate_packed_struct(&packed_struct_name, fields, has_pubkey_fields)?;

    // Generate pack method body
    let pack_body = generate_pack_body(&packed_struct_name, fields, has_pubkey_fields)?;

    // Generate unpack method body
    let unpack_body =
        generate_unpack_body(struct_name, &packed_struct_name, fields, has_pubkey_fields)?;

    // Generate compress_as body for set_decompressed
    let compress_as_assignments = generate_compress_as_assignments(fields, &compress_as_fields);

    // Generate compress_as impl body for CompressAs trait
    let compress_as_impl_body = generate_compress_as_impl_body(fields, &compress_as_fields);

    // Generate the 800-byte size assertion and account type based on zero-copy mode
    let (size_assertion, account_type_token, init_space_token) = if is_zero_copy {
        (
            quote! {
                const _: () = {
                    assert!(
                        core::mem::size_of::<#struct_name>() <= 800,
                        "Compressed account size exceeds 800 byte limit"
                    );
                };
            },
            quote! { light_sdk::interface::AccountType::PdaZeroCopy },
            quote! { core::mem::size_of::<Self>() },
        )
    } else {
        (
            quote! {
                const _: () = {
                    assert!(
                        <#struct_name as anchor_lang::Space>::INIT_SPACE <= 800,
                        "Compressed account size exceeds 800 byte limit"
                    );
                };
            },
            quote! { light_sdk::interface::AccountType::Pda },
            quote! { <Self as anchor_lang::Space>::INIT_SPACE },
        )
    };

    // Generate the LightAccount impl
    let light_account_impl = quote! {
        #packed_struct

        #size_assertion

        impl light_sdk::interface::LightAccount for #struct_name {
            const ACCOUNT_TYPE: light_sdk::interface::AccountType = #account_type_token;

            type Packed = #packed_struct_name;

            const INIT_SPACE: usize = #init_space_token;

            #[inline]
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                &self.compression_info
            }

            #[inline]
            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                &mut self.compression_info
            }

            fn set_decompressed(&mut self, config: &light_sdk::interface::LightConfig, current_slot: u64) {
                self.compression_info = light_sdk::compressible::CompressionInfo::new_from_config(config, current_slot);
                #compress_as_assignments
            }

            #[inline(never)]
            fn pack(
                &self,
                accounts: &mut light_sdk::instruction::PackedAccounts,
            ) -> std::result::Result<Self::Packed, solana_program_error::ProgramError> {
                #pack_body
            }

            #[inline(never)]
            fn unpack<A: light_sdk::light_account_checks::AccountInfoTrait>(
                packed: &Self::Packed,
                accounts: &light_sdk::light_account_checks::packed_accounts::ProgramPackedAccounts<A>,
            ) -> std::result::Result<Self, solana_program_error::ProgramError> {
                #unpack_body
            }
        }

        // V1 compatibility: Pack trait (delegates to LightAccount::pack)
        // Pack trait is only available off-chain (client-side)
        #[cfg(not(target_os = "solana"))]
        impl light_sdk::interface::Pack for #struct_name {
            type Packed = #packed_struct_name;

            fn pack(
                &self,
                remaining_accounts: &mut light_sdk::instruction::PackedAccounts,
            ) -> std::result::Result<Self::Packed, solana_program_error::ProgramError> {
                <Self as light_sdk::interface::LightAccount>::pack(self, remaining_accounts)
            }
        }

        // V1 compatibility: Unpack trait for packed struct
        impl light_sdk::interface::Unpack for #packed_struct_name {
            type Unpacked = #struct_name;

            fn unpack(
                &self,
                remaining_accounts: &[solana_account_info::AccountInfo],
            ) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
                // Create a ProgramPackedAccounts wrapper from remaining_accounts
                let accounts = light_sdk::light_account_checks::packed_accounts::ProgramPackedAccounts {
                    accounts: remaining_accounts
                };
                <#struct_name as light_sdk::interface::LightAccount>::unpack(self, &accounts)
            }
        }

        // V1 compatibility: HasCompressionInfo trait (wraps non-Option compression_info)
        impl light_sdk::interface::HasCompressionInfo for #struct_name {
            fn compression_info(&self) -> std::result::Result<&light_sdk::interface::CompressionInfo, solana_program_error::ProgramError> {
                Ok(&self.compression_info)
            }

            fn compression_info_mut(&mut self) -> std::result::Result<&mut light_sdk::interface::CompressionInfo, solana_program_error::ProgramError> {
                Ok(&mut self.compression_info)
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::interface::CompressionInfo> {
                // V2 types use non-Option CompressionInfo, so this can't return a reference
                // This method is only used by V1 code paths that expect Option<CompressionInfo>
                panic!("compression_info_mut_opt not supported for LightAccount types (use compression_info_mut instead)")
            }

            fn set_compression_info_none(&mut self) -> std::result::Result<(), solana_program_error::ProgramError> {
                // V2 types use non-Option CompressionInfo
                // Setting to "compressed" state is the equivalent of "None" for V1
                self.compression_info = light_sdk::compressible::CompressionInfo::compressed();
                Ok(())
            }
        }

        // V1 compatibility: Size trait
        impl light_sdk::account::Size for #struct_name {
            #[inline]
            fn size(&self) -> std::result::Result<usize, solana_program_error::ProgramError> {
                Ok(<Self as light_sdk::interface::LightAccount>::INIT_SPACE)
            }
        }

        // V1 compatibility: CompressAs trait
        impl light_sdk::interface::CompressAs for #struct_name {
            type Output = Self;

            fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
                #compress_as_impl_body
            }
        }

        // V1 compatibility: CompressedInitSpace trait
        impl light_sdk::interface::CompressedInitSpace for #struct_name {
            const COMPRESSED_INIT_SPACE: usize = <Self as light_sdk::interface::LightAccount>::INIT_SPACE;
        }
    };

    Ok(light_account_impl)
}

/// Generates the PackedXxx struct definition.
/// Excludes compression_info field to save 24 bytes.
fn generate_packed_struct(
    packed_struct_name: &Ident,
    fields: &Punctuated<Field, Token![,]>,
    has_pubkey_fields: bool,
) -> Result<TokenStream> {
    if !has_pubkey_fields {
        // No Pubkey fields - Packed is just a type alias (but still excludes compression_info)
        // We need a minimal struct that just holds non-pubkey fields
        let non_compression_fields: Vec<_> = fields
            .iter()
            .filter(|f| {
                f.ident
                    .as_ref()
                    .is_none_or(|name| name != "compression_info")
            })
            .collect();

        if non_compression_fields.is_empty() {
            // Only compression_info field - create empty struct
            return Ok(quote! {
                #[derive(Debug, Clone, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
                pub struct #packed_struct_name;
            });
        }

        // Create struct with same fields (no Pubkey transformation needed)
        let packed_fields = non_compression_fields.iter().filter_map(|field| {
            let field_name = field.ident.as_ref()?;
            let field_type = &field.ty;
            Some(quote! { pub #field_name: #field_type })
        });

        return Ok(quote! {
            #[derive(Debug, Clone, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
            pub struct #packed_struct_name {
                #(#packed_fields,)*
            }
        });
    }

    // Has Pubkey fields - generate packed struct with u8 indices
    let packed_fields = fields.iter().filter_map(|field| {
        let field_name = field.ident.as_ref()?;

        // Skip compression_info - not included in packed struct
        if field_name == "compression_info" {
            return None;
        }

        let field_type = &field.ty;
        let packed_type = if is_pubkey_type(field_type) {
            quote! { u8 }
        } else {
            quote! { #field_type }
        };

        Some(quote! { pub #field_name: #packed_type })
    });

    Ok(quote! {
        #[derive(Debug, Clone, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
        pub struct #packed_struct_name {
            #(#packed_fields,)*
        }
    })
}

/// Generates the pack method body.
fn generate_pack_body(
    packed_struct_name: &Ident,
    fields: &Punctuated<Field, Token![,]>,
    has_pubkey_fields: bool,
) -> Result<TokenStream> {
    let pack_assignments: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?;

            // Skip compression_info - excluded from packed struct
            if field_name == "compression_info" {
                return None;
            }

            let field_type = &field.ty;

            Some(if is_pubkey_type(field_type) {
                quote! { #field_name: accounts.insert_or_get_read_only(self.#field_name) }
            } else if is_copy_type(field_type) {
                quote! { #field_name: self.#field_name }
            } else {
                quote! { #field_name: self.#field_name.clone() }
            })
        })
        .collect();

    if !has_pubkey_fields && pack_assignments.is_empty() {
        // Only compression_info field - return empty packed struct
        return Ok(quote! {
            Ok(#packed_struct_name)
        });
    }

    Ok(quote! {
        Ok(#packed_struct_name {
            #(#pack_assignments,)*
        })
    })
}

/// Generates the unpack method body.
fn generate_unpack_body(
    struct_name: &Ident,
    _packed_struct_name: &Ident,
    fields: &Punctuated<Field, Token![,]>,
    has_pubkey_fields: bool,
) -> Result<TokenStream> {
    let struct_name_str = struct_name.to_string();

    let unpack_assignments: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?;
            let field_type = &field.ty;

            // compression_info gets canonical value
            if field_name == "compression_info" {
                return Some(quote! {
                    #field_name: light_sdk::compressible::CompressionInfo::compressed()
                });
            }

            Some(if is_pubkey_type(field_type) {
                let error_msg = format!("{}: {}", struct_name_str, field_name);
                quote! {
                    #field_name: {
                        let account = accounts
                            .get_u8(packed.#field_name, #error_msg)
                            .map_err(|_| solana_program_error::ProgramError::InvalidAccountData)?;
                        solana_pubkey::Pubkey::from(account.key())
                    }
                }
            } else if !has_pubkey_fields {
                // For structs without pubkey fields, fields are directly copied
                if is_copy_type(field_type) {
                    quote! { #field_name: packed.#field_name }
                } else {
                    quote! { #field_name: packed.#field_name.clone() }
                }
            } else if is_copy_type(field_type) {
                quote! { #field_name: packed.#field_name }
            } else {
                quote! { #field_name: packed.#field_name.clone() }
            })
        })
        .collect();

    Ok(quote! {
        Ok(#struct_name {
            #(#unpack_assignments,)*
        })
    })
}

/// Generates assignments for compress_as overrides.
/// These are applied during set_decompressed to reset transient fields.
fn generate_compress_as_assignments(
    fields: &Punctuated<Field, Token![,]>,
    compress_as_fields: &Option<CompressAsFields>,
) -> TokenStream {
    let Some(overrides) = compress_as_fields else {
        return quote! {};
    };

    let assignments: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?;

            // Skip compression_info (already set)
            if field_name == "compression_info" {
                return None;
            }

            // Skip fields marked with #[skip]
            if field.attrs.iter().any(|attr| attr.path().is_ident("skip")) {
                return None;
            }

            // Check if this field has an override
            let override_field = overrides.fields.iter().find(|f| &f.name == field_name)?;
            let value = &override_field.value;

            Some(quote! {
                self.#field_name = #value;
            })
        })
        .collect();

    quote! { #(#assignments)* }
}

/// Generates the body for CompressAs::compress_as() method.
/// If no overrides: returns Cow::Borrowed(self)
/// If overrides exist: returns Cow::Owned(modified_clone)
fn generate_compress_as_impl_body(
    fields: &Punctuated<Field, Token![,]>,
    compress_as_fields: &Option<CompressAsFields>,
) -> TokenStream {
    let Some(overrides) = compress_as_fields else {
        // No overrides - clone and set compression_info to Compressed
        return quote! {
            let mut result = self.clone();
            result.compression_info = light_sdk::compressible::CompressionInfo::compressed();
            std::borrow::Cow::Owned(result)
        };
    };

    // Collect the override assignments
    let assignments: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?;

            // Skip compression_info
            if field_name == "compression_info" {
                return None;
            }

            // Skip fields marked with #[skip]
            if field.attrs.iter().any(|attr| attr.path().is_ident("skip")) {
                return None;
            }

            // Check if this field has an override
            let override_field = overrides.fields.iter().find(|f| &f.name == field_name)?;
            let value = &override_field.value;

            Some(quote! {
                result.#field_name = #value;
            })
        })
        .collect();

    if assignments.is_empty() {
        // No field overrides - clone and set compression_info to Compressed
        quote! {
            let mut result = self.clone();
            result.compression_info = light_sdk::compressible::CompressionInfo::compressed();
            std::borrow::Cow::Owned(result)
        }
    } else {
        // Clone, set compression_info to Compressed, and apply overrides
        quote! {
            let mut result = self.clone();
            result.compression_info = light_sdk::compressible::CompressionInfo::compressed();
            #(#assignments)*
            std::borrow::Cow::Owned(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_light_account_basic() {
        let input: DeriveInput = parse_quote! {
            pub struct UserRecord {
                pub compression_info: CompressionInfo,
                pub owner: Pubkey,
                pub name: String,
                pub score: u64,
            }
        };

        let result = derive_light_account(input);
        assert!(result.is_ok(), "LightAccount should succeed");

        let output = result.unwrap().to_string();

        // Should contain LightHasherSha output
        assert!(output.contains("DataHasher"), "Should implement DataHasher");
        assert!(
            output.contains("ToByteArray"),
            "Should implement ToByteArray"
        );

        // Should contain LightDiscriminator output
        assert!(
            output.contains("LightDiscriminator"),
            "Should implement LightDiscriminator"
        );
        assert!(
            output.contains("LIGHT_DISCRIMINATOR"),
            "Should have discriminator constant"
        );

        // Should contain unified LightAccount implementation
        assert!(
            output.contains("impl light_sdk :: interface :: LightAccount for UserRecord"),
            "Should implement LightAccount trait"
        );

        // Should contain PackedUserRecord struct
        assert!(
            output.contains("PackedUserRecord"),
            "Should generate Packed struct"
        );

        // Should contain ACCOUNT_TYPE constant
        assert!(
            output.contains("ACCOUNT_TYPE"),
            "Should have ACCOUNT_TYPE constant"
        );

        // Should contain INIT_SPACE constant
        assert!(
            output.contains("INIT_SPACE"),
            "Should have INIT_SPACE constant"
        );

        // Should contain 800-byte size assertion
        assert!(
            output.contains("800"),
            "Should have 800-byte size assertion"
        );

        // Should contain compression_info accessors
        assert!(
            output.contains("compression_info"),
            "Should have compression_info methods"
        );

        // Should contain pack/unpack methods
        assert!(output.contains("fn pack"), "Should have pack method");
        assert!(output.contains("fn unpack"), "Should have unpack method");

        // Should contain set_decompressed method
        assert!(
            output.contains("set_decompressed"),
            "Should have set_decompressed method"
        );
    }

    #[test]
    fn test_light_account_with_compress_as() {
        let input: DeriveInput = parse_quote! {
            #[compress_as(start_time = 0, score = 0)]
            pub struct GameSession {
                pub compression_info: CompressionInfo,
                pub session_id: u64,
                pub player: Pubkey,
                pub start_time: u64,
                pub score: u64,
            }
        };

        let result = derive_light_account(input);
        assert!(
            result.is_ok(),
            "LightAccount with compress_as should succeed"
        );

        let output = result.unwrap().to_string();
        assert!(
            output.contains("LightAccount"),
            "Should implement LightAccount"
        );
    }

    #[test]
    fn test_light_account_no_pubkey_fields() {
        let input: DeriveInput = parse_quote! {
            pub struct SimpleRecord {
                pub compression_info: CompressionInfo,
                pub id: u64,
                pub value: u32,
            }
        };

        let result = derive_light_account(input);
        assert!(
            result.is_ok(),
            "LightAccount without Pubkey fields should succeed"
        );

        let output = result.unwrap().to_string();
        assert!(output.contains("DataHasher"), "Should implement DataHasher");
        assert!(
            output.contains("LightDiscriminator"),
            "Should implement LightDiscriminator"
        );
        assert!(
            output.contains("LightAccount"),
            "Should implement LightAccount"
        );
    }

    #[test]
    fn test_light_account_enum_fails() {
        let input: DeriveInput = parse_quote! {
            pub enum NotAStruct {
                A,
                B,
            }
        };

        let result = derive_light_account(input);
        assert!(result.is_err(), "LightAccount should fail for enums");
    }

    #[test]
    fn test_light_account_missing_compression_info() {
        let input: DeriveInput = parse_quote! {
            pub struct MissingCompressionInfo {
                pub id: u64,
                pub value: u32,
            }
        };

        let result = derive_light_account(input);
        assert!(
            result.is_err(),
            "Should fail without compression_info field"
        );
    }

    #[test]
    fn test_light_account_compression_info_in_middle_fails() {
        let input: DeriveInput = parse_quote! {
            pub struct BadLayout {
                pub id: u64,
                pub compression_info: CompressionInfo,
                pub value: u32,
            }
        };

        let result = derive_light_account(input);
        assert!(
            result.is_err(),
            "Should fail when compression_info is in middle"
        );
    }

    #[test]
    fn test_packed_struct_excludes_compression_info() {
        let input: DeriveInput = parse_quote! {
            pub struct UserRecord {
                pub compression_info: CompressionInfo,
                pub owner: Pubkey,
                pub score: u64,
            }
        };

        let result = derive_light_account(input);
        assert!(result.is_ok());

        let output = result.unwrap().to_string();

        // PackedUserRecord should have owner (as u8) and score, but NOT compression_info
        assert!(
            output.contains("pub struct PackedUserRecord"),
            "Should generate PackedUserRecord"
        );
        // The packed struct should contain owner as u8
        assert!(
            output.contains("pub owner : u8"),
            "Packed struct should have owner as u8"
        );
    }
}
