use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result, Type};

use super::parsing::{SeedElement, TokenSeedSpec};
use crate::light_pdas::shared_utils::{
    make_packed_type, make_packed_variant_name, qualify_type_with_crate,
};

// =============================================================================
// RENTFREE VARIANT BUILDER
// =============================================================================

/// Builder for generating `LightAccountVariant` enum and its trait implementations.
///
/// Encapsulates the PDA context seed info and configuration needed to generate
/// all variant-related code: enum definition, trait impls, and wrapper struct.
pub(super) struct LightVariantBuilder<'a> {
    /// PDA context seed info for each account type.
    pda_ctx_seeds: &'a [PdaCtxSeedInfo],
    /// Whether to include CToken variants in the generated enum.
    include_ctoken: bool,
}

impl<'a> LightVariantBuilder<'a> {
    /// Create a new LightVariantBuilder with the given PDA context seeds.
    ///
    /// # Arguments
    /// * `pda_ctx_seeds` - PDA context seed info for each account type
    ///
    /// # Returns
    /// A new LightVariantBuilder instance
    pub fn new(pda_ctx_seeds: &'a [PdaCtxSeedInfo]) -> Self {
        Self {
            pda_ctx_seeds,
            include_ctoken: true, // Default to including CToken variants
        }
    }

