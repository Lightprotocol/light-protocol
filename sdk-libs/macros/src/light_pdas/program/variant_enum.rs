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
                "#[light_program] requires at least one Accounts struct with \
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

        let packed_data_structs = self.generate_packed_data_structs()?;
        let enum_def = self.generate_enum_def()?;
        let default_impl = self.generate_default_impl();
        let data_hasher_impl = self.generate_data_hasher_impl();
        let light_discriminator_impl = self.generate_light_discriminator_impl();
        let has_compression_info_impl = self.generate_has_compression_info_impl();
        let size_impl = self.generate_size_impl();
        let pack_impl = self.generate_pack_impl();
        let unpack_impl = self.generate_unpack_impl()?;
        let light_account_data_struct = self.generate_light_account_data_struct();
        let decompressible_impls = self.generate_decompressible_account_impls()?;
        let decompressible_enum_impl = self.generate_decompressible_account_enum_impl();

        Ok(quote! {
            #packed_data_structs
            #enum_def
            #default_impl
            #data_hasher_impl
            #light_discriminator_impl
            #has_compression_info_impl
            #size_impl
            #pack_impl
            #unpack_impl
            #light_account_data_struct
            #decompressible_impls
            #decompressible_enum_impl
        })
    }

    /// Generate PackedXxxData structs for each account type.
    ///
    /// These structs wrap the packed data and seed indices, and implement
    /// `DecompressibleAccount` for simple dispatch.
    ///
    /// For zero_copy accounts, the data field is `Vec<u8>` instead of a packed type,
    /// since Pod types don't need Pubkey-to-index packing (they use `[u8; 32]` directly).
    fn generate_packed_data_structs(&self) -> Result<TokenStream> {
        let mut structs = Vec::new();

        for info in self.pda_ctx_seeds.iter() {
            let variant_name = &info.variant_name;
            let packed_data_struct_name = format_ident!("Packed{}Data", variant_name);
            let ctx_fields = &info.ctx_seed_fields;
            let params_only_fields = &info.params_only_seed_fields;

            // For zero_copy accounts, use Vec<u8> as the data type since Pod types
            // don't need packing (they already use [u8; 32] instead of Pubkey)
            let data_field_type = if info.is_zero_copy {
                quote! { Vec<u8> }
            } else {
                let packed_inner_type = make_packed_type(&info.inner_type).ok_or_else(|| {
                    syn::Error::new_spanned(&info.inner_type, "invalid type path for packed type")
                })?;
                quote! { #packed_inner_type }
            };

            // Generate struct fields
            let idx_fields = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                quote! { pub #idx_field: u8 }
            });
            let params_fields = params_only_fields.iter().map(|(field, ty, _)| {
                quote! { pub #field: #ty }
            });

            structs.push(quote! {
                /// Packed data struct for #variant_name, wrapping packed data and seed indices.
                #[derive(Clone, Debug, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
                pub struct #packed_data_struct_name {
                    pub data: #data_field_type,
                    #(#idx_fields,)*
                    #(#params_fields,)*
                }
            });
        }

        Ok(quote! { #(#structs)* })
    }

    /// Generate the enum definition with all variants.
    ///
    /// Packed variants now wrap PackedXxxData structs for simplified dispatch.
    /// For zero_copy accounts, the unpacked variant stores `Vec<u8>` instead of the inner type,
    /// since Pod types don't implement Borsh serialization required by the enum's derives.
    fn generate_enum_def(&self) -> Result<TokenStream> {
        let mut account_variants_tokens = Vec::new();
        for info in self.pda_ctx_seeds.iter() {
            let variant_name = &info.variant_name;
            let packed_variant_name = make_packed_variant_name(variant_name);
            let packed_data_struct_name = format_ident!("Packed{}Data", variant_name);
            let ctx_fields = &info.ctx_seed_fields;
            let params_only_fields = &info.params_only_seed_fields;

            let unpacked_ctx_fields = ctx_fields.iter().map(|field| {
                quote! { #field: Pubkey }
            });
            let unpacked_params_fields = params_only_fields.iter().map(|(field, ty, _)| {
                quote! { #field: #ty }
            });

            // For zero_copy accounts, store data as Vec<u8> since Pod types don't implement Borsh
            if info.is_zero_copy {
                account_variants_tokens.push(quote! {
                    #variant_name { data: Vec<u8>, #(#unpacked_ctx_fields,)* #(#unpacked_params_fields,)* },
                    #packed_variant_name(#packed_data_struct_name),
                });
            } else {
                let inner_type = qualify_type_with_crate(&info.inner_type);
                account_variants_tokens.push(quote! {
                    #variant_name { data: #inner_type, #(#unpacked_ctx_fields,)* #(#unpacked_params_fields,)* },
                    #packed_variant_name(#packed_data_struct_name),
                });
            }
        }

        let ctoken_variants = if self.include_ctoken {
            quote! {
                PackedCTokenData(light_token::compat::PackedCTokenData<PackedTokenAccountVariant>),
                CTokenData(light_token::compat::CTokenData<TokenAccountVariant>),
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
    ///
    /// For zero_copy accounts, defaults to an empty Vec<u8> since the unpacked variant stores bytes.
    fn generate_default_impl(&self) -> TokenStream {
        let first = &self.pda_ctx_seeds[0];
        let first_variant = &first.variant_name;
        let first_ctx_fields = &first.ctx_seed_fields;
        let first_params_only_fields = &first.params_only_seed_fields;

        let first_default_ctx_fields = first_ctx_fields.iter().map(|field| {
            quote! { #field: Pubkey::default() }
        });
        let first_default_params_fields = first_params_only_fields.iter().map(|(field, ty, _)| {
            quote! { #field: <#ty as Default>::default() }
        });

        // For zero_copy accounts, use empty Vec<u8> as default
        let data_default = if first.is_zero_copy {
            quote! { Vec::new() }
        } else {
            let first_type = qualify_type_with_crate(&first.inner_type);
            quote! { #first_type::default() }
        };

        quote! {
            impl Default for LightAccountVariant {
                fn default() -> Self {
                    Self::#first_variant { data: #data_default, #(#first_default_ctx_fields,)* #(#first_default_params_fields,)* }
                }
            }
        }
    }

    /// Generate the DataHasher implementation.
    ///
    /// Packed variants now use tuple syntax.
    /// For zero_copy accounts, the unpacked variant stores `Vec<u8>`, so we hash the bytes directly.
    fn generate_data_hasher_impl(&self) -> TokenStream {
        let hash_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let packed_variant_name = format_ident!("Packed{}", variant_name);

            // For zero_copy accounts, hash the raw bytes since data is Vec<u8>
            if info.is_zero_copy {
                quote! {
                    LightAccountVariant::#variant_name { data, .. } => H::hashv(&[data.as_slice()]),
                    LightAccountVariant::#packed_variant_name(_) => Err(::light_sdk::hasher::HasherError::EmptyInput),
                }
            } else {
                let inner_type = qualify_type_with_crate(&info.inner_type);
                quote! {
                    LightAccountVariant::#variant_name { data, .. } => <#inner_type as ::light_sdk::hasher::DataHasher>::hash::<H>(data),
                    LightAccountVariant::#packed_variant_name(_) => Err(::light_sdk::hasher::HasherError::EmptyInput),
                }
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
    ///
    /// Packed variants now use tuple syntax.
    /// For zero_copy accounts, the unpacked variant stores `Vec<u8>` and cannot implement
    /// HasCompressionInfo trait methods, so we return errors for those variants.
    fn generate_has_compression_info_impl(&self) -> TokenStream {
        let compression_info_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let packed_variant_name = format_ident!("Packed{}", variant_name);

            // For zero_copy accounts, unpacked variant stores Vec<u8> - cannot access compression info
            if info.is_zero_copy {
                quote! {
                    LightAccountVariant::#variant_name { .. } => Err(light_sdk::error::LightSdkError::ZeroCopyUnpackedVariant.into()),
                    LightAccountVariant::#packed_variant_name(_) => Err(light_sdk::error::LightSdkError::PackedVariantCompressionInfo.into()),
                }
            } else {
                let inner_type = qualify_type_with_crate(&info.inner_type);
                quote! {
                    LightAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::interface::HasCompressionInfo>::compression_info(data),
                    LightAccountVariant::#packed_variant_name(_) => Err(light_sdk::error::LightSdkError::PackedVariantCompressionInfo.into()),
                }
            }
        });

        let compression_info_mut_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let packed_variant_name = format_ident!("Packed{}", variant_name);

            if info.is_zero_copy {
                quote! {
                    LightAccountVariant::#variant_name { .. } => Err(light_sdk::error::LightSdkError::ZeroCopyUnpackedVariant.into()),
                    LightAccountVariant::#packed_variant_name(_) => Err(light_sdk::error::LightSdkError::PackedVariantCompressionInfo.into()),
                }
            } else {
                let inner_type = qualify_type_with_crate(&info.inner_type);
                quote! {
                    LightAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::interface::HasCompressionInfo>::compression_info_mut(data),
                    LightAccountVariant::#packed_variant_name(_) => Err(light_sdk::error::LightSdkError::PackedVariantCompressionInfo.into()),
                }
            }
        });

        let compression_info_mut_opt_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let packed_variant_name = format_ident!("Packed{}", variant_name);

            if info.is_zero_copy {
                quote! {
                    LightAccountVariant::#variant_name { .. } => panic!("compression_info_mut_opt not supported on zero_copy unpacked variants"),
                    LightAccountVariant::#packed_variant_name(_) => panic!("compression_info_mut_opt not supported on packed variants"),
                }
            } else {
                let inner_type = qualify_type_with_crate(&info.inner_type);
                quote! {
                    LightAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::interface::HasCompressionInfo>::compression_info_mut_opt(data),
                    LightAccountVariant::#packed_variant_name(_) => panic!("compression_info_mut_opt not supported on packed variants"),
                }
            }
        });

        let set_compression_info_none_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let packed_variant_name = format_ident!("Packed{}", variant_name);

            if info.is_zero_copy {
                quote! {
                    LightAccountVariant::#variant_name { .. } => Err(light_sdk::error::LightSdkError::ZeroCopyUnpackedVariant.into()),
                    LightAccountVariant::#packed_variant_name(_) => Err(light_sdk::error::LightSdkError::PackedVariantCompressionInfo.into()),
                }
            } else {
                let inner_type = qualify_type_with_crate(&info.inner_type);
                quote! {
                    LightAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::interface::HasCompressionInfo>::set_compression_info_none(data),
                    LightAccountVariant::#packed_variant_name(_) => Err(light_sdk::error::LightSdkError::PackedVariantCompressionInfo.into()),
                }
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
    ///
    /// Packed variants now use tuple syntax.
    /// For zero_copy accounts, the unpacked variant stores `Vec<u8>` so we return its length.
    fn generate_size_impl(&self) -> TokenStream {
        let size_match_arms = self.pda_ctx_seeds.iter().map(|info| {
            let variant_name = &info.variant_name;
            let packed_variant_name = format_ident!("Packed{}", variant_name);

            // For zero_copy accounts, return the Vec length
            if info.is_zero_copy {
                quote! {
                    LightAccountVariant::#variant_name { data, .. } => Ok(data.len()),
                    LightAccountVariant::#packed_variant_name(_) => Err(solana_program_error::ProgramError::InvalidAccountData),
                }
            } else {
                let inner_type = qualify_type_with_crate(&info.inner_type);
                quote! {
                    LightAccountVariant::#variant_name { data, .. } => <#inner_type as light_sdk::account::Size>::size(data),
                    LightAccountVariant::#packed_variant_name(_) => Err(solana_program_error::ProgramError::InvalidAccountData),
                }
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
    ///
    /// Packed variants now use tuple syntax wrapping PackedXxxData structs.
    /// For zero_copy accounts, the unpacked variant stores `Vec<u8>` and packing from
    /// unpacked is not supported (returns error).
    fn generate_pack_impl(&self) -> TokenStream {
        let pack_match_arms: Vec<_> = self
            .pda_ctx_seeds
            .iter()
            .map(|info| {
                let seeds =
                    SeedFieldCollection::new(&info.ctx_seed_fields, &info.params_only_seed_fields);
                generate_pack_match_arm(info, &seeds)
            })
            .collect();

        let ctoken_arms = if self.include_ctoken {
            quote! {
                Self::PackedCTokenData(_) => Err(solana_program_error::ProgramError::InvalidAccountData),
                Self::CTokenData(data) => {
                    Ok(Self::PackedCTokenData(light_token::pack::Pack::pack(data, remaining_accounts)?))
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
    ///
    /// Packed variants now use tuple syntax - access inner struct fields via `inner.field`.
    /// For zero_copy accounts, the unpacked variant stores `Vec<u8>` containing the Pod bytes.
    fn generate_unpack_impl(&self) -> Result<TokenStream> {
        let mut unpack_match_arms = Vec::new();
        for info in self.pda_ctx_seeds.iter() {
            let seeds =
                SeedFieldCollection::new(&info.ctx_seed_fields, &info.params_only_seed_fields);
            unpack_match_arms.push(generate_unpack_match_arm(info, &seeds)?);
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

    /// Generate DecompressibleAccount implementations for each PackedXxxData struct.
    ///
    /// Each impl provides:
    /// - `is_token()` returning false (PDA variants are not tokens)
    /// - `prepare()` that resolves indices, unpacks data, derives PDA, and calls
    ///   prepare_account_for_decompression_idempotent
    fn generate_decompressible_account_impls(&self) -> Result<TokenStream> {
        let mut impls = Vec::new();

        for info in self.pda_ctx_seeds.iter() {
            let variant_name = &info.variant_name;
            let packed_data_struct_name = format_ident!("Packed{}Data", variant_name);
            let inner_type = qualify_type_with_crate(&info.inner_type);
            let ctx_seeds_struct_name = format_ident!("{}CtxSeeds", variant_name);
            let ctx_fields = &info.ctx_seed_fields;
            let params_only_fields = &info.params_only_seed_fields;

            // Generate code to resolve idx fields to Pubkeys
            let resolve_ctx_seeds: Vec<_> = ctx_fields
                .iter()
                .map(|field| {
                    let idx_field = format_ident!("{}_idx", field);
                    quote! {
                        let #field = *ctx.remaining_accounts
                            .get(self.#idx_field as usize)
                            .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                            .key;
                    }
                })
                .collect();

            // Generate CtxSeeds struct construction
            let ctx_seeds_construction = if ctx_fields.is_empty() {
                quote! { let ctx_seeds = #ctx_seeds_struct_name; }
            } else {
                let field_inits: Vec<_> = ctx_fields.iter().map(|f| quote! { #f }).collect();
                quote! { let ctx_seeds = #ctx_seeds_struct_name { #(#field_inits),* }; }
            };

            // Generate SeedParams from params-only fields
            let seed_params_construction = if params_only_fields.is_empty() {
                quote! { let seed_params = SeedParams::default(); }
            } else {
                let field_inits: Vec<_> = params_only_fields
                    .iter()
                    .map(|(field, _, _)| {
                        quote! { #field: std::option::Option::Some(self.#field) }
                    })
                    .collect();
                quote! {
                    let seed_params = SeedParams {
                        #(#field_inits,)*
                        ..Default::default()
                    };
                }
            };

            // Generate data unpacking code based on whether this is a zero-copy account
            // For zero_copy, use unpack_stripped to reconstruct from stripped bytes; for Borsh, use Unpack trait
            let unpack_data_code = if info.is_zero_copy {
                quote! {
                    // Reconstruct full Pod from stripped bytes (zeros at CompressionInfo offset)
                    let data: #inner_type = <#inner_type as light_sdk::interface::PodCompressionInfoField>::unpack_stripped(&self.data)?;
                }
            } else {
                let packed_inner_type = make_packed_type(&info.inner_type).ok_or_else(|| {
                    syn::Error::new_spanned(&info.inner_type, "invalid type path for packed type")
                })?;
                quote! {
                    let data: #inner_type = <#packed_inner_type as light_sdk::interface::Unpack>::unpack(
                        &self.data, ctx.remaining_accounts
                    )?;
                }
            };

            // Generate the decompression call based on whether this is a zero-copy account
            let decompression_call = if info.is_zero_copy {
                quote! {
                    light_sdk::interface::prepare_account_for_decompression_idempotent_pod::<#inner_type>(
                        ctx.program_id,
                        data,
                        compressed_meta,
                        solana_account,
                        ctx.rent_sponsor,
                        ctx.cpi_accounts,
                        &seed_refs[..len],
                        ctx.rent,
                        ctx.current_slot,
                    ).map_err(|e| e.into())
                }
            } else {
                quote! {
                    light_sdk::interface::prepare_account_for_decompression_idempotent::<#inner_type>(
                        ctx.program_id,
                        data,
                        compressed_meta,
                        solana_account,
                        ctx.rent_sponsor,
                        ctx.cpi_accounts,
                        &seed_refs[..len],
                        ctx.rent,
                        ctx.current_slot,
                    ).map_err(|e| e.into())
                }
            };

            impls.push(quote! {
                impl light_sdk::interface::DecompressibleAccount for #packed_data_struct_name {
                    fn is_token(&self) -> bool { false }

                    fn prepare<'a, 'info>(
                        self,
                        ctx: &light_sdk::interface::DecompressCtx<'a, 'info>,
                        solana_account: &solana_account_info::AccountInfo<'info>,
                        meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                        index: usize,
                    ) -> std::result::Result<
                        std::option::Option<light_sdk::compressed_account::CompressedAccountInfo>,
                        solana_program_error::ProgramError
                    > {
                        // 1. Resolve idx fields to Pubkeys
                        #(#resolve_ctx_seeds)*

                        // 2. Build CtxSeeds struct
                        #ctx_seeds_construction

                        // 3. Build SeedParams
                        #seed_params_construction

                        // 4. Unpack data
                        #unpack_data_code

                        // 5. Derive PDA seeds
                        let (seeds_vec, derived_pda) = <#inner_type as light_sdk::interface::PdaSeedDerivation<
                            #ctx_seeds_struct_name, SeedParams
                        >>::derive_pda_seeds_with_accounts(
                            &data, ctx.program_id, &ctx_seeds, &seed_params
                        )?;

                        // 6. Verify PDA matches
                        if derived_pda != *solana_account.key {
                            solana_msg::msg!(
                                "Derived PDA mismatch at {}: expected {:?}, got {:?}",
                                index, solana_account.key, derived_pda
                            );
                            return Err(light_sdk::error::LightSdkError::ConstraintViolation.into());
                        }

                        // 7. Build seed refs and call appropriate decompression function
                        const MAX_SEEDS: usize = 16;
                        let mut seed_refs: [&[u8]; MAX_SEEDS] = [&[]; MAX_SEEDS];
                        let len = seeds_vec.len().min(MAX_SEEDS);
                        for i in 0..len {
                            seed_refs[i] = seeds_vec[i].as_slice();
                        }

                        let compressed_meta = light_sdk::interface::into_compressed_meta_with_address(
                            meta, solana_account, ctx.address_space, ctx.program_id
                        );

                        #decompression_call
                    }
                }
            });
        }

        Ok(quote! { #(#impls)* })
    }

    /// Generate DecompressibleAccount implementation for the LightAccountVariant enum.
    ///
    /// - `is_token()` returns true for CToken variants, false for PDA variants
    /// - `prepare()` delegates to the inner PackedXxxData struct's prepare method
    fn generate_decompressible_account_enum_impl(&self) -> TokenStream {
        let is_token_arms: Vec<_> = self
            .pda_ctx_seeds
            .iter()
            .map(|info| {
                let variant_name = &info.variant_name;
                let packed_variant_name = format_ident!("Packed{}", variant_name);
                quote! {
                    Self::#variant_name { .. } => false,
                    Self::#packed_variant_name(_) => false,
                }
            })
            .collect();

        let prepare_arms: Vec<_> = self
            .pda_ctx_seeds
            .iter()
            .map(|info| {
                let variant_name = &info.variant_name;
                let packed_variant_name = format_ident!("Packed{}", variant_name);
                quote! {
                    Self::#packed_variant_name(inner) => inner.prepare(ctx, solana_account, meta, index),
                    Self::#variant_name { .. } => {
                        Err(light_sdk::error::LightSdkError::UnexpectedUnpackedVariant.into())
                    }
                }
            })
            .collect();

        let ctoken_is_token_arms = if self.include_ctoken {
            quote! {
                Self::PackedCTokenData(_) => true,
                Self::CTokenData(_) => true,
            }
        } else {
            quote! {}
        };

        let ctoken_prepare_arms = if self.include_ctoken {
            quote! {
                Self::PackedCTokenData(_) | Self::CTokenData(_) => {
                    Err(light_sdk::error::LightSdkError::TokenPrepareCalled.into())
                }
            }
        } else {
            quote! {}
        };

        quote! {
            impl light_sdk::interface::DecompressibleAccount for LightAccountVariant {
                fn is_token(&self) -> bool {
                    match self {
                        #(#is_token_arms)*
                        #ctoken_is_token_arms
                    }
                }

                fn prepare<'a, 'info>(
                    self,
                    ctx: &light_sdk::interface::DecompressCtx<'a, 'info>,
                    solana_account: &solana_account_info::AccountInfo<'info>,
                    meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    index: usize,
                ) -> std::result::Result<
                    std::option::Option<light_sdk::compressed_account::CompressedAccountInfo>,
                    solana_program_error::ProgramError
                > {
                    match self {
                        #(#prepare_arms)*
                        #ctoken_prepare_arms
                    }
                }
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
    /// True if the field uses zero-copy serialization (AccountLoader).
    /// When true, decompression uses prepare_account_for_decompression_idempotent_pod.
    pub is_zero_copy: bool,
}

impl PdaCtxSeedInfo {
    pub fn with_state_fields(
        variant_name: Ident,
        inner_type: Type,
        ctx_seed_fields: Vec<Ident>,
        state_field_names: std::collections::HashSet<String>,
        params_only_seed_fields: Vec<(Ident, Type, bool)>,
        is_zero_copy: bool,
    ) -> Self {
        Self {
            variant_name,
            inner_type,
            ctx_seed_fields,
            state_field_names,
            params_only_seed_fields,
            is_zero_copy,
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
        generate_token_variant_enum(self.token_seeds, "TokenAccountVariant", false)
    }

    /// Generate the packed PackedTokenAccountVariant enum.
    fn generate_packed_enum(&self) -> TokenStream {
        generate_token_variant_enum(self.token_seeds, "PackedTokenAccountVariant", true)
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
            impl light_token::pack::Pack for TokenAccountVariant {
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
            impl light_token::pack::Unpack for PackedTokenAccountVariant {
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
            impl light_sdk::interface::IntoCTokenVariant<LightAccountVariant, light_token::compat::TokenData> for TokenAccountVariant {
                fn into_ctoken_variant(self, token_data: light_token::compat::TokenData) -> LightAccountVariant {
                    LightAccountVariant::CTokenData(light_token::compat::CTokenData {
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

// -----------------------------------------------------------------------------
// Seed Field Collection Helper (Phase 1)
// -----------------------------------------------------------------------------

/// Collected seed field identifiers for code generation.
///
/// This struct centralizes the collection of context and params-only seed fields,
/// avoiding repeated collection logic across pack/unpack implementations.
struct SeedFieldCollection<'a> {
    /// References to ctx.accounts.* field names
    ctx_field_names: Vec<&'a Ident>,
    /// Derived index field names (e.g., `field_idx` for `field`)
    idx_field_names: Vec<Ident>,
    /// References to params-only field names
    params_field_names: Vec<&'a Ident>,
}

impl<'a> SeedFieldCollection<'a> {
    /// Create a new SeedFieldCollection from context seed fields and params-only fields.
    fn new(
        ctx_fields: &'a [Ident],
        params_only_fields: &'a [(Ident, Type, bool)],
    ) -> Self {
        Self {
            ctx_field_names: ctx_fields.iter().collect(),
            idx_field_names: ctx_fields
                .iter()
                .map(|f| format_ident!("{}_idx", f))
                .collect(),
            params_field_names: params_only_fields.iter().map(|(f, _, _)| f).collect(),
        }
    }

    /// Returns true if there are any seeds (ctx or params).
    fn has_seeds(&self) -> bool {
        !self.ctx_field_names.is_empty() || !self.params_field_names.is_empty()
    }
}

// -----------------------------------------------------------------------------
// Seed Packing/Unpacking Helpers (Phase 2)
// -----------------------------------------------------------------------------

/// Generate statements to pack context seeds into indices.
///
/// For each ctx field, generates: `let field_idx = remaining_accounts.insert_or_get(*field);`
fn generate_pack_seed_statements(ctx_fields: &[Ident]) -> Vec<TokenStream> {
    ctx_fields
        .iter()
        .map(|field| {
            let idx_field = format_ident!("{}_idx", field);
            quote! { let #idx_field = remaining_accounts.insert_or_get(*#field); }
        })
        .collect()
}

/// Generate statements to unpack seed indices back to Pubkeys.
///
/// For each ctx field, generates a statement that retrieves the Pubkey from remaining_accounts
/// using the stored index.
fn generate_unpack_seed_statements(ctx_fields: &[Ident]) -> Vec<TokenStream> {
    ctx_fields
        .iter()
        .map(|field| {
            let idx_field = format_ident!("{}_idx", field);
            quote! {
                let #field = *remaining_accounts
                    .get(inner.#idx_field as usize)
                    .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                    .key;
            }
        })
        .collect()
}

// -----------------------------------------------------------------------------
// Pack/Unpack Match Arm Generators (Phase 3 & 4)
// -----------------------------------------------------------------------------

/// Generate a pack match arm for a single PDA variant.
///
/// Handles both zero_copy and Borsh accounts, with or without seeds.
fn generate_pack_match_arm(info: &PdaCtxSeedInfo, seeds: &SeedFieldCollection) -> TokenStream {
    let variant_name = &info.variant_name;
    let packed_variant_name = format_ident!("Packed{}", variant_name);
    let packed_data_struct_name = format_ident!("Packed{}Data", variant_name);

    // Data packing expression differs by account type
    let data_expr = if info.is_zero_copy {
        quote! { data.clone() }
    } else {
        let inner_type = qualify_type_with_crate(&info.inner_type);
        quote! { <#inner_type as light_sdk::interface::Pack>::pack(data, remaining_accounts)? }
    };

    // Generate pack statements for ctx seeds
    let pack_ctx_seeds = generate_pack_seed_statements(&info.ctx_seed_fields);
    let idx_field_names = &seeds.idx_field_names;
    let params_field_names = &seeds.params_field_names;
    let ctx_field_names = &seeds.ctx_field_names;

    if seeds.has_seeds() {
        quote! {
            LightAccountVariant::#packed_variant_name(_) => Err(solana_program_error::ProgramError::InvalidAccountData),
            LightAccountVariant::#variant_name { data, #(#ctx_field_names,)* #(#params_field_names,)* .. } => {
                #(#pack_ctx_seeds)*
                Ok(LightAccountVariant::#packed_variant_name(#packed_data_struct_name {
                    data: #data_expr,
                    #(#idx_field_names,)*
                    #(#params_field_names: *#params_field_names,)*
                }))
            },
        }
    } else {
        quote! {
            LightAccountVariant::#packed_variant_name(_) => Err(solana_program_error::ProgramError::InvalidAccountData),
            LightAccountVariant::#variant_name { data, .. } => {
                Ok(LightAccountVariant::#packed_variant_name(#packed_data_struct_name {
                    data: #data_expr,
                }))
            },
        }
    }
}

/// Generate an unpack match arm for a single PDA variant.
///
/// Handles both zero_copy and Borsh accounts, with or without seeds.
fn generate_unpack_match_arm(
    info: &PdaCtxSeedInfo,
    seeds: &SeedFieldCollection,
) -> Result<TokenStream> {
    let variant_name = &info.variant_name;
    let packed_variant_name = make_packed_variant_name(variant_name);
    let inner_type = &info.inner_type;

    // Data unpacking expression and assignment differ by account type
    let (data_unpack, data_expr) = if info.is_zero_copy {
        let qualified = qualify_type_with_crate(inner_type);
        (
            quote! {
                let full_pod = <#qualified as light_sdk::interface::PodCompressionInfoField>::unpack_stripped(&inner.data)?;
            },
            quote! { bytemuck::bytes_of(&full_pod).to_vec() },
        )
    } else {
        let packed_inner_type = make_packed_type(inner_type).ok_or_else(|| {
            syn::Error::new_spanned(inner_type, "invalid type path for packed type")
        })?;
        (
            quote! {
                let data = <#packed_inner_type as light_sdk::interface::Unpack>::unpack(&inner.data, remaining_accounts)?;
            },
            quote! { data },
        )
    };

    let unpack_ctx_seeds = generate_unpack_seed_statements(&info.ctx_seed_fields);
    let ctx_field_names = &seeds.ctx_field_names;
    let params_field_values: Vec<_> = seeds
        .params_field_names
        .iter()
        .map(|f| quote! { #f: inner.#f })
        .collect();

    if seeds.has_seeds() {
        Ok(quote! {
            LightAccountVariant::#packed_variant_name(inner) => {
                #(#unpack_ctx_seeds)*
                #data_unpack
                Ok(LightAccountVariant::#variant_name {
                    data: #data_expr,
                    #(#ctx_field_names,)*
                    #(#params_field_values,)*
                })
            },
            LightAccountVariant::#variant_name { .. } => Err(solana_program_error::ProgramError::InvalidAccountData),
        })
    } else {
        Ok(quote! {
            LightAccountVariant::#packed_variant_name(inner) => {
                #data_unpack
                Ok(LightAccountVariant::#variant_name {
                    data: #data_expr,
                })
            },
            LightAccountVariant::#variant_name { .. } => Err(solana_program_error::ProgramError::InvalidAccountData),
        })
    }
}

// -----------------------------------------------------------------------------
// Token Variant Enum Helper (Phase 5)
// -----------------------------------------------------------------------------

/// Generate a token variant enum with customizable field types.
///
/// This unifies the generation of TokenAccountVariant (Pubkey fields) and
/// PackedTokenAccountVariant (u8 index fields).
fn generate_token_variant_enum(
    token_seeds: &[TokenSeedSpec],
    enum_name: &str,
    is_packed: bool,
) -> TokenStream {
    let enum_ident = format_ident!("{}", enum_name);
    let variants = token_seeds.iter().map(|spec| {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        let fields: Vec<_> = ctx_fields
            .iter()
            .map(|field| {
                if is_packed {
                    let idx_field = format_ident!("{}_idx", field);
                    quote! { #idx_field: u8 }
                } else {
                    quote! { #field: Pubkey }
                }
            })
            .collect();

        if ctx_fields.is_empty() {
            quote! { #variant_name, }
        } else {
            quote! { #variant_name { #(#fields,)* }, }
        }
    });

    quote! {
        #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
        pub enum #enum_ident {
            #(#variants)*
        }
    }
}

// -----------------------------------------------------------------------------
// Public Helper Functions
// -----------------------------------------------------------------------------

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
