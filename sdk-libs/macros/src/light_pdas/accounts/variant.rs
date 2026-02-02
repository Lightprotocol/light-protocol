//! Variant generation for LightAccounts derive macro.
//!
//! This module generates per-field variant types and trait implementations for
//! PDA fields marked with `#[light_account(init)]`.
//!
//! For each PDA field, generates:
//! - `{Field}Seeds` - Struct containing dynamic seed values
//! - `Packed{Field}Seeds` - Packed version with u8 indices + bump
//! - `{Field}Variant` - Full variant combining seeds + data
//! - `Packed{Field}Variant` - Packed variant for efficient serialization
//! - `impl LightAccountVariant<N>` - Trait implementation for unpacked variant
//! - `impl PackedLightAccountVariant<N>` - Trait implementation for packed variant

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Type};

use crate::light_pdas::{
    seeds::{ClassifiedSeed, FnArgKind},
    shared_utils::make_packed_type,
};

/// Information about a single seed for code generation.
#[derive(Clone, Debug)]
pub(super) struct SeedFieldInfo {
    /// The field name in the seeds struct (e.g., `authority`, `owner`)
    pub field_name: Ident,
    /// The type of the field in the unpacked seeds struct (e.g., `Pubkey`, `u64`)
    pub field_type: TokenStream,
    /// The type of the field in the packed seeds struct (e.g., `u8` for idx, `[u8; 8]` for nonce)
    pub packed_field_type: TokenStream,
    /// Whether this is an account seed (needs u8 index in packed form)
    pub is_account_seed: bool,
    /// Whether the original expression uses to_le_bytes (indicates numeric type)
    pub has_le_bytes: bool,
}

/// Builder for generating variant code for a single PDA field.
pub(crate) struct VariantBuilder {
    /// The field name from the Accounts struct (e.g., `user_record`)
    /// Kept for future use (e.g., error messages, debugging)
    #[allow(dead_code)]
    field_name: Ident,
    /// The variant name in PascalCase (e.g., `UserRecord`)
    variant_name: Ident,
    /// The inner data type (e.g., `UserRecord`)
    inner_type: Type,
    /// Classified seeds from the `#[account(seeds = [...])]` attribute
    seeds: Vec<ClassifiedSeed>,
    /// Extracted seed field information for code generation
    seed_fields: Vec<SeedFieldInfo>,
    /// Number of seeds including bump (for const generic)
    seed_count: usize,
    /// Whether this is a zero-copy account (AccountLoader)
    #[allow(dead_code)]
    is_zero_copy: bool,
    /// The module path where the Accounts struct is defined (e.g., "crate::instructions::create")
    /// Used to qualify bare constant names in seed expressions.
    module_path: Option<String>,
}

impl VariantBuilder {
    /// Create from ExtractedSeedSpec (used by #[light_program]).
    pub fn from_extracted_spec(spec: &crate::light_pdas::seeds::ExtractedSeedSpec) -> Self {
        let field_name = to_snake_case_ident(&spec.variant_name);
        let variant_name = spec.variant_name.clone();
        // Qualify inner_type with crate:: if not already qualified
        let inner_type = crate::light_pdas::shared_utils::qualify_type_with_crate(&spec.inner_type);
        let seeds = spec.seeds.clone();
        let is_zero_copy = spec.is_zero_copy;

        let seed_fields = extract_seed_fields(&seeds);
        let seed_count = seeds.len() + 1;

        Self {
            field_name,
            variant_name,
            inner_type,
            seeds,
            seed_fields,
            seed_count,
            is_zero_copy,
            module_path: Some(spec.module_path.clone()),
        }
    }