    /// Validate the builder configuration.
    ///
    /// Checks that at least one account type is provided.
    ///
    /// # Returns
    /// `Ok(())` if validation passes, or a `syn::Error` describing the issue.
    pub fn validate(&self) -> Result<()> {
        if self.pda_ctx_seeds.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[rentfree_program] requires at least one Accounts struct with \
                 #[light_account(init)] fields.\n\n\
                 Make sure your program has:\n\
                 1. An Accounts struct with #[derive(Accounts, LightAccounts)]\n\
                 2. At least one field marked with #[light_account(init)]\n\n\
                 Example:\n\
                 #[derive(Accounts, LightAccounts)]\n\
                 #[instruction(params: MyParams)]\n\
                 pub struct MyAccounts<'info> {\n    \
                     #[account(init, ...)]\n    \
                     #[light_account(init)]\n    \
                     pub my_account: Account<'info, MyData>,\n\
                 }",
            ));
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Code Generation Methods
    // -------------------------------------------------------------------------

    /// Generate the complete enum and all trait implementations.
    ///
    /// This is the main entry point that combines all generated code pieces.
    pub fn build(&self) -> Result<TokenStream> {
        self.validate()?;

        let enum_def = self.generate_enum_def()?;
        let default_impl = self.generate_default_impl();
        let data_hasher_impl = self.generate_data_hasher_impl();
        let light_discriminator_impl = self.generate_light_discriminator_impl();
        let has_compression_info_impl = self.generate_has_compression_info_impl();
        let size_impl = self.generate_size_impl();
        let pack_impl = self.generate_pack_impl();
        let unpack_impl = self.generate_unpack_impl()?;
        let light_account_data_struct = self.generate_light_account_data_struct();

        Ok(quote! {
            #enum_def
            #default_impl
            #data_hasher_impl
            #light_discriminator_impl
            #has_compression_info_impl
            #size_impl
            #pack_impl
            #unpack_impl
            #light_account_data_struct
        })
    }

    /// Generate the enum definition with all variants.
    fn generate_enum_def(&self) -> Result<TokenStream> {
        let mut account_variants_tokens = Vec::new();
        for info in self.pda_ctx_seeds.iter() {
            let variant_name = &info.variant_name;
            let inner_type = qualify_type_with_crate(&info.inner_type);
            let packed_variant_name = make_packed_variant_name(variant_name);
            let packed_inner_type = make_packed_type(&info.inner_type).ok_or_else(|| {
                syn::Error::new_spanned(&info.inner_type, "invalid type path for packed type")
            })?;
            let ctx_fields = &info.ctx_seed_fields;
            let params_only_fields = &info.params_only_seed_fields;

            let unpacked_ctx_fields = ctx_fields.iter().map(|field| {
                quote! { #field: Pubkey }
            });
            let unpacked_params_fields = params_only_fields.iter().map(|(field, ty, _)| {
                quote! { #field: #ty }
            });

            let packed_ctx_fields = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                quote! { #idx_field: u8 }
            });
            let packed_params_fields = params_only_fields.iter().map(|(field, ty, _)| {
                quote! { #field: #ty }
            });

            account_variants_tokens.push(quote! {
                #variant_name { data: #inner_type, #(#unpacked_ctx_fields,)* #(#unpacked_params_fields,)* },
                #packed_variant_name { data: #packed_inner_type, #(#packed_ctx_fields,)* #(#packed_params_fields,)* },
            });
        }

        let ctoken_variants = if self.include_ctoken {
            quote! {
                PackedCTokenData(light_token_sdk::compat::PackedCTokenData<PackedTokenAccountVariant>),
                CTokenData(light_token_sdk::compat::CTokenData<TokenAccountVariant>),
            }
        } else {
            quote! {}
        };

        Ok(quote! {
            #[derive(Clone, Debug, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
            pub enum LightAccountVariant {
                #(#account_variants_tokens)*
                #ctoken_variants
            }
        })
    }

    /// Generate the Default implementation.
    fn generate_default_impl(&self) -> TokenStream {
        let first = &self.pda_ctx_seeds[0];
        let first_variant = &first.variant_name;
        let first_type = qualify_type_with_crate(&first.inner_type);
        let first_ctx_fields = &first.ctx_seed_fields;
        let first_params_only_fields = &first.params_only_seed_fields;

        let first_default_ctx_fields = first_ctx_fields.iter().map(|field| {
            quote! { #field: Pubkey::default() }
        });
        let first_default_params_fields = first_params_only_fields.iter().map(|(field, ty, _)| {
            quote! { #field: <#ty as Default>::default() }
        });

        quote! {
            impl Default for LightAccountVariant {
                fn default() -> Self {
                    Self::#first_variant { data: #first_type::default(), #(#first_default_ctx_fields,)* #(#first_default_params_fields,)* }
                }
            }
        }
    }

    /// Generate the DataHasher implementation.
    fn generate_data_hasher_impl(&self) -> TokenStream {
        let hash_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let inner_type = qualify_type_with_crate(&info.inner_type);
            let packed_variant_name = format_ident!("Packed{}", variant_name);
            quote! {
                LightAccountVariant::#variant_name { data, .. } => <#inner_type as ::light_sdk::hasher::DataHasher>::hash::<H>(data),
                LightAccountVariant::#packed_variant_name { .. } => Err(::light_sdk::hasher::HasherError::EmptyInput),
            }
        });

        let ctoken_arms = if self.include_ctoken {
            quote! {
                Self::PackedCTokenData(_) => Err(::light_sdk::hasher::HasherError::EmptyInput),
                Self::CTokenData(_) => Err(::light_sdk::hasher::HasherError::EmptyInput),
            }
        } else {
            quote! {}
        };

        quote! {
            impl ::light_sdk::hasher::DataHasher for LightAccountVariant {
                fn hash<H: ::light_sdk::hasher::Hasher>(&self) -> std::result::Result<[u8; 32], ::light_sdk::hasher::HasherError> {
                    match self {
                        #(#hash_match_arms)*
                        #ctoken_arms
                    }
                }
            }
        }
    }

    /// Generate the LightDiscriminator implementation.
    fn generate_light_discriminator_impl(&self) -> TokenStream {
        quote! {
            impl light_sdk::LightDiscriminator for LightAccountVariant {
                const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8];
                const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
            }
        }
    }

    /// Generate the HasCompressionInfo implementation.
    fn generate_has_compression_info_impl(&self) -> TokenStream {
        let compression_info_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let inner_type = qualify_type_with_crate(&info.inner_type);
            let packed_variant_name = format_ident!("Packed{}", variant_name);
            quote! {
                LightAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::interface::HasCompressionInfo>::compression_info(data),
                LightAccountVariant::#packed_variant_name { .. } => Err(light_sdk::error::LightSdkError::PackedVariantCompressionInfo.into()),
            }
        });

        let compression_info_mut_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let inner_type = qualify_type_with_crate(&info.inner_type);
            let packed_variant_name = format_ident!("Packed{}", variant_name);
            quote! {
                LightAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::interface::HasCompressionInfo>::compression_info_mut(data),
                LightAccountVariant::#packed_variant_name { .. } => Err(light_sdk::error::LightSdkError::PackedVariantCompressionInfo.into()),
            }
        });

        let compression_info_mut_opt_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let inner_type = qualify_type_with_crate(&info.inner_type);
            let packed_variant_name = format_ident!("Packed{}", variant_name);
            quote! {
                LightAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::interface::HasCompressionInfo>::compression_info_mut_opt(data),
                LightAccountVariant::#packed_variant_name { .. } => panic!("compression_info_mut_opt not supported on packed variants"),
            }
        });

        let set_compression_info_none_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let inner_type = qualify_type_with_crate(&info.inner_type);
            let packed_variant_name = format_ident!("Packed{}", variant_name);
            quote! {
                LightAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::interface::HasCompressionInfo>::set_compression_info_none(data),
                LightAccountVariant::#packed_variant_name { .. } => Err(light_sdk::error::LightSdkError::PackedVariantCompressionInfo.into()),
            }
        });

        let ctoken_arms = if self.include_ctoken {
            quote! {
                Self::PackedCTokenData(_) | Self::CTokenData(_) => Err(light_sdk::error::LightSdkError::CTokenCompressionInfo.into()),
            }
        } else {
            quote! {}
        };

        let ctoken_arms_mut_opt = if self.include_ctoken {
            quote! {
                Self::PackedCTokenData(_) | Self::CTokenData(_) => panic!("compression_info_mut_opt not supported on CToken variants"),
            }
        } else {
            quote! {}
        };

        quote! {
            impl light_sdk::interface::HasCompressionInfo for LightAccountVariant {
                fn compression_info(&self) -> std::result::Result<&light_sdk::interface::CompressionInfo, solana_program_error::ProgramError> {
                    match self {
                        #(#compression_info_match_arms)*
                        #ctoken_arms
                    }
                }

                fn compression_info_mut(&mut self) -> std::result::Result<&mut light_sdk::interface::CompressionInfo, solana_program_error::ProgramError> {
                    match self {
                        #(#compression_info_mut_match_arms)*
                        #ctoken_arms
                    }
                }

                fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::interface::CompressionInfo> {
                    match self {
                        #(#compression_info_mut_opt_match_arms)*
                        #ctoken_arms_mut_opt
                    }
                }

                fn set_compression_info_none(&mut self) -> std::result::Result<(), solana_program_error::ProgramError> {
                    match self {
                        #(#set_compression_info_none_match_arms)*
                        #ctoken_arms
                    }
                }
            }
        }
    }

    /// Generate the Size implementation.
    fn generate_size_impl(&self) -> TokenStream {
        let size_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let inner_type = qualify_type_with_crate(&info.inner_type);
            let packed_variant_name = format_ident!("Packed{}", variant_name);
            quote! {
                LightAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::account::Size>::size(data),
                LightAccountVariant::#packed_variant_name { .. } => Err(solana_program_error::ProgramError::InvalidAccountData),
            }
        });

        let ctoken_arms = if self.include_ctoken {
            quote! {
                Self::PackedCTokenData(_) => Err(solana_program_error::ProgramError::InvalidAccountData),
                Self::CTokenData(_) => Err(solana_program_error::ProgramError::InvalidAccountData),
            }
        } else {
            quote! {}
        };

        quote! {
            impl light_sdk::account::Size for LightAccountVariant {
                fn size(&self) -> std::result::Result<usize, solana_program_error::ProgramError> {
                    match self {
                        #(#size_match_arms)*
                        #ctoken_arms
                    }
                }
            }
        }
    }

    /// Generate the Pack implementation.
    fn generate_pack_impl(&self) -> TokenStream {
        let pack_match_arms: Vec<_> = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let inner_type = qualify_type_with_crate(&info.inner_type);
            let packed_variant_name = format_ident!("Packed{}", variant_name);
            let ctx_fields = &info.ctx_seed_fields;
            let params_only_fields = &info.params_only_seed_fields;

            let ctx_field_names: Vec<_> = ctx_fields.iter().collect();
            let idx_field_names: Vec<_> = ctx_fields.iter().map(|f| format_ident!("{}_idx", f)).collect();
            let pack_ctx_seeds: Vec<_> = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                quote! { let #idx_field = remaining_accounts.insert_or_get(*#field); }
            }).collect();

            let params_field_names: Vec<_> = params_only_fields.iter().map(|(f, _, _)| f).collect();

            if ctx_fields.is_empty() && params_only_fields.is_empty() {
                quote! {
                    LightAccountVariant::#packed_variant_name { .. } => Err(solana_program_error::ProgramError::InvalidAccountData),
                    LightAccountVariant::#variant_name { data, .. } => Ok(LightAccountVariant::#packed_variant_name {
                        data: <#inner_type as light_sdk::interface::Pack>::pack(data, remaining_accounts)?,
                    }),
                }
            } else {
                quote! {
                    LightAccountVariant::#packed_variant_name { .. } => Err(solana_program_error::ProgramError::InvalidAccountData),
                    LightAccountVariant::#variant_name { data, #(#ctx_field_names,)* #(#params_field_names,)* .. } => {
                        #(#pack_ctx_seeds)*
                        Ok(LightAccountVariant::#packed_variant_name {
                            data: <#inner_type as light_sdk::interface::Pack>::pack(data, remaining_accounts)?,
                            #(#idx_field_names,)*
                            #(#params_field_names: *#params_field_names,)*
                        })
                    },
                }
            }
        }).collect();

        let ctoken_arms = if self.include_ctoken {
            quote! {
                Self::PackedCTokenData(_) => Err(solana_program_error::ProgramError::InvalidAccountData),
                Self::CTokenData(data) => {
                    Ok(Self::PackedCTokenData(light_token_sdk::pack::Pack::pack(data, remaining_accounts)?))
                }
            }
        } else {
            quote! {}
        };

        quote! {
            impl light_sdk::interface::Pack for LightAccountVariant {
                type Packed = Self;

                fn pack(&self, remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> std::result::Result<Self::Packed, solana_program_error::ProgramError> {
                    match self {
                        #(#pack_match_arms)*
                        #ctoken_arms
                    }
                }
            }
        }
    }

    /// Generate the Unpack implementation.
    fn generate_unpack_impl(&self) -> Result<TokenStream> {
        let mut unpack_match_arms = Vec::new();
        for info in self.pda_ctx_seeds.iter() {
            let variant_name = &info.variant_name;
            let inner_type = &info.inner_type;
            let packed_variant_name = make_packed_variant_name(variant_name);
            let packed_inner_type = make_packed_type(inner_type).ok_or_else(|| {
                syn::Error::new_spanned(inner_type, "invalid type path for packed type")
            })?;
            let ctx_fields = &info.ctx_seed_fields;
            let params_only_fields = &info.params_only_seed_fields;

            let idx_field_names: Vec<_> = ctx_fields
                .iter()
                .map(|f| format_ident!("{}_idx", f))
                .collect();
            let ctx_field_names: Vec<_> = ctx_fields.iter().collect();
            let unpack_ctx_seeds: Vec<_> = ctx_fields
                .iter()
                .map(|field| {
                    let idx_field = format_ident!("{}_idx", field);
                    quote! {
                        let #field = *remaining_accounts
                            .get(*#idx_field as usize)
                            .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                            .key;
                    }
                })
                .collect();

            let params_field_names: Vec<_> = params_only_fields.iter().map(|(f, _, _)| f).collect();

            if ctx_fields.is_empty() && params_only_fields.is_empty() {
                unpack_match_arms.push(quote! {
                    LightAccountVariant::#packed_variant_name { data, .. } => Ok(LightAccountVariant::#variant_name {
                        data: <#packed_inner_type as light_sdk::interface::Unpack>::unpack(data, remaining_accounts)?,
                    }),
                    LightAccountVariant::#variant_name { .. } => Err(solana_program_error::ProgramError::InvalidAccountData),
                });
            } else {
                unpack_match_arms.push(quote! {
                    LightAccountVariant::#packed_variant_name { data, #(#idx_field_names,)* #(#params_field_names,)* .. } => {
                        #(#unpack_ctx_seeds)*
                        Ok(LightAccountVariant::#variant_name {
                            data: <#packed_inner_type as light_sdk::interface::Unpack>::unpack(data, remaining_accounts)?,
                            #(#ctx_field_names,)*
                            #(#params_field_names: *#params_field_names,)*
                        })
                    },
                    LightAccountVariant::#variant_name { .. } => Err(solana_program_error::ProgramError::InvalidAccountData),
                });
            }
        }

        let ctoken_arms = if self.include_ctoken {
            quote! {
                Self::PackedCTokenData(_) => Err(solana_program_error::ProgramError::InvalidAccountData),
                Self::CTokenData(_data) => Err(solana_program_error::ProgramError::InvalidAccountData),
            }
        } else {
            quote! {}
        };

        Ok(quote! {
            impl light_sdk::interface::Unpack for LightAccountVariant {
                type Unpacked = Self;

                fn unpack(
                    &self,
                    remaining_accounts: &[anchor_lang::prelude::AccountInfo],
                ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
                    match self {
                        #(#unpack_match_arms)*
                        #ctoken_arms
                    }
                }
            }
        })
    }

    /// Generate the LightAccountData struct.
    fn generate_light_account_data_struct(&self) -> TokenStream {
        quote! {
            #[derive(Clone, Debug, anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)]
            pub struct LightAccountData {
                pub meta: light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                pub data: LightAccountVariant,
            }
        }
    }
}

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
    /// Params-only seed fields (name, type, has_conversion) - seeds from params.* that don't exist on state
    /// The bool indicates whether a conversion method like to_le_bytes() is applied
    pub params_only_seed_fields: Vec<(Ident, Type, bool)>,
}

