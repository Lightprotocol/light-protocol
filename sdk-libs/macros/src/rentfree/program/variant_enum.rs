use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result, Type};

use super::parsing::{SeedElement, TokenSeedSpec};
use crate::rentfree::shared_utils::{
    make_packed_type, make_packed_variant_name, qualify_type_with_crate,
};

/// Info about ctx.* seeds for a PDA type
#[derive(Clone, Debug)]
pub struct PdaCtxSeedInfo {
    /// The variant name (derived from field name, e.g., "Record" from field "record")
    pub variant_name: Ident,
    /// The inner type (e.g., crate::state::SinglePubkeyRecord - preserves full path)
    pub inner_type: Type,
    /// Field names from ctx.accounts.XXX references in seeds
    pub ctx_seed_fields: Vec<Ident>,
    /// Field names that exist on the state struct (for filtering data.* seeds)
    pub state_field_names: std::collections::HashSet<String>,
}

impl PdaCtxSeedInfo {
    pub fn with_state_fields(
        variant_name: Ident,
        inner_type: Type,
        ctx_seed_fields: Vec<Ident>,
        state_field_names: std::collections::HashSet<String>,
    ) -> Self {
        Self {
            variant_name,
            inner_type,
            ctx_seed_fields,
            state_field_names,
        }
    }
}