    /// Generate all variant code for this PDA field.
    pub fn build(&self) -> TokenStream {
        let seeds_struct = self.generate_seeds_struct();
        let packed_seeds_struct = self.generate_packed_seeds_struct();
        let variant_struct = self.generate_variant_struct();
        let packed_variant_struct = self.generate_packed_variant_struct();
        let light_account_variant_impl = self.generate_light_account_variant_impl();
        let packed_light_account_variant_impl = self.generate_packed_light_account_variant_impl();
        let pack_impl = self.generate_pack_impl();

        quote! {
            #seeds_struct
            #packed_seeds_struct
            #variant_struct
            #packed_variant_struct
            #light_account_variant_impl
            #packed_light_account_variant_impl
            #pack_impl
        }
    }

    /// Generate the `{Field}Seeds` struct.
    fn generate_seeds_struct(&self) -> TokenStream {
        let struct_name = format_ident!("{}Seeds", self.variant_name);
        let doc = format!("Seeds for {} PDA.", self.variant_name);

        // Filter to only account and data seeds (constants are inline)
        let fields: Vec<_> = self
            .seed_fields
            .iter()
            .map(|sf| {
                let name = &sf.field_name;
                let ty = &sf.field_type;
                quote! { pub #name: #ty }
            })
            .collect();

        // AnchorSerialize derive provides IdlBuild impl when idl-build feature is enabled
        if fields.is_empty() {
            quote! {
                #[doc = #doc]
                #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
                pub struct #struct_name;
            }
        } else {
            quote! {
                #[doc = #doc]
                #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
                pub struct #struct_name {
                    #(#fields,)*
                }
            }
        }
    }

    /// Generate the `Packed{Field}Seeds` struct.
    fn generate_packed_seeds_struct(&self) -> TokenStream {
        let struct_name = format_ident!("Packed{}Seeds", self.variant_name);
        let doc = format!(
            "Packed seeds with u8 indices for {} PDA.",
            self.variant_name
        );

        // Generate packed fields
        let fields: Vec<_> = self
            .seed_fields
            .iter()
            .map(|sf| {
                let name = if sf.is_account_seed {
                    format_ident!("{}_idx", sf.field_name)
                } else {
                    sf.field_name.clone()
                };
                let ty = &sf.packed_field_type;
                quote! { pub #name: #ty }
            })
            .collect();

        quote! {
            #[doc = #doc]
            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
            pub struct #struct_name {
                #(#fields,)*
                pub bump: u8,
            }
        }
    }