impl PdaCtxSeedInfo {
    pub fn with_state_fields(
        variant_name: Ident,
        inner_type: Type,
        ctx_seed_fields: Vec<Ident>,
        state_field_names: std::collections::HashSet<String>,
        params_only_seed_fields: Vec<(Ident, Type, bool)>,
    ) -> Self {
        Self {
            variant_name,
            inner_type,
            ctx_seed_fields,
            state_field_names,
            params_only_seed_fields,
        }
    }
}

// =============================================================================
// TOKEN VARIANT BUILDER
// =============================================================================

/// Builder for generating `TokenAccountVariant` and `PackedTokenAccountVariant` enums.
///
/// Encapsulates the token seed specifications needed to generate
/// all token variant-related code: enum definitions, Pack/Unpack impls, and IntoCTokenVariant.
pub(super) struct TokenVariantBuilder<'a> {
    /// Token seed specifications for each token variant.
    token_seeds: &'a [TokenSeedSpec],
}

impl<'a> TokenVariantBuilder<'a> {
    /// Create a new TokenVariantBuilder with the given token seeds.
    ///
    /// # Arguments
    /// * `token_seeds` - Token seed specifications for each variant
    ///
    /// # Returns
    /// A new TokenVariantBuilder instance
    pub fn new(token_seeds: &'a [TokenSeedSpec]) -> Self {
        Self { token_seeds }
    }

