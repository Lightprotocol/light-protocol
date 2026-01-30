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

    /// Generate the complete enum definitions and trait implementations.
    pub fn build(&self) -> Result<TokenStream> {
        self.validate()?;

        // NOTE: Variant structs (`RecordVariant`, `PackedRecordVariant`, etc.) are generated
        // by `#[derive(LightAccounts)]` in the instruction module. We just wrap them in
        // the program-wide enum here. Do NOT regenerate them to avoid conflicts.
        let token_seeds_structs = self.generate_token_seeds_structs();
        let token_variant_trait_impls = self.generate_token_variant_trait_impls();
        let unpacked_enum = self.generate_unpacked_enum();
        let packed_enum = self.generate_packed_enum();
        let light_account_data_struct = self.generate_light_account_data_struct();
        let decompress_variant_impl = self.generate_decompress_variant_impl();
        let pack_impl = self.generate_pack_impl();

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

    /// Generate the `LightAccountData` wrapper struct.
    fn generate_light_account_data_struct(&self) -> TokenStream {
        quote! {
            /// Wrapper for compressed account data with metadata.
            /// Contains PACKED variant data that will be decompressed into PDA accounts.
            #[derive(Clone, Debug, borsh::BorshSerialize, borsh::BorshDeserialize)]
            pub struct LightAccountData {
                pub meta: light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                pub data: PackedLightAccountVariant,
            }
        }
    }

    // =========================================================================
    // TOKEN SEEDS STRUCTS
    // =========================================================================

    /// Generate `{Variant}Seeds`, `Packed{Variant}Seeds`, and their Pack/Unpack impls
    /// for each token variant. Same pattern as PDA seeds structs in accounts/variant.rs.
    fn generate_token_seeds_structs(&self) -> TokenStream {
        let structs: Vec<_> = self
            .token_seeds
            .iter()
            .map(|spec| {
                let variant_name = &spec.variant;
                let seeds_name = format_ident!("{}Seeds", variant_name);
                let packed_seeds_name = format_ident!("Packed{}Seeds", variant_name);
                let ctx_fields = extract_ctx_fields_from_token_spec(spec);

                // Unpacked seeds: Pubkey fields
                let unpacked_fields: Vec<_> = ctx_fields
                    .iter()
                    .map(|f| quote! { pub #f: Pubkey })
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
                        quote! { #idx: remaining_accounts.insert_or_get(self.#f) }
                    })
                    .collect();

                // Seed refs for find_program_address bump derivation
                let bump_seed_refs: Vec<_> = spec
                    .seeds
                    .iter()
                    .map(seed_to_unpacked_ref)
                    .collect();

                // Unpack impl: u8 index -> Pubkey
                let unpack_resolve_stmts: Vec<_> = ctx_fields
                    .iter()
                    .map(|f| {
                        let idx = format_ident!("{}_idx", f);
                        quote! {
                            let #f = *remaining_accounts
                                .get(self.#idx as usize)
                                .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                                .key;
                        }
                    })
                    .collect();

                let unpack_field_assigns: Vec<_> = ctx_fields.iter().map(|f| quote! { #f }).collect();

                let seeds_struct = if unpacked_fields.is_empty() {
                    quote! {
                        #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
                        pub struct #seeds_name;
                    }
                } else {
                    quote! {
                        #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
                        pub struct #seeds_name {
                            #(#unpacked_fields,)*
                        }
                    }
                };

                quote! {
                    #seeds_struct

                    #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
                    pub struct #packed_seeds_name {
                        #(#packed_fields,)*
                        pub bump: u8,
                    }

                    // Pack trait is only available off-chain (client-side)
                    #[cfg(not(target_os = "solana"))]
                    impl light_sdk::Pack for #seeds_name {
                        type Packed = #packed_seeds_name;

                        fn pack(
                            &self,
                            remaining_accounts: &mut light_sdk::instruction::PackedAccounts,
                        ) -> std::result::Result<Self::Packed, solana_program_error::ProgramError> {
                            let __seeds: &[&[u8]] = &[#(#bump_seed_refs),*];
                            let (_, __bump) = solana_pubkey::Pubkey::find_program_address(
                                __seeds,
                                &crate::ID,
                            );
                            Ok(#packed_seeds_name {
                                #(#pack_stmts,)*
                                bump: __bump,
                            })
                        }
                    }

                    impl light_sdk::Unpack for #packed_seeds_name {
                        type Unpacked = #seeds_name;

                        fn unpack(
                            &self,
                            remaining_accounts: &[solana_account_info::AccountInfo],
                        ) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
                            #(#unpack_resolve_stmts)*
                            Ok(#seeds_name {
                                #(#unpack_field_assigns,)*
                            })
                        }
                    }

                }
            })
            .collect();

        quote! { #(#structs)* }
    }

    // =========================================================================
    // TOKEN VARIANT TRAIT IMPLS
    // =========================================================================

    /// Generate `UnpackedTokenSeeds<N>` and `PackedTokenSeeds<N>` impls
    /// on the local seed structs. The blanket impls in `light_sdk::interface::token`
    /// then provide `LightAccountVariantTrait` / `PackedLightAccountVariantTrait`.
    fn generate_token_variant_trait_impls(&self) -> TokenStream {
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
                    .map(seed_to_packed_ref)
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
                    quote! {
                        let (__owner, _) = solana_pubkey::Pubkey::find_program_address(
                            &[#(#owner_seed_refs),*],
                            &crate::ID,
                        );
                        __owner
                    }
                } else {
                    // No owner_seeds - return default (shouldn't happen for token accounts)
                    quote! { solana_pubkey::Pubkey::default() }
                };

                quote! {
                    impl light_sdk::interface::UnpackedTokenSeeds<#seed_count>
                        for #seeds_name
                    {
                        type Packed = #packed_seeds_name;

                        const PROGRAM_ID: Pubkey = crate::ID;

                        fn seed_vec(&self) -> Vec<Vec<u8>> {
                            vec![#(#seed_vec_items),*]
                        }

                        fn seed_refs_with_bump<'a>(&'a self, bump_storage: &'a [u8; 1]) -> [&'a [u8]; #seed_count] {
                            [#(#unpacked_seed_ref_items,)* bump_storage]
                        }
                    }

                    impl light_sdk::interface::PackedTokenSeeds<#seed_count>
                        for #packed_seeds_name
                    {
                        fn bump(&self) -> u8 {
                            self.bump
                        }


                        fn seed_refs_with_bump<'a>(
                            &'a self,
                            accounts: &'a [anchor_lang::prelude::AccountInfo],
                            bump_storage: &'a [u8; 1],
                        ) -> std::result::Result<[&'a [u8]; #seed_count], solana_program_error::ProgramError> {
                            Ok([#(#packed_seed_ref_items,)* bump_storage])
                        }

                        fn derive_owner(&self) -> solana_pubkey::Pubkey {
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

    /// Generate the unpacked `LightAccountVariant` enum.
    fn generate_unpacked_enum(&self) -> TokenStream {
        let pda_variants: Vec<_> = self
            .pda_ctx_seeds
            .iter()
            .map(|info| {
                let variant_name = &info.variant_name;
                let variant_type = format_ident!("{}Variant", variant_name);
                quote! { #variant_name(#variant_type) }
            })
            .collect();

        let token_variants: Vec<_> = self
            .token_seeds
            .iter()
            .map(|spec| {
                let variant_name = &spec.variant;
                let seeds_name = format_ident!("{}Seeds", variant_name);
                quote! {
                    #variant_name(light_sdk::interface::token::TokenDataWithSeeds<#seeds_name>)
                }
            })
            .collect();

        quote! {
            /// Program-wide unpacked variant enum collecting all per-field variants.
            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
            pub enum LightAccountVariant {
                #(#pda_variants,)*
                #(#token_variants,)*
            }
        }
    }

    /// Generate the packed `PackedLightAccountVariant` enum.
    fn generate_packed_enum(&self) -> TokenStream {
        let pda_variants: Vec<_> = self
            .pda_ctx_seeds
            .iter()
            .map(|info| {
                let variant_name = &info.variant_name;
                let packed_variant_type = format_ident!("Packed{}Variant", variant_name);
                quote! { #variant_name(#packed_variant_type) }
            })
            .collect();

        let token_variants: Vec<_> = self
            .token_seeds
            .iter()
            .map(|spec| {
                let variant_name = &spec.variant;
                let packed_seeds_name = format_ident!("Packed{}Seeds", variant_name);
                quote! {
                    #variant_name(light_sdk::interface::token::TokenDataWithPackedSeeds<#packed_seeds_name>)
                }
            })
            .collect();

        quote! {
            /// Program-wide packed variant enum for efficient serialization.
            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
            pub enum PackedLightAccountVariant {
                #(#pda_variants,)*
                #(#token_variants,)*
            }
        }
    }

    // =========================================================================
    // DECOMPRESS VARIANT IMPL
    // =========================================================================

    /// Generate `impl DecompressVariant for PackedLightAccountVariant`.
    fn generate_decompress_variant_impl(&self) -> TokenStream {
        let pda_arms: Vec<_> = self
            .pda_ctx_seeds
            .iter()
            .map(|info| {
                let variant_name = &info.variant_name;
                let packed_variant_type = format_ident!("Packed{}Variant", variant_name);
                let seed_count = info.seed_count;

                quote! {
                    Self::#variant_name(packed_data) => {
                        light_sdk::interface::prepare_account_for_decompression::<#seed_count, #packed_variant_type>(
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

        let token_arms: Vec<_> = self
            .token_seeds
            .iter()
            .map(|spec| {
                let variant_name = &spec.variant;
                let packed_seeds_name = format_ident!("Packed{}Seeds", variant_name);
                let seed_count = spec.seeds.len() + 1;

                quote! {
                    Self::#variant_name(packed_data) => {
                        light_sdk::interface::token::prepare_token_account_for_decompression::<
                            #seed_count,
                            light_sdk::interface::token::TokenDataWithPackedSeeds<#packed_seeds_name>,
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

        quote! {
            impl<'info> light_sdk::interface::DecompressVariant<'info> for PackedLightAccountVariant {
                fn decompress(
                    &self,
                    tree_info: &light_sdk::instruction::PackedStateTreeInfo,
                    pda_account: &anchor_lang::prelude::AccountInfo<'info>,
                    ctx: &mut light_sdk::interface::DecompressCtx<'_, 'info>,
                ) -> std::result::Result<(), solana_program_error::ProgramError> {
                    let output_queue_index = ctx.output_queue_index;
                    match self {
                        #(#pda_arms)*
                        #(#token_arms)*
                    }
                }
            }
        }
    }

    // =========================================================================
    // PACK IMPL
    // =========================================================================

    /// Generate `impl light_sdk::Pack for LightAccountVariant`.
    fn generate_pack_impl(&self) -> TokenStream {
        let pda_arms: Vec<_> = self
            .pda_ctx_seeds
            .iter()
            .map(|info| {
                let variant_name = &info.variant_name;

                quote! {
                    Self::#variant_name(variant) => {
                        let packed = light_sdk::Pack::pack(variant, accounts)?;
                        Ok(PackedLightAccountVariant::#variant_name(packed))
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
                        let packed = light_sdk::Pack::pack(data, accounts)?;
                        Ok(PackedLightAccountVariant::#variant_name(packed))
                    }
                }
            })
            .collect();

        quote! {
            // Pack trait is only available off-chain (client-side)
            #[cfg(not(target_os = "solana"))]
            impl light_sdk::Pack for LightAccountVariant {
                type Packed = PackedLightAccountVariant;

                fn pack(
                    &self,
                    accounts: &mut light_sdk::instruction::PackedAccounts,
                ) -> std::result::Result<Self::Packed, solana_program_error::ProgramError> {
                    match self {
                        #(#pda_arms)*
                        #(#token_arms)*
                    }
                }
            }
        }
    }
}

// =============================================================================
// PdaCtxSeedInfo
// =============================================================================

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
fn seed_to_packed_ref(seed: &SeedElement) -> TokenStream {
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
                        .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                        .key
                        .as_ref()
                };
            }
            quote! { { let __seed: &[u8] = (#expr).as_ref(); __seed } }
        }
    }
}
