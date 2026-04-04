//! Program-wide variant enum generation for #[light_program] macro.
//!
//! This module generates:
//! 1. `LightAccountVariant` enum collecting all per-field variants from instruction structs
//! 2. `PackedLightAccountVariant` enum with packed versions
//! 3. `impl DecompressVariant for PackedLightAccountVariant` dispatch
//!
//! Token variants are first-class members of the main enums, using
//! `TokenDataWithSeeds<S>` / `TokenDataWithPackedSeeds<S>` wrappers.
//! The per-field variant structs (`{Field}Variant`, `Packed{Field}Variant`) are generated
//! by `#[derive(LightAccounts)]` in `accounts/variant.rs`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result, Type};

use super::parsing::{SeedElement, TokenSeedSpec};
use crate::light_pdas::{backend::CodegenBackend, shared_utils::qualify_type_with_crate};

// =============================================================================
// LIGHT VARIANT BUILDER
// =============================================================================

/// Builder for generating program-wide variant enums and dispatch implementations.
///
/// Takes `PdaCtxSeedInfo` and `TokenSeedSpec` collected from instruction account
/// structs and generates unified enums where both PDA and token variants are
/// first-class members.
pub(super) struct LightVariantBuilder<'a> {
    /// PDA ctx seed info collected from all instruction account structs.
    pda_ctx_seeds: &'a [PdaCtxSeedInfo],
    /// Token seed specifications (empty slice if no token accounts).
    token_seeds: &'a [TokenSeedSpec],
}

impl<'a> LightVariantBuilder<'a> {
    /// Create a new LightVariantBuilder with the given PDA ctx seed info.
    pub fn new(pda_ctx_seeds: &'a [PdaCtxSeedInfo]) -> Self {
        Self {
            pda_ctx_seeds,
            token_seeds: &[],
        }
    }

    /// Set token seed specs (for programs with token fields).
    pub fn with_token_seeds(mut self, token_seeds: &'a [TokenSeedSpec]) -> Self {
        self.token_seeds = token_seeds;
        self
    }

    /// Validate the builder configuration.
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

    /// Generate the complete enum definitions and trait implementations using the specified backend.
    pub fn build_with_backend(&self, backend: &dyn CodegenBackend) -> Result<TokenStream> {
        self.validate()?;

        // NOTE: Variant structs (`RecordVariant`, `PackedRecordVariant`, etc.) are generated
        // by `#[derive(LightAccounts)]` in the instruction module. We just wrap them in
        // the program-wide enum here. Do NOT regenerate them to avoid conflicts.
        let token_seeds_structs = self.generate_token_seeds_structs_with_backend(backend);
        let token_variant_trait_impls =
            self.generate_token_variant_trait_impls_with_backend(backend);
        let unpacked_enum = self.generate_unpacked_enum_with_backend(backend);
        let packed_enum = self.generate_packed_enum_with_backend(backend);
        let light_account_data_struct =
            self.generate_light_account_data_struct_with_backend(backend);
        let decompress_variant_impl = self.generate_decompress_variant_impl_with_backend(backend);
        let pack_impl = self.generate_pack_impl_with_backend(backend);

        Ok(quote! {
            #token_seeds_structs
            #token_variant_trait_impls
            #unpacked_enum
            #packed_enum
            #light_account_data_struct
            #decompress_variant_impl
            #pack_impl
        })
    }

    // =========================================================================
    // UNIFIED BACKEND-BASED GENERATION METHODS
    // =========================================================================