    // -------------------------------------------------------------------------
    // Code Generation Methods
    // -------------------------------------------------------------------------

    /// Generate the complete token variant code.
    ///
    /// This is the main entry point that combines all generated code pieces.
    pub fn build(&self) -> Result<TokenStream> {
        let unpacked_enum = self.generate_unpacked_enum();
        let packed_enum = self.generate_packed_enum();
        let pack_impl = self.generate_pack_impl();
        let unpack_impl = self.generate_unpack_impl();
        let into_ctoken_variant_impl = self.generate_into_ctoken_variant_impl();

        Ok(quote! {
            #unpacked_enum
            #packed_enum
            #pack_impl
            #unpack_impl
            #into_ctoken_variant_impl
        })
    }

    /// Generate the unpacked TokenAccountVariant enum.
    fn generate_unpacked_enum(&self) -> TokenStream {
        let variants = self.token_seeds.iter().map(|spec| {
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

        quote! {
            #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
            pub enum TokenAccountVariant {
                #(#variants)*
            }
        }
    }

    /// Generate the packed PackedTokenAccountVariant enum.
    fn generate_packed_enum(&self) -> TokenStream {
        let variants = self.token_seeds.iter().map(|spec| {
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

        quote! {
            #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
            pub enum PackedTokenAccountVariant {
                #(#variants)*
            }
        }
    }

    /// Generate the Pack implementation for TokenAccountVariant.
    fn generate_pack_impl(&self) -> TokenStream {
        let arms = self.token_seeds.iter().map(|spec| {
            let variant_name = &spec.variant;
            let ctx_fields = extract_ctx_fields_from_token_spec(spec);

            if ctx_fields.is_empty() {
                quote! {
                    TokenAccountVariant::#variant_name => Ok(PackedTokenAccountVariant::#variant_name),
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
                        Ok(PackedTokenAccountVariant::#variant_name { #(#idx_fields,)* })
                    }
                }
            }
        });

        quote! {
            impl light_token_sdk::pack::Pack for TokenAccountVariant {
                type Packed = PackedTokenAccountVariant;

                fn pack(&self, remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> std::result::Result<Self::Packed, solana_program_error::ProgramError> {
                    match self {
                        #(#arms)*
                    }
                }
            }
        }
    }

    /// Generate the Unpack implementation for PackedTokenAccountVariant.
    fn generate_unpack_impl(&self) -> TokenStream {
        let arms = self.token_seeds.iter().map(|spec| {
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

        quote! {
            impl light_token_sdk::pack::Unpack for PackedTokenAccountVariant {
                type Unpacked = TokenAccountVariant;

                fn unpack(
                    &self,
                    remaining_accounts: &[solana_account_info::AccountInfo],
                ) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
                    match self {
                        #(#arms)*
                    }
                }
            }
        }
    }

    /// Generate the IntoCTokenVariant implementation.
    fn generate_into_ctoken_variant_impl(&self) -> TokenStream {
        quote! {
            impl light_sdk::interface::IntoCTokenVariant<LightAccountVariant, light_token_sdk::compat::TokenData> for TokenAccountVariant {
                fn into_ctoken_variant(self, token_data: light_token_sdk::compat::TokenData) -> LightAccountVariant {
                    LightAccountVariant::CTokenData(light_token_sdk::compat::CTokenData {
                        variant: self,
                        token_data,
                    })
                }
            }
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS
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