    /// Generate the `{Field}Variant` struct.
    fn generate_variant_struct(&self) -> TokenStream {
        let struct_name = format_ident!("{}Variant", self.variant_name);
        let seeds_struct_name = format_ident!("{}Seeds", self.variant_name);
        let inner_type = &self.inner_type;
        let doc = format!(
            "Full variant combining seeds + data for {}.",
            self.variant_name
        );

        quote! {
            #[doc = #doc]
            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
            pub struct #struct_name {
                pub seeds: #seeds_struct_name,
                pub data: #inner_type,
            }
        }
    }

    /// Generate the `Packed{Field}Variant` struct.
    fn generate_packed_variant_struct(&self) -> TokenStream {
        let struct_name = format_ident!("Packed{}Variant", self.variant_name);
        let packed_seeds_struct_name = format_ident!("Packed{}Seeds", self.variant_name);
        let inner_type = &self.inner_type;
        let doc = format!(
            "Packed variant for efficient serialization of {}.",
            self.variant_name
        );

        // Use packed data type for all accounts (including zero-copy)
        // Zero-copy accounts use the same LightAccount::Packed pattern as regular accounts
        let data_type = if let Some(packed_type) = make_packed_type(inner_type) {
            quote! { #packed_type }
        } else {
            // Fallback: prepend "Packed" to the type name
            let type_str = quote!(#inner_type).to_string().replace(' ', "");
            let packed_name = format_ident!("Packed{}", type_str);
            quote! { #packed_name }
        };

        quote! {
            #[doc = #doc]
            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
            pub struct #struct_name {
                pub seeds: #packed_seeds_struct_name,
                pub data: #data_type,
            }
        }
    }

    /// Generate `impl LightAccountVariant<N>` for the variant struct.
    fn generate_light_account_variant_impl(&self) -> TokenStream {
        let variant_name = format_ident!("{}Variant", self.variant_name);
        let seeds_struct_name = format_ident!("{}Seeds", self.variant_name);
        let packed_variant_name = format_ident!("Packed{}Variant", self.variant_name);
        let inner_type = &self.inner_type;
        let seed_count = self.seed_count;

        // Generate seed_vec body
        let seed_vec_items = self.generate_seed_vec_items();

        // Generate seed_refs_with_bump body
        let seed_refs_items = self.generate_seed_refs_items();

        // NOTE: pack() is NOT generated here - it's in the Pack trait impl (off-chain only)

        quote! {
            impl light_account::LightAccountVariantTrait<#seed_count> for #variant_name {
                const PROGRAM_ID: Pubkey = crate::ID;

                type Seeds = #seeds_struct_name;
                type Data = #inner_type;
                type Packed = #packed_variant_name;

                fn data(&self) -> &Self::Data {
                    &self.data
                }

                fn seed_vec(&self) -> Vec<Vec<u8>> {
                    vec![#(#seed_vec_items),*]
                }

                fn seed_refs_with_bump<'a>(&'a self, bump_storage: &'a [u8; 1]) -> [&'a [u8]; #seed_count] {
                    [#(#seed_refs_items,)* bump_storage]
                }
            }
        }
    }

    /// Generate `impl PackedLightAccountVariant<N>` for the packed variant struct.
    fn generate_packed_light_account_variant_impl(&self) -> TokenStream {
        let variant_name = format_ident!("{}Variant", self.variant_name);
        let seeds_struct_name = format_ident!("{}Seeds", self.variant_name);
        let packed_variant_name = format_ident!("Packed{}Variant", self.variant_name);
        let inner_type = &self.inner_type;
        let seed_count = self.seed_count;

        // Generate unpack seed fields
        let unpack_seed_stmts = self.generate_unpack_seed_statements(false);
        let unpack_seed_fields = self.generate_unpack_seed_fields();

        // Generate seed_refs_with_bump body for packed variant
        let packed_seed_refs_items = self.generate_packed_seed_refs_items();

        // Use LightAccount::unpack for all accounts (including zero-copy)
        // Build ProgramPackedAccounts from the accounts slice
        let unpack_data = quote! {
            {
                let packed_accounts = light_sdk::light_account_checks::packed_accounts::ProgramPackedAccounts { accounts };
                <#inner_type as light_account::LightAccount>::unpack(&self.data, &packed_accounts)
                    .map_err(|_| anchor_lang::error::ErrorCode::InvalidProgramId)?
            }
        };

        quote! {
            impl light_account::PackedLightAccountVariantTrait<#seed_count> for #packed_variant_name {
                type Unpacked = #variant_name;

                const ACCOUNT_TYPE: light_account::AccountType =
                    <#inner_type as light_account::LightAccount>::ACCOUNT_TYPE;

                fn bump(&self) -> u8 {
                    self.seeds.bump
                }

                fn unpack(&self, accounts: &[anchor_lang::prelude::AccountInfo]) -> anchor_lang::Result<Self::Unpacked> {
                    #(#unpack_seed_stmts)*

                    Ok(#variant_name {
                        seeds: #seeds_struct_name {
                            #(#unpack_seed_fields,)*
                        },
                        data: #unpack_data,
                    })
                }

                fn seed_refs_with_bump<'a>(
                    &'a self,
                    accounts: &'a [anchor_lang::prelude::AccountInfo],
                    bump_storage: &'a [u8; 1],
                ) -> std::result::Result<[&'a [u8]; #seed_count], solana_program_error::ProgramError> {
                    Ok([#(#packed_seed_refs_items,)* bump_storage])
                }

                fn into_in_token_data(&self, _tree_info: &light_sdk::instruction::PackedStateTreeInfo, _output_queue_index: u8) -> anchor_lang::Result<light_account::token::MultiInputTokenDataWithContext> {
                    Err(solana_program_error::ProgramError::InvalidAccountData.into())
                }

                fn into_in_tlv(&self) -> anchor_lang::Result<Option<Vec<light_account::token::ExtensionInstructionData>>> {
                    Ok(None)
                }
            }
        }
    }

    /// Generate `impl Pack` for the variant struct.
    ///
    /// This is off-chain only (client-side packing). Gated with `#[cfg(not(target_os = "solana"))]`.
    fn generate_pack_impl(&self) -> TokenStream {
        let variant_name = format_ident!("{}Variant", self.variant_name);
        let packed_variant_name = format_ident!("Packed{}Variant", self.variant_name);
        let packed_seeds_struct_name = format_ident!("Packed{}Seeds", self.variant_name);
        let inner_type = &self.inner_type;

        // Generate pack body for seed fields
        let pack_seed_fields = self.generate_pack_seed_fields();

        // Use LightAccount::pack for all accounts (including zero-copy)
        let pack_data = quote! {
            <#inner_type as light_account::LightAccount>::pack(&self.data, accounts)
                .map_err(|_| solana_program_error::ProgramError::InvalidAccountData)?
        };

        quote! {
            // Pack trait is only available off-chain (client-side packing)
            #[cfg(not(target_os = "solana"))]
            impl light_sdk::Pack for #variant_name {
                type Packed = #packed_variant_name;

                fn pack(
                    &self,
                    accounts: &mut light_sdk::instruction::PackedAccounts,
                ) -> std::result::Result<Self::Packed, solana_program_error::ProgramError> {
                    use light_account::LightAccountVariantTrait;
                    let (_, bump) = self.derive_pda();
                    Ok(#packed_variant_name {
                        seeds: #packed_seeds_struct_name {
                            #(#pack_seed_fields,)*
                            bump,
                        },
                        data: #pack_data,
                    })
                }
            }
        }
    }

    /// Generate seed_vec items for each seed.
    fn generate_seed_vec_items(&self) -> Vec<TokenStream> {
        self.seeds
            .iter()
            .map(|seed| match seed {
                ClassifiedSeed::Literal(_)
                | ClassifiedSeed::Constant { .. }
                | ClassifiedSeed::Passthrough(_) => {
                    let expr = seed_to_expr(seed, self.module_path.as_deref());
                    quote! { (#expr).to_vec() }
                }
                ClassifiedSeed::CtxRooted { account, .. } => {
                    quote! { self.seeds.#account.to_bytes().to_vec() }
                }
                ClassifiedSeed::DataRooted { root, expr, .. } => {
                    let field = extract_data_field_name(root, expr);
                    if is_le_bytes_expr(expr) {
                        quote! { self.seeds.#field.to_le_bytes().to_vec() }
                    } else {
                        quote! { self.seeds.#field.to_bytes().to_vec() }
                    }
                }
                ClassifiedSeed::FunctionCall {
                    func_expr,
                    args,
                    has_as_ref,
                } => {
                    // Reconstruct call with self.seeds.X args
                    let rewritten = rewrite_fn_call_for_self(func_expr, args);
                    if *has_as_ref {
                        quote! { #rewritten.as_ref().to_vec() }
                    } else {
                        quote! { (#rewritten).to_vec() }
                    }
                }
            })
            .collect()
    }

    /// Generate seed_refs_with_bump items for unpacked variant.
    fn generate_seed_refs_items(&self) -> Vec<TokenStream> {
        self.seeds
            .iter()
            .map(|seed| match seed {
                ClassifiedSeed::Literal(_) | ClassifiedSeed::Constant { .. } => {
                    let expr = seed_to_expr(seed, self.module_path.as_deref());
                    quote! { #expr }
                }
                ClassifiedSeed::Passthrough(pass_expr) => {
                    // Check if the expression contains a function call that creates a temporary.
                    // E.g., crate::id().as_ref() -- the Pubkey temporary is dropped before the
                    // returned array reference is used.
                    if expr_contains_call(pass_expr) {
                        // Use a typed block to avoid `!` type causing unreachable expression warnings
                        // in the surrounding array literal.
                        quote! {
                            {
                                panic!("seed_refs_with_bump not supported for function call seeds on unpacked variant. \
                                       Use packed variant or derive_pda() + seed_vec() instead.");
                                #[allow(unreachable_code)]
                                { bump_storage as &[u8] }
                            }
                        }
                    } else {
                        let expr = seed_to_expr(seed, self.module_path.as_deref());
                        quote! { #expr }
                    }
                }
                ClassifiedSeed::CtxRooted { account, .. } => {
                    quote! { self.seeds.#account.as_ref() }
                }
                ClassifiedSeed::DataRooted { root, expr, .. } => {
                    let field = extract_data_field_name(root, expr);
                    if is_le_bytes_expr(expr) {
                        // Numeric data seeds: can't return reference to temporary.
                        // Use a typed block to avoid `!` type causing unreachable expression warnings.
                        quote! {
                            {
                                panic!("seed_refs_with_bump not supported for numeric data seeds on unpacked variant. \
                                       Use packed variant or derive_pda() + seed_vec() instead.");
                                #[allow(unreachable_code)]
                                { bump_storage as &[u8] }
                            }
                        }
                    } else {
                        quote! { self.seeds.#field.as_ref() }
                    }
                }
                ClassifiedSeed::FunctionCall { .. } => {
                    // FunctionCall produces temporaries -- can't use seed_refs_with_bump.
                    // Use a typed block to avoid `!` type causing unreachable expression warnings.
                    quote! {
                        {
                            panic!("seed_refs_with_bump not supported for function call seeds on unpacked variant. \
                                   Use packed variant or derive_pda() + seed_vec() instead.");
                            #[allow(unreachable_code)]
                            { bump_storage as &[u8] }
                        }
                    }
                }
            })
            .collect()
    }

    /// Generate pack statements for seed fields.
    fn generate_pack_seed_fields(&self) -> Vec<TokenStream> {
        self.seed_fields
            .iter()
            .map(|sf| {
                let field = &sf.field_name;
                if sf.is_account_seed {
                    let idx_field = format_ident!("{}_idx", field);
                    quote! { #idx_field: accounts.insert_or_get(self.seeds.#field) }
                } else if sf.has_le_bytes {
                    quote! { #field: self.seeds.#field.to_le_bytes() }
                } else {
                    quote! { #field: self.seeds.#field }
                }
            })
            .collect()
    }

    /// Generate unpack statements to resolve indices to Pubkeys.
    ///
    /// Used in `unpack()` which returns `anchor_lang::Result`.
    fn generate_unpack_seed_statements(&self, _for_program_error: bool) -> Vec<TokenStream> {
        self.seed_fields
            .iter()
            .filter(|sf| sf.is_account_seed)
            .map(|sf| {
                let field = &sf.field_name;
                let idx_field = format_ident!("{}_idx", field);
                quote! {
                    let #field = *accounts
                        .get(self.seeds.#idx_field as usize)
                        .ok_or(anchor_lang::error::ErrorCode::AccountNotEnoughKeys)?
                        .key;
                }
            })
            .collect()
    }

    /// Generate unpack seed field assignments.
    fn generate_unpack_seed_fields(&self) -> Vec<TokenStream> {
        self.seed_fields
            .iter()
            .map(|sf| {
                let field = &sf.field_name;
                if sf.is_account_seed {
                    quote! { #field }
                } else if sf.has_le_bytes {
                    let ty = &sf.field_type;
                    quote! { #field: #ty::from_le_bytes(self.seeds.#field) }
                } else {
                    quote! { #field: self.seeds.#field }
                }
            })
            .collect()
    }

    /// Generate seed_refs_with_bump items for packed variant.
    ///
    /// For packed variant, account seeds are looked up directly from the accounts slice
    /// using inline expressions (borrowing from `accounts` with lifetime `'a`).
    /// Data seeds are stored directly in the packed struct (borrowing from `&'a self`).
    ///
    /// Account lookups are inlined rather than bound to local variables to avoid
    /// E0515 (cannot return value referencing local variable).
    fn generate_packed_seed_refs_items(&self) -> Vec<TokenStream> {
        self.seeds
            .iter()
            .map(|seed| match seed {
                ClassifiedSeed::Literal(_) | ClassifiedSeed::Constant { .. } => {
                    let expr = seed_to_expr(seed, self.module_path.as_deref());
                    quote! { #expr }
                }
                ClassifiedSeed::Passthrough(pass_expr) => {
                    if expr_contains_call(pass_expr) {
                        // Use a typed block to avoid `!` type causing unreachable expression warnings.
                        quote! {
                            {
                                panic!("seed_refs_with_bump not supported for function call seeds on packed variant. \
                                       Use derive_pda() + seed_vec() instead.");
                                #[allow(unreachable_code)]
                                { bump_storage as &[u8] }
                            }
                        }
                    } else {
                        let expr = seed_to_expr(seed, self.module_path.as_deref());
                        quote! { #expr }
                    }
                }
                ClassifiedSeed::CtxRooted { account, .. } => {
                    // Inline account lookup to borrow from `accounts` (lifetime 'a)
                    let idx_field = format_ident!("{}_idx", account);
                    quote! {
                        accounts
                            .get(self.seeds.#idx_field as usize)
                            .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                            .key
                            .as_ref()
                    }
                }
                ClassifiedSeed::DataRooted { root, expr, .. } => {
                    let field = extract_data_field_name(root, expr);
                    if is_le_bytes_expr(expr) {
                        quote! { &self.seeds.#field }
                    } else {
                        quote! { self.seeds.#field.as_ref() }
                    }
                }
                ClassifiedSeed::FunctionCall { .. } => {
                    // FunctionCall args are packed as individual fields (account = idx, data = Pubkey)
                    // The packed_seed_refs_items needs the full reconstructed seed, but that's
                    // impossible without temporary allocations.
                    // Use a typed block to avoid `!` type causing unreachable expression warnings.
                    quote! {
                        {
                            panic!("seed_refs_with_bump not supported for function call seeds on packed variant. \
                                   Use derive_pda() + seed_vec() instead.");
                            #[allow(unreachable_code)]
                            { bump_storage as &[u8] }
                        }
                    }
                }
            })
            .collect()
    }
}

/// Extract seed field information from classified seeds.
fn extract_seed_fields(seeds: &[ClassifiedSeed]) -> Vec<SeedFieldInfo> {
    let mut fields = Vec::new();

    for seed in seeds {
        match seed {
            ClassifiedSeed::Literal(_)
            | ClassifiedSeed::Constant { .. }
            | ClassifiedSeed::Passthrough(_) => {
                // Constants/literals/passthrough don't need fields - inlined
            }
            ClassifiedSeed::CtxRooted { account, .. } => {
                fields.push(SeedFieldInfo {
                    field_name: account.clone(),
                    field_type: quote! { Pubkey },
                    packed_field_type: quote! { u8 },
                    is_account_seed: true,
                    has_le_bytes: false,
                });
            }
            ClassifiedSeed::DataRooted { root, expr, .. } => {
                let field_name = extract_data_field_name(root, expr);
                if is_le_bytes_expr(expr) {
                    fields.push(SeedFieldInfo {
                        field_name,
                        field_type: quote! { u64 },
                        packed_field_type: quote! { [u8; 8] },
                        is_account_seed: false,
                        has_le_bytes: true,
                    });
                } else {
                    fields.push(SeedFieldInfo {
                        field_name,
                        field_type: quote! { Pubkey },
                        packed_field_type: quote! { Pubkey },
                        is_account_seed: false,
                        has_le_bytes: false,
                    });
                }
            }
            ClassifiedSeed::FunctionCall { args, .. } => {
                // One field per classified argument
                for arg in args {
                    match arg.kind {
                        FnArgKind::CtxAccount => {
                            fields.push(SeedFieldInfo {
                                field_name: arg.field_name.clone(),
                                field_type: quote! { Pubkey },
                                packed_field_type: quote! { u8 },
                                is_account_seed: true,
                                has_le_bytes: false,
                            });
                        }
                        FnArgKind::DataField => {
                            fields.push(SeedFieldInfo {
                                field_name: arg.field_name.clone(),
                                field_type: quote! { Pubkey },
                                packed_field_type: quote! { Pubkey },
                                is_account_seed: false,
                                has_le_bytes: false,
                            });
                        }
                    }
                }
            }
        }
    }

    fields
}

/// Convert a ClassifiedSeed to a token expression for inline code generation.
/// Constants are qualified with `crate::` to ensure they're accessible.
fn seed_to_expr(seed: &ClassifiedSeed, _module_path: Option<&str>) -> TokenStream {
    match seed {
        ClassifiedSeed::Literal(bytes) => {
            let byte_values: Vec<_> = bytes.iter().map(|b| quote!(#b)).collect();
            quote! { &[#(#byte_values),*] }
        }
        ClassifiedSeed::Constant { path, expr } => {
            // Qualify constant path with crate:: if not already qualified
            let qualified_path = qualify_path_with_crate(path);
            // Reconstruct the expression with the qualified path
            reconstruct_expr_with_qualified_path(expr, path, &qualified_path)
        }
        ClassifiedSeed::Passthrough(expr) => {
            quote! { #expr }
        }
        _ => unreachable!("seed_to_expr called with non-inline seed"),
    }
}

/// Reserved constant names that conflict with Solana runtime.
/// `A` is used by the BumpAllocator in Solana programs.
const RESERVED_CONSTANT_NAMES: &[&str] = &["A"];

/// Qualify a path with `crate::` if it's not already qualified.
/// Panics if the path uses a reserved name like `A` (BumpAllocator).
fn qualify_path_with_crate(path: &syn::Path) -> syn::Path {
    // Check if already qualified (crate::, super::, self::, or external crate)
    if let Some(first_segment) = path.segments.first() {
        let first_ident = first_segment.ident.to_string();
        if first_ident == "crate" || first_ident == "super" || first_ident == "self" {
            return path.clone();
        }
        // Check for external crate paths (contains ::)
        if path.segments.len() > 1 {
            // Likely already qualified with module path
            return path.clone();
        }
        // Check for reserved names that conflict with Solana runtime
        if RESERVED_CONSTANT_NAMES.contains(&first_ident.as_str()) {
            panic!(
                "Seed constant '{}' is reserved (conflicts with Solana BumpAllocator). \
                 Please rename your constant.",
                first_ident
            );
        }
    }
    // Prepend crate:: to the path
    let mut qualified = syn::Path {
        leading_colon: None,
        segments: syn::punctuated::Punctuated::new(),
    };
    qualified.segments.push(syn::PathSegment {
        ident: format_ident!("crate"),
        arguments: syn::PathArguments::None,
    });
    for segment in &path.segments {
        qualified.segments.push(segment.clone());
    }
    qualified
}

/// Reconstruct an expression replacing the original path with a qualified one.
fn reconstruct_expr_with_qualified_path(
    expr: &syn::Expr,
    original_path: &syn::Path,
    qualified_path: &syn::Path,
) -> TokenStream {
    // If the expression is just a path, return the qualified path
    if let syn::Expr::Path(expr_path) = expr {
        if paths_equal(&expr_path.path, original_path) {
            return quote! { #qualified_path };
        }
    }

    // For method calls like CONSTANT.as_bytes(), replace the receiver
    if let syn::Expr::MethodCall(method_call) = expr {
        if let syn::Expr::Path(receiver_path) = method_call.receiver.as_ref() {
            if paths_equal(&receiver_path.path, original_path) {
                let method = &method_call.method;
                let args = &method_call.args;
                return quote! { #qualified_path.#method(#args) };
            }
        }
        // Handle chained method calls like CONSTANT.as_bytes().as_ref()
        let rewritten_receiver = reconstruct_expr_with_qualified_path(
            &method_call.receiver,
            original_path,
            qualified_path,
        );
        let method = &method_call.method;
        let args = &method_call.args;
        return quote! { #rewritten_receiver.#method(#args) };
    }

    // For reference expressions like &CONSTANT
    if let syn::Expr::Reference(ref_expr) = expr {
        let rewritten_inner =
            reconstruct_expr_with_qualified_path(&ref_expr.expr, original_path, qualified_path);
        let mutability = &ref_expr.mutability;
        return quote! { &#mutability #rewritten_inner };
    }

    // Fallback: return original expression
    quote! { #expr }
}

/// Check if two paths are equal.
fn paths_equal(a: &syn::Path, b: &syn::Path) -> bool {
    if a.segments.len() != b.segments.len() {
        return false;
    }
    a.segments
        .iter()
        .zip(b.segments.iter())
        .all(|(seg_a, seg_b)| seg_a.ident == seg_b.ident)
}

/// Check if a DataRooted expression uses to_le_bytes (indicates numeric type).
fn is_le_bytes_expr(expr: &syn::Expr) -> bool {
    let expr_str = quote!(#expr).to_string();
    expr_str.contains("to_le_bytes")
}

/// Check if an expression contains a function call (Expr::Call).
/// Used to detect Passthrough seeds that create temporaries, e.g. `crate::id().as_ref()`.
fn expr_contains_call(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Call(_) => true,
        syn::Expr::MethodCall(mc) => expr_contains_call(&mc.receiver),
        syn::Expr::Reference(r) => expr_contains_call(&r.expr),
        syn::Expr::Paren(p) => expr_contains_call(&p.expr),
        _ => false,
    }
}

/// Extract the terminal field name from a DataRooted seed expression.
/// For `params.owner.as_ref()` returns `owner`.
/// For `params.nonce.to_le_bytes()` returns `nonce`.
/// Falls back to the root identifier if no field access found.
fn extract_data_field_name(root: &Ident, expr: &syn::Expr) -> Ident {
    // Use the extraction helper from seed_extraction
    crate::light_pdas::seeds::extract_data_field_name_from_expr(expr)
        .unwrap_or_else(|| root.clone())
}

/// Rewrite a function call expression so each classified arg uses `self.seeds.X`.
fn rewrite_fn_call_for_self(
    func_expr: &syn::Expr,
    args: &[crate::light_pdas::seeds::ClassifiedFnArg],
) -> TokenStream {
    // Clone the call expression and rewrite its arguments
    if let syn::Expr::Call(call) = func_expr {
        let func_path = &call.func;
        let rewritten_args: Vec<_> = call
            .args
            .iter()
            .map(|arg| {
                // Check if this arg matches any classified arg
                for classified in args {
                    let field = &classified.field_name;
                    // Match by checking if the arg expression contains the field name
                    let arg_str = quote!(#arg).to_string();
                    let field_str = field.to_string();
                    if arg_str.contains(&field_str) {
                        return quote! { &self.seeds.#field };
                    }
                }
                // Non-dynamic arg: pass through
                quote! { #arg }
            })
            .collect();
        quote! { #func_path(#(#rewritten_args),*) }
    } else {
        // Shouldn't happen, but safe fallback
        quote! { #func_expr }
    }
}

/// Convert a PascalCase identifier to snake_case.
fn to_snake_case_ident(ident: &Ident) -> Ident {
    use crate::utils::to_snake_case;
    let snake = to_snake_case(&ident.to_string());
    format_ident!("{}", snake)
}