    /// Generate the `LightAccountData` wrapper struct using the specified backend.
    fn generate_light_account_data_struct_with_backend(
        &self,
        backend: &dyn CodegenBackend,
    ) -> TokenStream {
        let account_crate = backend.account_crate();
        let serialize_derive = backend.serialize_derive();
        let deserialize_derive = backend.deserialize_derive();

        if backend.is_pinocchio() {
            quote! {
                #[derive(Clone, Debug, #serialize_derive, #deserialize_derive)]
                pub struct LightAccountData {
                    pub meta: #account_crate::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    pub data: PackedLightAccountVariant,
                }
            }
        } else {
            quote! {
                /// Wrapper for compressed account data with metadata.
                /// Contains PACKED variant data that will be decompressed into PDA accounts.
                #[derive(Clone, Debug, #serialize_derive, #deserialize_derive)]
                pub struct LightAccountData {
                    pub meta: #account_crate::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    pub data: PackedLightAccountVariant,
                }
            }
        }
    }

    // =========================================================================
    // TOKEN SEEDS STRUCTS
    // =========================================================================

    /// Generate `{Variant}Seeds`, `Packed{Variant}Seeds`, and their Pack/Unpack impls
    /// for each token variant using the specified backend.
    fn generate_token_seeds_structs_with_backend(
        &self,
        backend: &dyn CodegenBackend,
    ) -> TokenStream {
        let account_crate = backend.account_crate();
        let serialize_derive = backend.serialize_derive();
        let deserialize_derive = backend.deserialize_derive();
        let sdk_error = backend.sdk_error_type();

        let structs: Vec<_> = self
            .token_seeds
            .iter()
            .map(|spec| {
                let variant_name = &spec.variant;
                let seeds_name = format_ident!("{}Seeds", variant_name);
                let packed_seeds_name = format_ident!("Packed{}Seeds", variant_name);
                let ctx_fields = extract_ctx_fields_from_token_spec(spec);

                // Unpacked seeds: Pubkey or [u8; 32] fields depending on backend
                let unpacked_fields: Vec<_> = ctx_fields
                    .iter()
                    .map(|f| {
                        if backend.is_pinocchio() {
                            quote! { pub #f: [u8; 32] }
                        } else {
                            quote! { pub #f: Pubkey }
                        }
                    })
                    .collect();

                // Packed seeds: u8 index fields + bump
                let packed_fields: Vec<_> = ctx_fields
                    .iter()
                    .map(|f| {
                        let idx = format_ident!("{}_idx", f);
                        quote! { pub #idx: u8 }
                    })
                    .collect();

                // Pack impl: Pubkey -> u8 index
                let pack_stmts: Vec<_> = ctx_fields
                    .iter()
                    .map(|f| {
                        let idx = format_ident!("{}_idx", f);
                        if backend.is_pinocchio() {
                            quote! { #idx: remaining_accounts.insert_or_get(light_account_pinocchio::solana_pubkey::Pubkey::from(self.#f)) }
                        } else {
                            quote! { #idx: remaining_accounts.insert_or_get(AM::pubkey_from_bytes(self.#f.to_bytes())) }
                        }
                    })
                    .collect();

                // Seed refs for find_program_address bump derivation
                let bump_seed_refs: Vec<_> = spec
                    .seeds
                    .iter()
                    .map(seed_to_unpacked_ref)
                    .collect();

                // Unpack impl: u8 index -> Pubkey or [u8; 32]
                let unpack_resolve_stmts: Vec<_> = ctx_fields
                    .iter()
                    .map(|f| {
                        let idx = format_ident!("{}_idx", f);
                        if backend.is_pinocchio() {
                            quote! {
                                let #f: [u8; 32] =
                                    remaining_accounts
                                        .get(self.#idx as usize)
                                        .ok_or(#sdk_error::InvalidInstructionData)?
                                        .key();
                            }
                        } else {
                            quote! {
                                let #f = solana_pubkey::Pubkey::new_from_array(
                                    remaining_accounts
                                        .get(self.#idx as usize)
                                        .ok_or(#sdk_error::InvalidInstructionData)?
                                        .key()
                                );
                            }
                        }
                    })
                    .collect();

                let unpack_field_assigns: Vec<_> = ctx_fields.iter().map(|f| quote! { #f }).collect();

                let seeds_struct = if unpacked_fields.is_empty() {
                    quote! {
                        #[derive(#serialize_derive, #deserialize_derive, Clone, Debug)]
                        pub struct #seeds_name;
                    }
                } else {
                    quote! {
                        #[derive(#serialize_derive, #deserialize_derive, Clone, Debug)]
                        pub struct #seeds_name {
                            #(#unpacked_fields,)*
                        }
                    }
                };

                let pack_impl = if backend.is_pinocchio() {
                    quote! {
                        #[cfg(not(target_os = "solana"))]
                        impl #account_crate::Pack<#account_crate::solana_instruction::AccountMeta> for #seeds_name {
                            type Packed = #packed_seeds_name;

                            fn pack(
                                &self,
                                remaining_accounts: &mut #account_crate::PackedAccounts,
                            ) -> std::result::Result<Self::Packed, #sdk_error> {
                                let __seeds: &[&[u8]] = &[#(#bump_seed_refs),*];
                                let (_, __bump) = #account_crate::solana_pubkey::Pubkey::find_program_address(
                                    __seeds,
                                    &#account_crate::solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id),
                                );
                                Ok(#packed_seeds_name {
                                    #(#pack_stmts,)*
                                    bump: __bump,
                                })
                            }
                        }
                    }
                } else {
                    quote! {
                        // Pack trait is only available off-chain (client-side)
                        #[cfg(not(target_os = "solana"))]
                        impl<AM: #account_crate::AccountMetaTrait> #account_crate::Pack<AM> for #seeds_name {
                            type Packed = #packed_seeds_name;

                            fn pack(
                                &self,
                                remaining_accounts: &mut #account_crate::interface::instruction::PackedAccounts<AM>,
                            ) -> std::result::Result<Self::Packed, #sdk_error> {
                                let __seeds: &[&[u8]] = &[#(#bump_seed_refs),*];
                                let (_, __bump) = solana_pubkey::Pubkey::find_program_address(
                                    __seeds,
                                    &solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id),
                                );
                                Ok(#packed_seeds_name {
                                    #(#pack_stmts,)*
                                    bump: __bump,
                                })
                            }
                        }
                    }
                };

                let unpack_impl = if backend.is_pinocchio() {
                    quote! {
                        impl<AI: #account_crate::light_account_checks::AccountInfoTrait> #account_crate::Unpack<AI> for #packed_seeds_name {
                            type Unpacked = #seeds_name;

                            fn unpack(
                                &self,
                                remaining_accounts: &[AI],
                            ) -> std::result::Result<Self::Unpacked, #sdk_error> {
                                #(#unpack_resolve_stmts)*
                                Ok(#seeds_name {
                                    #(#unpack_field_assigns,)*
                                })
                            }
                        }
                    }
                } else {
                    quote! {
                        impl<AI: #account_crate::AccountInfoTrait> #account_crate::Unpack<AI> for #packed_seeds_name {
                            type Unpacked = #seeds_name;

                            fn unpack(
                                &self,
                                remaining_accounts: &[AI],
                            ) -> std::result::Result<Self::Unpacked, #sdk_error> {
                                #(#unpack_resolve_stmts)*
                                Ok(#seeds_name {
                                    #(#unpack_field_assigns,)*
                                })
                            }
                        }
                    }
                };

