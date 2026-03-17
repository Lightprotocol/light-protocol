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

/// Target framework for generated code.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    Anchor,
    Pinocchio,
}

impl Framework {
    /// Crate path prefix for on-chain code (LightAccount trait, AccountType, etc.)
    fn on_chain_crate(&self) -> TokenStream {
        match self {
            Framework::Anchor => quote! { light_account },
            Framework::Pinocchio => quote! { light_account_pinocchio },
        }
    }

    /// Serialization derives for packed struct.
    fn serde_derives(&self) -> TokenStream {
        match self {
            Framework::Anchor => {
                quote! { anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize }
            }
            Framework::Pinocchio => quote! { borsh::BorshSerialize, borsh::BorshDeserialize },
        }
    }
}

use super::{
    traits::{parse_compress_as_overrides, CompressAsFields},
    validation::validate_compression_info_field,
};
use crate::{
    discriminator,
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

/// Derives all required traits for a compressible account (Anchor variant).
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
/// use light_account::CompressionInfo;
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
    derive_light_account_internal(input, Framework::Anchor)
}

/// Derives all required traits for a compressible account (Pinocchio variant).
///
/// Same as `derive_light_account` but generates pinocchio-compatible code:
/// - Uses `BorshSerialize/BorshDeserialize` instead of Anchor serialization
/// - Uses `light_account_pinocchio::` paths for on-chain code
/// - Uses `core::mem::size_of::<Self>()` for INIT_SPACE
pub fn derive_light_pinocchio_account(input: DeriveInput) -> Result<TokenStream> {
    derive_light_account_internal(input, Framework::Pinocchio)
}

/// Parses the `discriminator` bytes from `#[light_pinocchio(discriminator = [...])]` if present.
/// Returns None if the attribute is absent (use hash-derived discriminator).
fn parse_pinocchio_discriminator(attrs: &[syn::Attribute]) -> Result<Option<Vec<u8>>> {
    for attr in attrs {
        if !attr.path().is_ident("light_pinocchio") {
            continue;
        }
        let meta_list = attr.meta.require_list()?;
        let nested: Punctuated<syn::Meta, Token![,]> =
            meta_list.parse_args_with(Punctuated::parse_terminated)?;
        for meta in &nested {
            if let syn::Meta::NameValue(nv) = meta {
                if nv.path.is_ident("discriminator") {
                    if let syn::Expr::Array(arr) = &nv.value {
                        let bytes: Vec<u8> = arr
                            .elems
                            .iter()
                            .map(|e| {
                                if let syn::Expr::Lit(lit) = e {
                                    if let syn::Lit::Int(i) = &lit.lit {
                                        return i
                                            .base10_parse::<u8>()
                                            .map_err(|err| syn::Error::new_spanned(i, err));
                                    }
                                }
                                if let syn::Expr::Cast(cast) = e {
                                    if let syn::Expr::Lit(lit) = cast.expr.as_ref() {
                                        if let syn::Lit::Int(i) = &lit.lit {
                                            return i
                                                .base10_parse::<u8>()
                                                .map_err(|err| syn::Error::new_spanned(i, err));
                                        }
                                    }
                                }
                                Err(syn::Error::new_spanned(e, "expected integer literal"))
                            })
                            .collect::<Result<Vec<u8>>>()?;
                        if bytes.is_empty() {
                            return Err(syn::Error::new_spanned(
                                arr,
                                "discriminator must have at least one byte",
                            ));
                        }
                        if bytes.len() > 8 {
                            return Err(syn::Error::new_spanned(
                                arr,
                                "discriminator must not exceed 8 bytes",
                            ));
                        }
                        return Ok(Some(bytes));
                    }
                    return Err(syn::Error::new_spanned(
                        &nv.value,
                        "discriminator must be an array like [1u8]",
                    ));
                }
            }
        }
    }
    Ok(None)
}