/// Enhanced function that generates variants with ctx.* seed fields.
/// Now uses variant_name for enum variant naming and inner_type for type references.
pub fn compressed_account_variant_with_ctx_seeds(
    pda_ctx_seeds: &[PdaCtxSeedInfo],
) -> Result<TokenStream> {
    if pda_ctx_seeds.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "At least one account type must be specified",
        ));
    }

    // Phase 2: Generate struct variants with ctx.* seed fields
    // Uses variant_name for enum variant naming, inner_type for data field types
    let account_variants = pda_ctx_seeds.iter().map(|info| {
        let variant_name = &info.variant_name;
        // Qualify inner_type with crate:: to ensure it's accessible from generated code
        let inner_type = qualify_type_with_crate(&info.inner_type);
        let packed_variant_name = make_packed_variant_name(variant_name);
        // Create packed type (also qualified with crate::)
        let packed_inner_type =
            make_packed_type(&info.inner_type).expect("inner_type should be a valid type path");
        let ctx_fields = &info.ctx_seed_fields;

        // Unpacked variant: Pubkey fields for ctx.* seeds
        // Note: Use bare Pubkey which is in scope via `use anchor_lang::prelude::*`
        let unpacked_ctx_fields = ctx_fields.iter().map(|field| {
            quote! { #field: Pubkey }
        });

        // Packed variant: u8 index fields for ctx.* seeds
        let packed_ctx_fields = ctx_fields.iter().map(|field| {
            let idx_field = format_ident!("{}_idx", field);
            quote! { #idx_field: u8 }
        });

        quote! {
            #variant_name { data: #inner_type, #(#unpacked_ctx_fields,)* },
            #packed_variant_name { data: #packed_inner_type, #(#packed_ctx_fields,)* },
        }
    });

    // Phase 8: PackedCTokenData uses PackedTokenAccountVariant (with idx fields)
    //          CTokenData uses TokenAccountVariant (with Pubkey fields)
    let enum_def = quote! {
        #[derive(Clone, Debug, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
        pub enum RentFreeAccountVariant {
            #(#account_variants)*
            PackedCTokenData(light_token_sdk::compat::PackedCTokenData<PackedTokenAccountVariant>),
            CTokenData(light_token_sdk::compat::CTokenData<TokenAccountVariant>),
        }
    };

    let first = &pda_ctx_seeds[0];
    let first_variant = &first.variant_name;
    let first_type = qualify_type_with_crate(&first.inner_type);
    let first_ctx_fields = &first.ctx_seed_fields;
    let first_default_ctx_fields = first_ctx_fields.iter().map(|field| {
        quote! { #field: Pubkey::default() }
    });
    let default_impl = quote! {
        impl Default for RentFreeAccountVariant {
            fn default() -> Self {
                Self::#first_variant { data: #first_type::default(), #(#first_default_ctx_fields,)* }
            }
        }
    };

    let hash_match_arms = pda_ctx_seeds.iter().map(|info| {
        let variant_name = &info.variant_name;
        let inner_type = qualify_type_with_crate(&info.inner_type);
        let packed_variant_name = format_ident!("Packed{}", variant_name);
        quote! {
            RentFreeAccountVariant::#variant_name { data, .. } => <#inner_type as light_hasher::DataHasher>::hash::<H>(data),
            RentFreeAccountVariant::#packed_variant_name { .. } => unreachable!(),
        }
    });

    let data_hasher_impl = quote! {
        impl light_hasher::DataHasher for RentFreeAccountVariant {
            fn hash<H: light_hasher::Hasher>(&self) -> std::result::Result<[u8; 32], light_hasher::HasherError> {
                match self {
                    #(#hash_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }
        }
    };

    let light_discriminator_impl = quote! {
        impl light_sdk::LightDiscriminator for RentFreeAccountVariant {
            const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8];
            const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
        }
    };

    let compression_info_match_arms = pda_ctx_seeds.iter().map(|info| {
        let variant_name = &info.variant_name;
        let inner_type = qualify_type_with_crate(&info.inner_type);
        let packed_variant_name = format_ident!("Packed{}", variant_name);
        quote! {
            RentFreeAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::compressible::HasCompressionInfo>::compression_info(data),
            RentFreeAccountVariant::#packed_variant_name { .. } => unreachable!(),
        }
    });

    let compression_info_mut_match_arms = pda_ctx_seeds.iter().map(|info| {
        let variant_name = &info.variant_name;
        let inner_type = qualify_type_with_crate(&info.inner_type);
        let packed_variant_name = format_ident!("Packed{}", variant_name);
        quote! {
            RentFreeAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::compressible::HasCompressionInfo>::compression_info_mut(data),
            RentFreeAccountVariant::#packed_variant_name { .. } => unreachable!(),
        }
    });

    let compression_info_mut_opt_match_arms = pda_ctx_seeds.iter().map(|info| {
        let variant_name = &info.variant_name;
        let inner_type = qualify_type_with_crate(&info.inner_type);
        let packed_variant_name = format_ident!("Packed{}", variant_name);
        quote! {
            RentFreeAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::compressible::HasCompressionInfo>::compression_info_mut_opt(data),
            RentFreeAccountVariant::#packed_variant_name { .. } => unreachable!(),
        }
    });

    let set_compression_info_none_match_arms = pda_ctx_seeds.iter().map(|info| {
        let variant_name = &info.variant_name;
        let inner_type = qualify_type_with_crate(&info.inner_type);
        let packed_variant_name = format_ident!("Packed{}", variant_name);
        quote! {
            RentFreeAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::compressible::HasCompressionInfo>::set_compression_info_none(data),
            RentFreeAccountVariant::#packed_variant_name { .. } => unreachable!(),
        }
    });

    let has_compression_info_impl = quote! {
        impl light_sdk::compressible::HasCompressionInfo for RentFreeAccountVariant {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                match self {
                    #(#compression_info_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                match self {
                    #(#compression_info_mut_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
                match self {
                    #(#compression_info_mut_opt_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }

            fn set_compression_info_none(&mut self) {
                match self {
                    #(#set_compression_info_none_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }
        }
    };

    let size_match_arms = pda_ctx_seeds.iter().map(|info| {
        let variant_name = &info.variant_name;
        let inner_type = qualify_type_with_crate(&info.inner_type);
        let packed_variant_name = format_ident!("Packed{}", variant_name);
        quote! {
            RentFreeAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::account::Size>::size(data),
            RentFreeAccountVariant::#packed_variant_name { .. } => unreachable!(),
        }
    });

    let size_impl = quote! {
        impl light_sdk::account::Size for RentFreeAccountVariant {
            fn size(&self) -> usize {
                match self {
                    #(#size_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }
        }
    };

    // Phase 2: Pack/Unpack with ctx seed fields
    let pack_match_arms: Vec<_> = pda_ctx_seeds.iter().map(|info| {
        let variant_name = &info.variant_name;
        let inner_type = qualify_type_with_crate(&info.inner_type);
        let packed_variant_name = format_ident!("Packed{}", variant_name);
        let ctx_fields = &info.ctx_seed_fields;

        if ctx_fields.is_empty() {
            // No ctx seeds - simple pack
            quote! {
                RentFreeAccountVariant::#packed_variant_name { .. } => unreachable!(),
                RentFreeAccountVariant::#variant_name { data, .. } => RentFreeAccountVariant::#packed_variant_name {
                    data: <#inner_type as light_sdk::compressible::Pack>::pack(data, remaining_accounts),
                },
            }
        } else {
            // Has ctx seeds - pack data and ctx seed pubkeys
            let field_names: Vec<_> = ctx_fields.iter().collect();
            let idx_field_names: Vec<_> = ctx_fields.iter().map(|f| format_ident!("{}_idx", f)).collect();
            let pack_ctx_seeds: Vec<_> = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                // Dereference because we're matching on &self, so field is &Pubkey
                quote! { let #idx_field = remaining_accounts.insert_or_get(*#field); }
            }).collect();

            quote! {
                RentFreeAccountVariant::#packed_variant_name { .. } => unreachable!(),
                RentFreeAccountVariant::#variant_name { data, #(#field_names,)* .. } => {
                    #(#pack_ctx_seeds)*
                    RentFreeAccountVariant::#packed_variant_name {
                        data: <#inner_type as light_sdk::compressible::Pack>::pack(data, remaining_accounts),
                        #(#idx_field_names,)*
                    }
                },
            }
        }
    }).collect();

    let pack_impl = quote! {
        impl light_sdk::compressible::Pack for RentFreeAccountVariant {
            type Packed = Self;

            fn pack(&self, remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
                match self {
                    #(#pack_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(data) => {
                        // Use ctoken-sdk's Pack trait for CTokenData
                        Self::PackedCTokenData(light_token_sdk::pack::Pack::pack(data, remaining_accounts))
                    }
                }
            }
        }
    };

    let unpack_match_arms: Vec<_> = pda_ctx_seeds.iter().map(|info| {
        let variant_name = &info.variant_name;
        let inner_type = &info.inner_type;
        let packed_variant_name = make_packed_variant_name(variant_name);
        // Create packed type preserving full path (e.g., crate::module::PackedMyRecord)
        let packed_inner_type = make_packed_type(inner_type)
            .expect("inner_type should be a valid type path");
        let ctx_fields = &info.ctx_seed_fields;

        if ctx_fields.is_empty() {
            // No ctx seeds - simple unpack
            quote! {
                RentFreeAccountVariant::#packed_variant_name { data, .. } => Ok(RentFreeAccountVariant::#variant_name {
                    data: <#packed_inner_type as light_sdk::compressible::Unpack>::unpack(data, remaining_accounts)?,
                }),
                RentFreeAccountVariant::#variant_name { .. } => unreachable!(),
            }
        } else {
            // Has ctx seeds - unpack data and resolve ctx seed pubkeys from indices
            let idx_field_names: Vec<_> = ctx_fields.iter().map(|f| format_ident!("{}_idx", f)).collect();
            let field_names: Vec<_> = ctx_fields.iter().collect();
            let unpack_ctx_seeds: Vec<_> = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                quote! {
                    let #field = *remaining_accounts
                        .get(*#idx_field as usize)
                        .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                        .key;
                }
            }).collect();

            quote! {
                RentFreeAccountVariant::#packed_variant_name { data, #(#idx_field_names,)* .. } => {
                    #(#unpack_ctx_seeds)*
                    Ok(RentFreeAccountVariant::#variant_name {
                        data: <#packed_inner_type as light_sdk::compressible::Unpack>::unpack(data, remaining_accounts)?,
                        #(#field_names,)*
                    })
                },
                RentFreeAccountVariant::#variant_name { .. } => unreachable!(),
            }
        }
    }).collect();

    let unpack_impl = quote! {
        impl light_sdk::compressible::Unpack for RentFreeAccountVariant {
            type Unpacked = Self;

            fn unpack(
                &self,
                remaining_accounts: &[anchor_lang::prelude::AccountInfo],
            ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
                match self {
                    #(#unpack_match_arms)*
                    Self::PackedCTokenData(_) => {
                        // PackedCTokenData is handled separately in collect_pda_and_token
                        unreachable!("PackedCTokenData should not be unpacked through Unpack trait")
                    }
                    Self::CTokenData(_data) => unreachable!(),
                }
            }
        }
    };

    let rentfree_account_data_struct = quote! {
        #[derive(Clone, Debug, anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)]
        pub struct RentFreeAccountData {
            pub meta: light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
            pub data: RentFreeAccountVariant,
        }
    };

    let expanded = quote! {
        #enum_def
        #default_impl
        #data_hasher_impl
        #light_discriminator_impl
        #has_compression_info_impl
        #size_impl
        #pack_impl
        #unpack_impl
        #rentfree_account_data_struct
    };

    Ok(expanded)
}