                quote! {
                    #seeds_struct

                    #[derive(#serialize_derive, #deserialize_derive, Clone, Debug)]
                    pub struct #packed_seeds_name {
                        #(#packed_fields,)*
                        pub bump: u8,
                    }

                    #pack_impl

                    #unpack_impl
                }
            })
            .collect();

        quote! { #(#structs)* }
    }

    // =========================================================================
    // TOKEN VARIANT TRAIT IMPLS
    // =========================================================================

    /// Generate `UnpackedTokenSeeds<N>` and `PackedTokenSeeds<N>` impls
    /// on the local seed structs using the specified backend.
    fn generate_token_variant_trait_impls_with_backend(
        &self,
        backend: &dyn CodegenBackend,
    ) -> TokenStream {
        let account_crate = backend.account_crate();
        let sdk_error = backend.sdk_error_type();
        let account_info_trait = backend.account_info_trait();

        let impls: Vec<_> = self
            .token_seeds
            .iter()
            .map(|spec| {
                let seeds_name = format_ident!("{}Seeds", spec.variant);
                let packed_seeds_name = format_ident!("Packed{}Seeds", spec.variant);

                // seed_count = number of seeds + 1 for bump
                let seed_count = spec.seeds.len() + 1;

                // --- Unpacked seed refs (self is the seeds struct directly) ---
                let unpacked_seed_ref_items: Vec<_> = spec
                    .seeds
                    .iter()
                    .map(seed_to_unpacked_ref)
                    .collect();

                // seed_vec items (owned Vec<u8> for each seed, self is seeds struct)
                let seed_vec_items: Vec<_> = spec
                    .seeds
                    .iter()
                    .map(|seed| {
                        match seed {
                            SeedElement::Literal(lit) => {
                                let value = lit.value();
                                quote! { #value.as_bytes().to_vec() }
                            }
                            SeedElement::Expression(expr) => {
                                if let Some(field_name) = extract_ctx_field_from_expr(expr) {
                                    quote! { self.#field_name.as_ref().to_vec() }
                                } else {
                                    if let syn::Expr::Lit(lit_expr) = &**expr {
                                        if let syn::Lit::ByteStr(byte_str) = &lit_expr.lit {
                                            let bytes = byte_str.value();
                                            return quote! { vec![#(#bytes),*] };
                                        }
                                    }
                                    if let syn::Expr::Path(path_expr) = &**expr {
                                        if path_expr.qself.is_none() {
                                            if let Some(last_seg) = path_expr.path.segments.last() {
                                                if crate::light_pdas::shared_utils::is_constant_identifier(&last_seg.ident.to_string()) {
                                                    let path = &path_expr.path;
                                                    return quote! { { let __seed: &[u8] = #path.as_ref(); __seed.to_vec() } };
                                                }
                                            }
                                        }
                                    }
                                    quote! { { let __seed: &[u8] = (#expr).as_ref(); __seed.to_vec() } }
                                }
                            }
                        }
                    })
                    .collect();

                // --- Packed seed refs (self is the packed seeds struct directly) ---
                let packed_seed_ref_items: Vec<_> = spec
                    .seeds
                    .iter()
                    .map(|s| seed_to_packed_ref_with_crate(s, &account_crate))
                    .collect();

                // --- Owner derivation from owner_seeds (constants only) ---
                let owner_derivation = if let Some(owner_seeds) = &spec.owner_seeds {
                    let owner_seed_refs: Vec<_> = owner_seeds
                        .iter()
                        .map(|seed| {
                            match seed {
                                SeedElement::Literal(lit) => {
                                    let value = lit.value();
                                    quote! { #value.as_bytes() }
                                }
                                SeedElement::Expression(expr) => {
                                    // For constants like AUTH_SEED.as_bytes()
                                    quote! { { let __seed: &[u8] = (#expr).as_ref(); __seed } }
                                }
                            }
                        })
                        .collect();

                    if backend.is_pinocchio() {
                        quote! {
                            let (__owner, _) = #account_crate::solana_pubkey::Pubkey::find_program_address(
                                &[#(#owner_seed_refs),*],
                                &#account_crate::solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id),
                            );
                            __owner.to_bytes()
                        }
                    } else {
                        quote! {
                            let (__owner, _) = solana_pubkey::Pubkey::find_program_address(
                                &[#(#owner_seed_refs),*],
                                &solana_pubkey::Pubkey::from(crate::LIGHT_CPI_SIGNER.program_id),
                            );
                            __owner.to_bytes()
                        }
                    }
                } else {
                    // No owner_seeds - return default (shouldn't happen for token accounts)
                    quote! { [0u8; 32] }
                };

                quote! {
                    impl #account_crate::UnpackedTokenSeeds<#seed_count>
                        for #seeds_name
                    {
                        type Packed = #packed_seeds_name;

                        const PROGRAM_ID: [u8; 32] = crate::LIGHT_CPI_SIGNER.program_id;

                        fn seed_vec(&self) -> Vec<Vec<u8>> {
                            vec![#(#seed_vec_items),*]
                        }

                        fn seed_refs_with_bump<'a>(&'a self, bump_storage: &'a [u8; 1]) -> [&'a [u8]; #seed_count] {
                            [#(#unpacked_seed_ref_items,)* bump_storage]
                        }
                    }

                    impl #account_crate::PackedTokenSeeds<#seed_count>
                        for #packed_seeds_name
                    {
                        type Unpacked = #seeds_name;

                        fn bump(&self) -> u8 {
                            self.bump
                        }

                        fn unpack_seeds<AI: #account_info_trait>(
                            &self,
                            accounts: &[AI],
                        ) -> std::result::Result<Self::Unpacked, #sdk_error> {
                            <Self as #account_crate::Unpack<AI>>::unpack(self, accounts)
                        }

                        fn seed_refs_with_bump<'a, AI: #account_info_trait>(
                            &'a self,
                            accounts: &'a [AI],
                            bump_storage: &'a [u8; 1],
                        ) -> std::result::Result<[&'a [u8]; #seed_count], #sdk_error> {
                            Ok([#(#packed_seed_ref_items,)* bump_storage])
                        }

                        fn derive_owner(&self) -> [u8; 32] {
                            #owner_derivation
                        }
                    }
                }
            })
            .collect();

        quote! { #(#impls)* }
    }

    // =========================================================================
    // ENUM GENERATION
    // =========================================================================

    /// Generate the unpacked `LightAccountVariant` enum using the specified backend.
    fn generate_unpacked_enum_with_backend(&self, backend: &dyn CodegenBackend) -> TokenStream {
        let account_crate = backend.account_crate();
        let serialize_derive = backend.serialize_derive();
        let deserialize_derive = backend.deserialize_derive();

        let pda_variants: Vec<_> = self
            .pda_ctx_seeds
            .iter()
            .map(|info| {
                let variant_name = &info.variant_name;
                let seeds_type = format_ident!("{}Seeds", variant_name);
                let inner_type = qualify_type_with_crate(&info.inner_type);
                quote! { #variant_name { seeds: #seeds_type, data: #inner_type } }
            })
            .collect();

        let token_variants: Vec<_> = self
            .token_seeds
            .iter()
            .map(|spec| {
                let variant_name = &spec.variant;
                let seeds_name = format_ident!("{}Seeds", variant_name);
                quote! {
                    #variant_name(#account_crate::token::TokenDataWithSeeds<#seeds_name>)
                }
            })
            .collect();

        if backend.is_pinocchio() {
            quote! {
                #[derive(#serialize_derive, #deserialize_derive, Clone, Debug)]
                pub enum LightAccountVariant {
                    #(#pda_variants,)*
                    #(#token_variants,)*
                }
            }
        } else {
            quote! {
                /// Program-wide unpacked variant enum collecting all per-field variants.
                #[derive(#serialize_derive, #deserialize_derive, Clone, Debug)]
                pub enum LightAccountVariant {
                    #(#pda_variants,)*
                    #(#token_variants,)*
                }
            }
        }
    }

    /// Generate the packed `PackedLightAccountVariant` enum using the specified backend.
    fn generate_packed_enum_with_backend(&self, backend: &dyn CodegenBackend) -> TokenStream {
        let account_crate = backend.account_crate();
        let serialize_derive = backend.serialize_derive();
        let deserialize_derive = backend.deserialize_derive();

        let pda_variants: Vec<_> =
            self.pda_ctx_seeds
                .iter()
                .map(|info| {
                    let variant_name = &info.variant_name;
                    let packed_seeds_type = format_ident!("Packed{}Seeds", variant_name);
                    let inner_type = &info.inner_type;
                    let packed_data_type =
                        crate::light_pdas::shared_utils::make_packed_type(inner_type)
                            .unwrap_or_else(|| {
                                let type_str = quote!(#inner_type).to_string().replace(' ', "");
                                let packed_name = format_ident!("Packed{}", type_str);
                                syn::parse_quote!(#packed_name)
                            });
                    quote! { #variant_name { seeds: #packed_seeds_type, data: #packed_data_type } }
                })
                .collect();

        let token_variants: Vec<_> = self
            .token_seeds
            .iter()
            .map(|spec| {
                let variant_name = &spec.variant;
                let packed_seeds_name = format_ident!("Packed{}Seeds", variant_name);
                quote! {
                    #variant_name(#account_crate::token::TokenDataWithPackedSeeds<#packed_seeds_name>)
                }
            })
            .collect();

        if backend.is_pinocchio() {
            quote! {
                #[derive(#serialize_derive, #deserialize_derive, Clone, Debug)]
                pub enum PackedLightAccountVariant {
                    #(#pda_variants,)*
                    #(#token_variants,)*
                }
            }
        } else {
            quote! {
                /// Program-wide packed variant enum for efficient serialization.
                #[derive(#serialize_derive, #deserialize_derive, Clone, Debug)]
                pub enum PackedLightAccountVariant {
                    #(#pda_variants,)*
                    #(#token_variants,)*
                }
            }
        }
    }

    // =========================================================================
    // DECOMPRESS VARIANT IMPL
    // =========================================================================

    /// Generate `impl DecompressVariant for PackedLightAccountVariant` using the specified backend.
    fn generate_decompress_variant_impl_with_backend(
        &self,
        backend: &dyn CodegenBackend,
    ) -> TokenStream {
        let account_crate = backend.account_crate();
        let account_info_type = backend.account_info_type();
        let sdk_error = backend.sdk_error_type();

        let pda_arms: Vec<_> = self
            .pda_ctx_seeds
            .iter()
            .map(|info| {
                let variant_name = &info.variant_name;
                let packed_variant_type = format_ident!("Packed{}Variant", variant_name);
                let seed_count = info.seed_count;

                quote! {
                    Self::#variant_name { seeds, data } => {
                        let packed_data = #packed_variant_type { seeds: seeds.clone(), data: data.clone() };
                        #account_crate::prepare_account_for_decompression::<#seed_count, #packed_variant_type, #account_info_type>(
                            &packed_data,
                            tree_info,
                            output_queue_index,
                            pda_account,
                            ctx,
                        )
                    }
                }
            })
            .collect();

        let token_arms: Vec<_> = self
            .token_seeds
            .iter()
            .map(|spec| {
                let variant_name = &spec.variant;
                let packed_seeds_name = format_ident!("Packed{}Seeds", variant_name);
                let seed_count = spec.seeds.len() + 1;

                quote! {
                    Self::#variant_name(packed_data) => {
                        #account_crate::token::prepare_token_account_for_decompression::<
                            #seed_count,
                            #account_crate::token::TokenDataWithPackedSeeds<#packed_seeds_name>,
                            #account_info_type,
                        >(
                            packed_data,
                            tree_info,
                            output_queue_index,
                            pda_account,
                            ctx,
                        )
                    }
                }
            })
            .collect();

        if backend.is_pinocchio() {
            quote! {
                impl #account_crate::DecompressVariant<#account_info_type> for PackedLightAccountVariant {
                    fn decompress(
                        &self,
                        tree_info: &#account_crate::PackedStateTreeInfo,
                        pda_account: &#account_info_type,
                        ctx: &mut #account_crate::DecompressCtx<'_>,
                    ) -> std::result::Result<(), #sdk_error> {
                        let output_queue_index = ctx.output_queue_index;
                        match self {
                            #(#pda_arms)*
                            #(#token_arms)*
                        }
                    }
                }
            }
        } else {
            quote! {
                impl<'info> #account_crate::DecompressVariant<#account_crate::AccountInfo<'info>> for PackedLightAccountVariant {
                    fn decompress(
                        &self,
                        tree_info: &#account_crate::PackedStateTreeInfo,
                        pda_account: &#account_crate::AccountInfo<'info>,
                        ctx: &mut #account_crate::DecompressCtx<'_, 'info>,
                    ) -> std::result::Result<(), #sdk_error> {
                        let output_queue_index = ctx.output_queue_index;
                        match self {
                            #(#pda_arms)*
                            #(#token_arms)*
                        }
                    }
                }
            }
        }
    }

    // =========================================================================
    // PACK IMPL
    // =========================================================================

    /// Generate `impl Pack for LightAccountVariant` using the specified backend.
    fn generate_pack_impl_with_backend(&self, backend: &dyn CodegenBackend) -> TokenStream {
        let account_crate = backend.account_crate();
        let sdk_error = backend.sdk_error_type();
        let packed_accounts_type = backend.packed_accounts_type();
        let account_meta_type = backend.account_meta_type();

        let pda_arms: Vec<_> = self
            .pda_ctx_seeds
            .iter()
            .map(|info| {
                let variant_name = &info.variant_name;
                let variant_struct_name = format_ident!("{}Variant", variant_name);

                quote! {
                    Self::#variant_name { seeds, data } => {
                        let variant = #variant_struct_name { seeds: seeds.clone(), data: data.clone() };
                        let packed = #account_crate::Pack::pack(&variant, accounts)?;
                        Ok(PackedLightAccountVariant::#variant_name { seeds: packed.seeds, data: packed.data })
                    }
                }
            })
            .collect();

        let token_arms: Vec<_> = self
            .token_seeds
            .iter()
            .map(|spec| {
                let variant_name = &spec.variant;
                quote! {
                    Self::#variant_name(data) => {
                        let packed = #account_crate::Pack::pack(data, accounts)?;
                        Ok(PackedLightAccountVariant::#variant_name(packed))
                    }
                }
            })
            .collect();

        if backend.is_pinocchio() {
            quote! {
                #[cfg(not(target_os = "solana"))]
                impl #account_crate::Pack<#account_meta_type> for LightAccountVariant {
                    type Packed = PackedLightAccountVariant;

                    fn pack(
                        &self,
                        accounts: &mut #packed_accounts_type,
                    ) -> std::result::Result<Self::Packed, #sdk_error> {
                        match self {
                            #(#pda_arms)*
                            #(#token_arms)*
                        }
                    }
                }
            }
        } else {
            quote! {
                // Pack trait is only available off-chain (client-side)
                #[cfg(not(target_os = "solana"))]
                impl<AM: #account_meta_type> #account_crate::Pack<AM> for LightAccountVariant {
                    type Packed = PackedLightAccountVariant;

                    fn pack(
                        &self,
                        accounts: &mut #packed_accounts_type,
                    ) -> std::result::Result<Self::Packed, #sdk_error> {
                        match self {
                            #(#pda_arms)*
                            #(#token_arms)*
                        }
                    }
                }
            }
        }
    }
}

/// Info about ctx.* seeds for a PDA type.
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
    /// Total number of seeds + 1 for bump. This is used as the const generic N
    /// for PackedLightAccountVariant<N>.
    pub seed_count: usize,
}

impl PdaCtxSeedInfo {
    pub fn with_state_fields(
        variant_name: Ident,
        inner_type: Type,
        ctx_seed_fields: Vec<Ident>,
        state_field_names: std::collections::HashSet<String>,
        params_only_seed_fields: Vec<(Ident, Type, bool)>,
        seed_count: usize,
    ) -> Self {
        Self {
            variant_name,
            inner_type,
            ctx_seed_fields,
            state_field_names,
            params_only_seed_fields,
            seed_count,
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Extract ctx.* field names from seed elements (both token seeds and owner seeds).
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

    for seed in spec.seeds.iter().chain(spec.owner_seeds.iter().flatten()) {
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

/// Extract a single ctx field name from an expression.
/// Returns `Some(field_name)` for expressions like `ctx.accounts.mint.key()` or `ctx.mint.key()`.
fn extract_ctx_field_from_expr(expr: &syn::Expr) -> Option<Ident> {
    let fields = super::visitors::FieldExtractor::ctx_fields(&[
        "fee_payer",
        "rent_sponsor",
        "config",
        "compression_authority",
    ])
    .extract(expr);
    fields.into_iter().next()
}

/// Generate a seed ref expression for the UNPACKED variant (uses `self.seeds.field.as_ref()`).
fn seed_to_unpacked_ref(seed: &SeedElement) -> TokenStream {
    match seed {
        SeedElement::Literal(lit) => {
            let value = lit.value();
            quote! { #value.as_bytes() }
        }
        SeedElement::Expression(expr) => {
            if let syn::Expr::Lit(lit_expr) = &**expr {
                if let syn::Lit::ByteStr(byte_str) = &lit_expr.lit {
                    let bytes = byte_str.value();
                    return quote! { &[#(#bytes),*] };
                }
            }
            if let syn::Expr::Path(path_expr) = &**expr {
                if path_expr.qself.is_none() {
                    if let Some(last_seg) = path_expr.path.segments.last() {
                        if crate::light_pdas::shared_utils::is_constant_identifier(
                            &last_seg.ident.to_string(),
                        ) {
                            let path = &path_expr.path;
                            return quote! { { let __seed: &[u8] = #path.as_ref(); __seed } };
                        }
                    }
                }
            }
            if let Some(field_name) = extract_ctx_field_from_expr(expr) {
                return quote! { self.#field_name.as_ref() };
            }
            quote! { { let __seed: &[u8] = (#expr).as_ref(); __seed } }
        }
    }
}

/// Generate a seed ref expression for the PACKED variant (uses `accounts[idx].key.as_ref()`).
///
/// `account_crate` selects the error path: `light_account` for Anchor, `light_account_pinocchio` for pinocchio.
fn seed_to_packed_ref_with_crate(seed: &SeedElement, account_crate: &TokenStream) -> TokenStream {
    match seed {
        SeedElement::Literal(lit) => {
            let value = lit.value();
            quote! { #value.as_bytes() }
        }
        SeedElement::Expression(expr) => {
            if let syn::Expr::Lit(lit_expr) = &**expr {
                if let syn::Lit::ByteStr(byte_str) = &lit_expr.lit {
                    let bytes = byte_str.value();
                    return quote! { &[#(#bytes),*] };
                }
            }
            if let syn::Expr::Path(path_expr) = &**expr {
                if path_expr.qself.is_none() {
                    if let Some(last_seg) = path_expr.path.segments.last() {
                        if crate::light_pdas::shared_utils::is_constant_identifier(
                            &last_seg.ident.to_string(),
                        ) {
                            let path = &path_expr.path;
                            return quote! { { let __seed: &[u8] = #path.as_ref(); __seed } };
                        }
                    }
                }
            }
            if let Some(field_name) = extract_ctx_field_from_expr(expr) {
                let idx_field = format_ident!("{}_idx", field_name);
                return quote! {
                    accounts
                        .get(self.#idx_field as usize)
                        .ok_or(#account_crate::LightSdkTypesError::InvalidInstructionData)?
                        .key_ref()
                };
            }
            quote! { { let __seed: &[u8] = (#expr).as_ref(); __seed } }
        }
    }
}