/// Internal implementation of LightAccount derive, parameterized by framework.
fn derive_light_account_internal(input: DeriveInput, framework: Framework) -> Result<TokenStream> {
    // Convert DeriveInput to ItemStruct for macros that need it
    let item_struct = derive_input_to_item_struct(&input)?;

    // Generate LightHasherSha implementation
    let hasher_impl = derive_light_hasher_sha(item_struct.clone())?;

    // Check for custom discriminator argument from #[light_pinocchio(discriminator = [...])]
    // Only valid for the Pinocchio framework; reject it on Anchor to avoid silent misuse.
    let discriminator_impl = if let Some(disc_bytes) = parse_pinocchio_discriminator(&input.attrs)?
    {
        if framework != Framework::Pinocchio {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "#[light_pinocchio(discriminator = [...])] is only valid with \
                 #[derive(LightPinocchioAccount)], not with #[derive(LightAccount)]",
            ));
        }
        let mut padded = [0u8; 8];
        let copy_len = disc_bytes.len().min(8);
        padded[..copy_len].copy_from_slice(&disc_bytes[..copy_len]);
        let discriminator_tokens: proc_macro2::TokenStream = format!("{padded:?}").parse().unwrap();
        let slice_tokens: proc_macro2::TokenStream = format!("{disc_bytes:?}").parse().unwrap();
        let struct_name = &input.ident;
        let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();
        quote! {
            impl #impl_gen LightDiscriminator for #struct_name #type_gen #where_clause {
                const LIGHT_DISCRIMINATOR: [u8; 8] = #discriminator_tokens;
                const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &#slice_tokens;
                fn discriminator() -> [u8; 8] { Self::LIGHT_DISCRIMINATOR }
            }
        }
    } else {
        // Generate LightDiscriminator implementation via SHA256
        discriminator::anchor_discriminator(item_struct)?
    };

    // Generate unified LightAccount implementation (includes PackedXxx struct)
    let light_account_impl = generate_light_account_impl(&input, framework)?;

    // For zero-copy (Pod) types with Anchor, generate AnchorSerialize/AnchorDeserialize impls
    // using fully-qualified anchor_lang:: paths. This is necessary because the workspace
    // borsh dependency resolves to a different crate instance than anchor_lang's borsh
    // (due to proc-macro boundary causing crate duplication).
    // For Pinocchio, we don't generate these - the struct should already derive BorshSerialize/BorshDeserialize.
    let anchor_serde_impls = if framework == Framework::Anchor && is_zero_copy(&input.attrs) {
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
fn generate_light_account_impl(input: &DeriveInput, framework: Framework) -> Result<TokenStream> {
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
    let packed_struct =
        generate_packed_struct(&packed_struct_name, fields, has_pubkey_fields, framework)?;

    // Generate pack method body (off-chain)
    let pack_body = generate_pack_body(&packed_struct_name, fields, has_pubkey_fields, framework)?;

    // Generate unpack method body (on-chain, uses framework-specific paths)
    let unpack_body = generate_unpack_body(struct_name, fields, has_pubkey_fields, framework)?;

    // Generate compress_as body for set_decompressed
    let compress_as_assignments =
        generate_compress_as_assignments(fields, &compress_as_fields, framework);

    // Generate compress_as impl body for CompressAs trait
    let compress_as_impl_body =
        generate_compress_as_impl_body(fields, &compress_as_fields, framework);

    // Get the on-chain crate path (light_account or light_account_pinocchio)
    let on_chain_crate = framework.on_chain_crate();

    // Generate the 800-byte size assertion and account type based on framework and zero-copy mode
    let (size_assertion, account_type_token, init_space_token) = match framework {
        Framework::Pinocchio => {
            // Pinocchio always uses core::mem::size_of and PdaZeroCopy
            (
                quote! {
                    const _: () = {
                        assert!(
                            core::mem::size_of::<#struct_name>() <= 800,
                            "Compressed account size exceeds 800 byte limit"
                        );
                    };
                },
                quote! { #on_chain_crate::AccountType::PdaZeroCopy },
                quote! { core::mem::size_of::<Self>() },
            )
        }
        Framework::Anchor => {
            if is_zero_copy {
                (
                    quote! {
                        const _: () = {
                            assert!(
                                core::mem::size_of::<#struct_name>() <= 800,
                                "Compressed account size exceeds 800 byte limit"
                            );
                        };
                    },
                    quote! { #on_chain_crate::AccountType::PdaZeroCopy },
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
                    quote! { #on_chain_crate::AccountType::Pda },
                    quote! { <Self as anchor_lang::Space>::INIT_SPACE },
                )
            }
        }
    };

    // Generate the LightAccount impl
    // Note: pack is off-chain only, uses light_account:: paths
    // unpack is on-chain, uses framework-specific paths
    let light_account_impl = quote! {
        #packed_struct

        #size_assertion

        impl #on_chain_crate::LightAccount for #struct_name {
            const ACCOUNT_TYPE: #on_chain_crate::AccountType = #account_type_token;

            type Packed = #packed_struct_name;

            const INIT_SPACE: usize = #init_space_token;

            #[inline]
            fn compression_info(&self) -> &#on_chain_crate::CompressionInfo {
                &self.compression_info
            }

            #[inline]
            fn compression_info_mut(&mut self) -> &mut #on_chain_crate::CompressionInfo {
                &mut self.compression_info
            }

            fn set_decompressed(&mut self, config: &#on_chain_crate::LightConfig, current_slot: u64) {
                self.compression_info = #on_chain_crate::CompressionInfo::new_from_config(config, current_slot);
                #compress_as_assignments
            }

            // pack is off-chain only (client-side)
            #[cfg(not(target_os = "solana"))]
            #[inline(never)]
            fn pack<AM: #on_chain_crate::AccountMetaTrait>(
                &self,
                accounts: &mut #on_chain_crate::interface::instruction::PackedAccounts<AM>,
            ) -> std::result::Result<Self::Packed, #on_chain_crate::LightSdkTypesError> {
                #pack_body
            }

            // unpack is on-chain - uses framework-specific paths
            #[inline(never)]
            fn unpack<A: #on_chain_crate::AccountInfoTrait>(
                packed: &Self::Packed,
                accounts: &#on_chain_crate::packed_accounts::ProgramPackedAccounts<A>,
            ) -> std::result::Result<Self, #on_chain_crate::LightSdkTypesError> {
                #unpack_body
            }
        }

        // V1 compatibility: Pack trait (delegates to LightAccount::pack)
        // Pack trait is off-chain only (client-side)
        #[cfg(not(target_os = "solana"))]
        impl<AM: #on_chain_crate::AccountMetaTrait> #on_chain_crate::Pack<AM> for #struct_name {
            type Packed = #packed_struct_name;

            fn pack(
                &self,
                remaining_accounts: &mut #on_chain_crate::interface::instruction::PackedAccounts<AM>,
            ) -> std::result::Result<Self::Packed, #on_chain_crate::LightSdkTypesError> {
                <Self as #on_chain_crate::LightAccount>::pack(self, remaining_accounts)
            }
        }

        // V1 compatibility: Unpack trait for packed struct
        // Uses framework-specific paths for on-chain code
        impl<AI: #on_chain_crate::AccountInfoTrait> #on_chain_crate::Unpack<AI> for #packed_struct_name {
            type Unpacked = #struct_name;

            fn unpack(
                &self,
                remaining_accounts: &[AI],
            ) -> std::result::Result<Self::Unpacked, #on_chain_crate::LightSdkTypesError> {
                // Create a ProgramPackedAccounts wrapper from remaining_accounts
                let accounts = #on_chain_crate::packed_accounts::ProgramPackedAccounts {
                    accounts: remaining_accounts
                };
                <#struct_name as #on_chain_crate::LightAccount>::unpack(self, &accounts)
            }
        }

        // V1 compatibility: HasCompressionInfo trait (wraps non-Option compression_info)
        impl #on_chain_crate::HasCompressionInfo for #struct_name {
            fn compression_info(&self) -> std::result::Result<&#on_chain_crate::CompressionInfo, #on_chain_crate::LightSdkTypesError> {
                Ok(&self.compression_info)
            }

            fn compression_info_mut(&mut self) -> std::result::Result<&mut #on_chain_crate::CompressionInfo, #on_chain_crate::LightSdkTypesError> {
                Ok(&mut self.compression_info)
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<#on_chain_crate::CompressionInfo> {
                // V2 types use non-Option CompressionInfo, so this can't return a reference
                // This method is only used by V1 code paths that expect Option<CompressionInfo>
                panic!("compression_info_mut_opt not supported for LightAccount types (use compression_info_mut instead)")
            }

            fn set_compression_info_none(&mut self) -> std::result::Result<(), #on_chain_crate::LightSdkTypesError> {
                // V2 types use non-Option CompressionInfo
                // Setting to "compressed" state is the equivalent of "None" for V1
                self.compression_info = #on_chain_crate::CompressionInfo::compressed();
                Ok(())
            }
        }

        // V1 compatibility: Size trait
        impl #on_chain_crate::Size for #struct_name {
            #[inline]
            fn size(&self) -> std::result::Result<usize, #on_chain_crate::LightSdkTypesError> {
                Ok(<Self as #on_chain_crate::LightAccount>::INIT_SPACE)
            }
        }

        // V1 compatibility: CompressAs trait
        impl #on_chain_crate::CompressAs for #struct_name {
            type Output = Self;

            fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
                #compress_as_impl_body
            }
        }

        // V1 compatibility: CompressedInitSpace trait
        impl #on_chain_crate::CompressedInitSpace for #struct_name {
            const COMPRESSED_INIT_SPACE: usize = <Self as #on_chain_crate::LightAccount>::INIT_SPACE;
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
    framework: Framework,
) -> Result<TokenStream> {
    let serde_derives = framework.serde_derives();

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
                #[derive(Debug, Clone, #serde_derives)]
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
            #[derive(Debug, Clone, #serde_derives)]
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
        #[derive(Debug, Clone, #serde_derives)]
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
    framework: Framework,
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
                // Anchor Pubkey has .to_bytes(), pinocchio Pubkey is [u8; 32]
                match framework {
                    Framework::Anchor => {
                        quote! { #field_name: accounts.insert_or_get_read_only(AM::pubkey_from_bytes(self.#field_name.to_bytes())) }
                    }
                    Framework::Pinocchio => {
                        quote! { #field_name: accounts.insert_or_get_read_only(AM::pubkey_from_bytes(self.#field_name)) }
                    }
                }
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
/// Uses framework-specific paths for on-chain code.
fn generate_unpack_body(
    struct_name: &Ident,
    fields: &Punctuated<Field, Token![,]>,
    has_pubkey_fields: bool,
    framework: Framework,
) -> Result<TokenStream> {
    let struct_name_str = struct_name.to_string();
    let on_chain_crate = framework.on_chain_crate();

    let unpack_assignments: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?;
            let field_type = &field.ty;

            // compression_info gets canonical value
            if field_name == "compression_info" {
                return Some(quote! {
                    #field_name: #on_chain_crate::CompressionInfo::compressed()
                });
            }

            Some(if is_pubkey_type(field_type) {
                let error_msg = format!("{}: {}", struct_name_str, field_name);
                // For Anchor: convert [u8; 32] to solana_pubkey::Pubkey
                // For Pinocchio: Pubkey is [u8; 32], so use key() directly
                let key_conversion = match framework {
                    Framework::Anchor => quote! { solana_pubkey::Pubkey::from(account.key()) },
                    Framework::Pinocchio => quote! { account.key() },
                };
                quote! {
                    #field_name: {
                        let account = accounts
                            .get_u8(packed.#field_name, #error_msg)
                            .map_err(|_| #on_chain_crate::LightSdkTypesError::InvalidInstructionData)?;
                        #key_conversion
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
    _framework: Framework,
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
    framework: Framework,
) -> TokenStream {
    let on_chain_crate = framework.on_chain_crate();

    let Some(overrides) = compress_as_fields else {
        // No overrides - clone and set compression_info to Compressed
        return quote! {
            let mut result = self.clone();
            result.compression_info = #on_chain_crate::CompressionInfo::compressed();
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
            result.compression_info = #on_chain_crate::CompressionInfo::compressed();
            std::borrow::Cow::Owned(result)
        }
    } else {
        // Clone, set compression_info to Compressed, and apply overrides
        quote! {
            let mut result = self.clone();
            result.compression_info = #on_chain_crate::CompressionInfo::compressed();
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
    fn test_light_pinocchio_custom_discriminator() {
        let input: DeriveInput = parse_quote! {
            #[light_pinocchio(discriminator = [1u8])]
            pub struct OneByteRecord {
                pub compression_info: CompressionInfo,
                pub owner: [u8; 32],
            }
        };

        let result = derive_light_pinocchio_account(input);
        assert!(
            result.is_ok(),
            "LightPinocchioAccount with custom discriminator should succeed: {:?}",
            result.err()
        );

        let output = result.unwrap().to_string();

        // Should contain custom discriminator (1, 0, 0, 0, 0, 0, 0, 0)
        assert!(
            output.contains("LIGHT_DISCRIMINATOR"),
            "Should have LIGHT_DISCRIMINATOR"
        );
        assert!(
            output.contains("1 , 0 , 0 , 0 , 0 , 0 , 0 , 0")
                || output.contains("1, 0, 0, 0, 0, 0, 0, 0"),
            "LIGHT_DISCRIMINATOR should be [1,0,0,0,0,0,0,0]"
        );
        // LIGHT_DISCRIMINATOR_SLICE must be &[1] (1 byte), NOT the padded &[1, 0, 0, 0, 0, 0, 0, 0]
        assert!(
            output.contains("LIGHT_DISCRIMINATOR_SLICE"),
            "Should have LIGHT_DISCRIMINATOR_SLICE"
        );
        // Verify the slice contains exactly 1 element (not 8)
        // The generated token stream renders as `& [1u8]` or `& [1]`
        assert!(
            output.contains("& [1u8]") || output.contains("& [1]"),
            "LIGHT_DISCRIMINATOR_SLICE should be &[1] (1 byte), got: {output}"
        );
    }

    #[test]
    fn test_light_pinocchio_custom_discriminator_empty_rejected() {
        let input: DeriveInput = parse_quote! {
            #[light_pinocchio(discriminator = [])]
            pub struct EmptyDisc {
                pub compression_info: CompressionInfo,
                pub owner: [u8; 32],
            }
        };
        let result = derive_light_pinocchio_account(input);
        assert!(
            result.is_err(),
            "Empty discriminator array should be rejected"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("at least one byte"),
            "Error should mention 'at least one byte', got: {err}"
        );
    }

    #[test]
    fn test_light_pinocchio_custom_discriminator_too_long_rejected() {
        let input: DeriveInput = parse_quote! {
            #[light_pinocchio(discriminator = [1, 2, 3, 4, 5, 6, 7, 8, 9])]
            pub struct TooLongDisc {
                pub compression_info: CompressionInfo,
                pub owner: [u8; 32],
            }
        };
        let result = derive_light_pinocchio_account(input);
        assert!(
            result.is_err(),
            "Discriminator longer than 8 bytes should be rejected"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceed 8 bytes"),
            "Error should mention max length, got: {err}"
        );
    }

    #[test]
    fn test_light_pinocchio_discriminator_rejected_on_anchor() {
        let input: DeriveInput = parse_quote! {
            #[light_pinocchio(discriminator = [1u8])]
            pub struct AnchorRecord {
                pub compression_info: CompressionInfo,
                pub owner: Pubkey,
            }
        };
        let result = derive_light_account(input);
        assert!(
            result.is_err(),
            "#[light_pinocchio(discriminator)] should be rejected with LightAccount (Anchor)"
        );
    }

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
            output.contains("impl light_account :: LightAccount for UserRecord"),
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