// =============================================================================
// TOKEN ACCOUNT VARIANT
// =============================================================================

/// Extract ctx.* field names from seed elements (both token seeds and authority seeds).
///
/// Uses the visitor-based FieldExtractor for clean AST traversal.
pub fn extract_ctx_fields_from_token_spec(spec: &TokenSeedSpec) -> Vec<Ident> {
    const EXCLUDED: &[&str] = &[
        "fee_payer",
        "rent_sponsor",
        "config",
        "compression_authority",
    ];

    let mut all_fields = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for seed in spec.seeds.iter().chain(spec.authority.iter().flatten()) {
        if let SeedElement::Expression(expr) = seed {
            // Extract fields from this expression using the visitor
            let fields = super::visitors::FieldExtractor::ctx_fields(EXCLUDED).extract(expr);
            // Deduplicate across seeds
            for field in fields {
                let name = field.to_string();
                if seen.insert(name) {
                    all_fields.push(field);
                }
            }
        }
    }

    all_fields
}

/// Generate TokenAccountVariant and PackedTokenAccountVariant enums with Pack/Unpack impls.
pub fn generate_ctoken_account_variant_enum(token_seeds: &[TokenSeedSpec]) -> Result<TokenStream> {
    let unpacked_variants = token_seeds.iter().map(|spec| {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        let fields = ctx_fields.iter().map(|field| {
            quote! { #field: Pubkey }
        });

        if ctx_fields.is_empty() {
            quote! { #variant_name, }
        } else {
            quote! { #variant_name { #(#fields,)* }, }
        }
    });

    let packed_variants = token_seeds.iter().map(|spec| {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        let fields = ctx_fields.iter().map(|field| {
            let idx_field = format_ident!("{}_idx", field);
            quote! { #idx_field: u8 }
        });

        if ctx_fields.is_empty() {
            quote! { #variant_name, }
        } else {
            quote! { #variant_name { #(#fields,)* }, }
        }
    });

    let pack_arms = token_seeds.iter().map(|spec| {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        if ctx_fields.is_empty() {
            quote! {
                TokenAccountVariant::#variant_name => PackedTokenAccountVariant::#variant_name,
            }
        } else {
            let field_bindings: Vec<_> = ctx_fields.iter().collect();
            let idx_fields: Vec<_> = ctx_fields
                .iter()
                .map(|f| format_ident!("{}_idx", f))
                .collect();
            let pack_stmts: Vec<_> = ctx_fields
                .iter()
                .zip(idx_fields.iter())
                .map(|(field, idx)| {
                    quote! { let #idx = remaining_accounts.insert_or_get(*#field); }
                })
                .collect();

            quote! {
                TokenAccountVariant::#variant_name { #(#field_bindings,)* } => {
                    #(#pack_stmts)*
                    PackedTokenAccountVariant::#variant_name { #(#idx_fields,)* }
                }
            }
        }
    });

    let unpack_arms = token_seeds.iter().map(|spec| {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        if ctx_fields.is_empty() {
            quote! {
                PackedTokenAccountVariant::#variant_name => Ok(TokenAccountVariant::#variant_name),
            }
        } else {
            let idx_fields: Vec<_> = ctx_fields
                .iter()
                .map(|f| format_ident!("{}_idx", f))
                .collect();
            let unpack_stmts: Vec<_> = ctx_fields
                .iter()
                .zip(idx_fields.iter())
                .map(|(field, idx)| {
                    quote! {
                        let #field = *remaining_accounts
                            .get(*#idx as usize)
                            .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                            .key;
                    }
                })
                .collect();
            let field_names: Vec<_> = ctx_fields.iter().collect();

            quote! {
                PackedTokenAccountVariant::#variant_name { #(#idx_fields,)* } => {
                    #(#unpack_stmts)*
                    Ok(TokenAccountVariant::#variant_name { #(#field_names,)* })
                }
            }
        }
    });

    Ok(quote! {
        #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
        pub enum TokenAccountVariant {
            #(#unpacked_variants)*
        }

        #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
        pub enum PackedTokenAccountVariant {
            #(#packed_variants)*
        }

        impl light_token_sdk::pack::Pack for TokenAccountVariant {
            type Packed = PackedTokenAccountVariant;

            fn pack(&self, remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
                match self {
                    #(#pack_arms)*
                }
            }
        }

        impl light_token_sdk::pack::Unpack for PackedTokenAccountVariant {
            type Unpacked = TokenAccountVariant;

            fn unpack(
                &self,
                remaining_accounts: &[solana_account_info::AccountInfo],
            ) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
                match self {
                    #(#unpack_arms)*
                }
            }
        }

        impl light_sdk::compressible::IntoCTokenVariant<RentFreeAccountVariant, light_token_sdk::compat::TokenData> for TokenAccountVariant {
            fn into_ctoken_variant(self, token_data: light_token_sdk::compat::TokenData) -> RentFreeAccountVariant {
                RentFreeAccountVariant::CTokenData(light_token_sdk::compat::CTokenData {
                    variant: self,
                    token_data,
                })
            }
        }
    })
}
