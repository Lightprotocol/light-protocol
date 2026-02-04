//! Manual `#[derive(LightProgram)]` macro implementation.
//!
//! Allows specifying compressed account variants on an enum, generating equivalent
//! code to `#[light_program]` auto-discovery. Useful for external programs where
//! you can't add `#[light_program]` to the module.
//!
//! ## Syntax
//!
//! ```ignore
//! #[derive(LightProgram)]
//! pub enum ProgramAccounts {
//!     #[light_account(pda::seeds = [b"record", ctx.owner])]
//!     Record(MinimalRecord),
//!
//!     #[light_account(pda::seeds = [RECORD_SEED, ctx.owner], pda::zero_copy)]
//!     ZeroCopyRecord(ZeroCopyRecord),
//!
//!     #[light_account(token::seeds = [VAULT_SEED, ctx.mint], token::owner_seeds = [VAULT_AUTH_SEED])]
//!     Vault,
//!
//!     #[light_account(associated_token)]
//!     Ata,
//! }
//! ```

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse::ParseStream, DeriveInput, Ident, Result, Token, Type};

use super::instructions::{
    generate_light_program_items, CompressibleAccountInfo, InstructionDataSpec, SeedElement,
    TokenSeedSpec,
};
use crate::light_pdas::{
    accounts::variant::VariantBuilder,
    light_account_keywords::validate_namespaced_key,
    parsing::CrateContext,
    seeds::{ClassifiedSeed, ExtractedSeedSpec},
    shared_utils::is_constant_identifier,
};

// =============================================================================
// PARSING TYPES
// =============================================================================

/// Kind of a manual variant in the enum.
#[derive(Clone, Debug)]
enum ManualVariantKind {
    Pda,
    Token,
    Ata,
}

/// A single seed element parsed from the attribute.
#[derive(Clone, Debug)]
enum ManualSeed {
    /// b"literal" - byte string literal
    ByteLiteral(syn::LitByteStr),
    /// "literal" - string literal (converted to bytes)
    StrLiteral(syn::LitStr),
    /// CONSTANT or path::CONSTANT
    Constant(syn::Path),
    /// ctx.field - context account reference
    CtxField(Ident),
    /// data.field - instruction data reference
    DataField(Ident),
}

/// Parsed variant from the manual enum.
#[derive(Clone, Debug)]
struct ParsedManualVariant {
    ident: Ident,
    kind: ManualVariantKind,
    inner_type: Option<Type>,
    is_zero_copy: bool,
    seeds: Vec<ManualSeed>,
    owner_seeds: Option<Vec<ManualSeed>>,
}

// =============================================================================
// PARSING
// =============================================================================

/// Parse all variants from the derive input enum.
fn parse_enum_variants(input: &DeriveInput) -> Result<Vec<ParsedManualVariant>> {
    let data = match &input.data {
        syn::Data::Enum(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "#[derive(LightProgram)] can only be applied to enums",
            ))
        }
    };

    let mut variants = Vec::new();
    for variant in &data.variants {
        // Find the #[light_account(...)] attribute
        let attr = variant
            .attrs
            .iter()
            .find(|a| a.path().is_ident("light_account"))
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    &variant.ident,
                    format!(
                        "Variant '{}' is missing #[light_account(...)] attribute",
                        variant.ident
                    ),
                )
            })?;

        let parsed = parse_variant_attr(attr, &variant.ident, &variant.fields)?;
        variants.push(parsed);
    }

    Ok(variants)
}

/// Parse a single variant's `#[light_account(...)]` attribute.
fn parse_variant_attr(
    attr: &syn::Attribute,
    variant_ident: &Ident,
    fields: &syn::Fields,
) -> Result<ParsedManualVariant> {
    let tokens: TokenStream = attr.parse_args()?;
    let parsed: VariantAttrContent = syn::parse2(tokens)?;

    // Extract inner type from tuple field for PDA variants
    let inner_type = match &parsed.kind {
        ManualVariantKind::Pda => {
            let ty = extract_inner_type(variant_ident, fields)?;
            Some(ty)
        }
        ManualVariantKind::Token | ManualVariantKind::Ata => {
            // Ensure unit variant
            if !matches!(fields, syn::Fields::Unit) {
                return Err(syn::Error::new_spanned(
                    variant_ident,
                    format!(
                        "Token/ATA variant '{}' must be a unit variant (no fields)",
                        variant_ident
                    ),
                ));
            }
            None
        }
    };

    Ok(ParsedManualVariant {
        ident: variant_ident.clone(),
        kind: parsed.kind,
        inner_type,
        is_zero_copy: parsed.is_zero_copy,
        seeds: parsed.seeds,
        owner_seeds: parsed.owner_seeds,
    })
}

/// Extract inner type from a tuple variant's first field.
fn extract_inner_type(variant_ident: &Ident, fields: &syn::Fields) -> Result<Type> {
    match fields {
        syn::Fields::Unnamed(unnamed) => {
            if unnamed.unnamed.len() != 1 {
                return Err(syn::Error::new_spanned(
                    variant_ident,
                    format!(
                        "PDA variant '{}' must have exactly one field (the data type)",
                        variant_ident
                    ),
                ));
            }
            Ok(unnamed.unnamed[0].ty.clone())
        }
        _ => Err(syn::Error::new_spanned(
            variant_ident,
            format!(
                "PDA variant '{}' must be a tuple variant with the data type, e.g., {}(MyRecord)",
                variant_ident, variant_ident
            ),
        )),
    }
}

// =============================================================================
// ATTRIBUTE CONTENT PARSING
// =============================================================================

/// Parsed content of `#[light_account(...)]`.
///
/// Kind is inferred from namespace prefix or standalone keyword:
/// - Any `pda::*` present -> PDA
/// - Any `token::*` present -> Token
/// - `associated_token` -> ATA
struct VariantAttrContent {
    kind: ManualVariantKind,
    is_zero_copy: bool,
    seeds: Vec<ManualSeed>,
    owner_seeds: Option<Vec<ManualSeed>>,
}

/// Tracks seen keywords/namespaces to detect duplicates and conflicts.
#[derive(Default)]
struct SeenDeriveKeywords {
    namespace: Option<String>,
    seen_keys: HashSet<String>,
}

impl SeenDeriveKeywords {
    /// Record a namespaced key. Returns error on mixed namespaces or duplicate keys.
    fn add_namespaced_key(&mut self, ns: &Ident, key: &Ident) -> Result<()> {
        let ns_str = ns.to_string();
        let key_str = key.to_string();

        if let Err(err_msg) = validate_namespaced_key(&ns_str, &key_str) {
            return Err(syn::Error::new_spanned(key, err_msg));
        }

        if let Some(ref prev_ns) = self.namespace {
            if prev_ns != &ns_str {
                return Err(syn::Error::new_spanned(
                    ns,
                    format!(
                        "Mixed namespaces: `{}::` conflicts with previous `{}::`. \
                         Each variant must use a single namespace.",
                        ns_str, prev_ns
                    ),
                ));
            }
        } else {
            self.namespace = Some(ns_str.clone());
        }

        if !self.seen_keys.insert(key_str.clone()) {
            return Err(syn::Error::new_spanned(
                key,
                format!(
                    "Duplicate key `{}::{}`. Each key can only appear once.",
                    ns_str, key_str
                ),
            ));
        }

        Ok(())
    }
}

/// Map namespace ident to ManualVariantKind.
fn infer_kind_from_namespace(ns: &Ident) -> Result<ManualVariantKind> {
    match ns.to_string().as_str() {
        "pda" => Ok(ManualVariantKind::Pda),
        "token" => Ok(ManualVariantKind::Token),
        _ => Err(syn::Error::new_spanned(
            ns,
            format!(
                "Unknown namespace `{}` for #[derive(LightProgram)]. \
                 Expected: `pda` or `token`. For ATA use `associated_token`. \
                 Mints are decompressed directly with the Light Token Program \
                 and don't need to be declared here.",
                ns
            ),
        )),
    }
}

/// Parse the value part of a namespaced key.
fn parse_namespaced_value(
    ns: &Ident,
    key: &Ident,
    input: ParseStream,
    seeds: &mut Vec<ManualSeed>,
    owner_seeds: &mut Option<Vec<ManualSeed>>,
    is_zero_copy: &mut bool,
) -> Result<()> {
    let ns_str = ns.to_string();
    let key_str = key.to_string();

    match (ns_str.as_str(), key_str.as_str()) {
        ("pda", "seeds") => {
            input.parse::<Token![=]>()?;
            *seeds = parse_seed_array(input)?;
        }
        ("pda", "zero_copy") => {
            *is_zero_copy = true;
        }
        ("token", "seeds") => {
            input.parse::<Token![=]>()?;
            *seeds = parse_seed_array(input)?;
        }
        ("token", "owner_seeds") => {
            input.parse::<Token![=]>()?;
            *owner_seeds = Some(parse_seed_array(input)?);
        }
        _ => {
            return Err(syn::Error::new_spanned(
                key,
                format!(
                    "Unsupported key `{}::{}` in #[derive(LightProgram)]",
                    ns_str, key_str
                ),
            ));
        }
    }
    Ok(())
}

impl syn::parse::Parse for VariantAttrContent {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut seen = SeenDeriveKeywords::default();
        let mut is_zero_copy = false;
        let mut seeds = Vec::new();
        let mut owner_seeds = None;

        // Parse first token to determine kind
        let first: Ident = input.parse()?;

        let kind = if first == "associated_token" {
            ManualVariantKind::Ata
        } else if input.peek(Token![::]) {
            // Namespaced key: pda::seeds, token::seeds, etc.
            input.parse::<Token![::]>()?;
            let key: Ident = input.parse()?;

            seen.add_namespaced_key(&first, &key)?;
            let k = infer_kind_from_namespace(&first)?;

            parse_namespaced_value(
                &first,
                &key,
                input,
                &mut seeds,
                &mut owner_seeds,
                &mut is_zero_copy,
            )?;
            k
        } else {
            return Err(syn::Error::new_spanned(
                &first,
                format!(
                    "Unknown keyword `{}`. Expected: `associated_token` \
                     or namespaced key like `pda::seeds`, `token::seeds`. \
                     Mints are decompressed directly with the Light Token Program \
                     and don't need to be declared here.",
                    first
                ),
            ));
        };

        // Parse remaining comma-separated items
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }

            let ident: Ident = input.parse()?;

            if !input.peek(Token![::]) {
                return Err(syn::Error::new_spanned(
                    &ident,
                    format!(
                        "Unexpected keyword `{}`. Use namespaced syntax: `pda::{}` or `token::{}`",
                        ident, ident, ident
                    ),
                ));
            }

            input.parse::<Token![::]>()?;
            let key: Ident = input.parse()?;

            seen.add_namespaced_key(&ident, &key)?;

            parse_namespaced_value(
                &ident,
                &key,
                input,
                &mut seeds,
                &mut owner_seeds,
                &mut is_zero_copy,
            )?;
        }

        // Post-parse validation

        match kind {
            ManualVariantKind::Pda => {
                if seeds.is_empty() {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        "PDA variant requires `pda::seeds = [...]`",
                    ));
                }
            }
            ManualVariantKind::Token => {
                if seeds.is_empty() {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        "Token variant requires `token::seeds = [...]`",
                    ));
                }
                if owner_seeds.is_none() {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        "Token variant requires `token::owner_seeds = [...]`",
                    ));
                }
            }
            ManualVariantKind::Ata => {}
        }

        Ok(VariantAttrContent {
            kind,
            is_zero_copy,
            seeds,
            owner_seeds,
        })
    }
}

/// Parse a seed array `[seed1, seed2, ...]`.
fn parse_seed_array(input: syn::parse::ParseStream) -> Result<Vec<ManualSeed>> {
    let content;
    syn::bracketed!(content in input);

    let mut seeds = Vec::new();
    while !content.is_empty() {
        seeds.push(parse_single_seed(&content)?);
        if content.peek(syn::Token![,]) {
            let _: syn::Token![,] = content.parse()?;
        } else {
            break;
        }
    }
    Ok(seeds)
}

/// Parse a single seed expression with explicit prefix disambiguation.
fn parse_single_seed(input: syn::parse::ParseStream) -> Result<ManualSeed> {
    // Check for byte string literal: b"..."
    if input.peek(syn::LitByteStr) {
        let lit: syn::LitByteStr = input.parse()?;
        return Ok(ManualSeed::ByteLiteral(lit));
    }

    // Check for string literal: "..."
    if input.peek(syn::LitStr) {
        let lit: syn::LitStr = input.parse()?;
        return Ok(ManualSeed::StrLiteral(lit));
    }

    // Parse as path/expression
    // Could be: ctx.field, data.field, CONSTANT, path::CONSTANT
    let expr: syn::Expr = input.parse()?;
    classify_seed_expr(&expr)
}

/// Classify a parsed expression into a ManualSeed.
fn classify_seed_expr(expr: &syn::Expr) -> Result<ManualSeed> {
    match expr {
        // ctx.field or data.field
        syn::Expr::Field(field_expr) => {
            if let syn::Expr::Path(base_path) = field_expr.base.as_ref() {
                if let Some(base_ident) = base_path.path.get_ident() {
                    let base_str = base_ident.to_string();
                    if let syn::Member::Named(field_name) = &field_expr.member {
                        if base_str == "ctx" {
                            return Ok(ManualSeed::CtxField(field_name.clone()));
                        } else if base_str == "data" {
                            return Ok(ManualSeed::DataField(field_name.clone()));
                        }
                    }
                }
            }
            Err(syn::Error::new_spanned(
                expr,
                "Field access seeds must use ctx.field or data.field prefix",
            ))
        }
        // CONSTANT or path::CONSTANT
        syn::Expr::Path(path_expr) => {
            let path = &path_expr.path;
            // Check if last segment is a constant (SCREAMING_SNAKE_CASE)
            if let Some(last_seg) = path.segments.last() {
                if is_constant_identifier(&last_seg.ident.to_string()) {
                    return Ok(ManualSeed::Constant(path.clone()));
                }
            }
            // Could be a single lowercase ident like `ctx` or `data` without field access
            Err(syn::Error::new_spanned(
                expr,
                "Seed path must be a SCREAMING_SNAKE_CASE constant, or use ctx.field / data.field prefix",
            ))
        }
        _ => Err(syn::Error::new_spanned(
            expr,
            "Unsupported seed expression. Use: b\"literal\", \"literal\", ctx.field, data.field, or CONSTANT",
        )),
    }
}

// =============================================================================
// CONVERSION: ManualSeed -> ClassifiedSeed
// =============================================================================

fn manual_seed_to_classified(seed: &ManualSeed) -> ClassifiedSeed {
    match seed {
        ManualSeed::ByteLiteral(lit) => ClassifiedSeed::Literal(lit.value()),
        ManualSeed::StrLiteral(lit) => ClassifiedSeed::Literal(lit.value().into_bytes()),
        ManualSeed::Constant(path) => {
            let expr: syn::Expr = syn::parse_quote!(#path);
            ClassifiedSeed::Constant {
                path: path.clone(),
                expr: Box::new(expr),
            }
        }
        ManualSeed::CtxField(ident) => ClassifiedSeed::CtxRooted {
            account: ident.clone(),
        },
        ManualSeed::DataField(ident) => {
            let expr: syn::Expr = syn::parse_quote!(data.#ident);
            ClassifiedSeed::DataRooted {
                root: ident.clone(),
                expr: Box::new(expr),
            }
        }
    }
}

// =============================================================================
// CONVERSION: ManualSeed -> SeedElement
// =============================================================================

fn manual_seed_to_seed_element(seed: &ManualSeed) -> SeedElement {
    match seed {
        ManualSeed::ByteLiteral(lit) => {
            let expr: syn::Expr = syn::parse_quote!(#lit);
            SeedElement::Expression(Box::new(expr))
        }
        ManualSeed::StrLiteral(lit) => SeedElement::Literal(lit.clone()),
        ManualSeed::Constant(path) => {
            let expr: syn::Expr = syn::parse_quote!(#path);
            SeedElement::Expression(Box::new(expr))
        }
        ManualSeed::CtxField(ident) => {
            let expr: syn::Expr = syn::parse_quote!(ctx.#ident);
            SeedElement::Expression(Box::new(expr))
        }
        ManualSeed::DataField(ident) => {
            let expr: syn::Expr = syn::parse_quote!(data.#ident);
            SeedElement::Expression(Box::new(expr))
        }
    }
}

fn manual_seeds_to_punctuated(
    seeds: &[ManualSeed],
) -> syn::punctuated::Punctuated<SeedElement, syn::Token![,]> {
    let mut result = syn::punctuated::Punctuated::new();
    for seed in seeds {
        result.push(manual_seed_to_seed_element(seed));
    }
    result
}

fn manual_seeds_to_seed_elements_vec(seeds: &[ManualSeed]) -> Vec<SeedElement> {
    seeds.iter().map(manual_seed_to_seed_element).collect()
}

// =============================================================================
// BUILDER: Convert parsed variants to intermediate types
// =============================================================================

#[allow(clippy::type_complexity)]
fn build_intermediate_types(
    variants: &[ParsedManualVariant],
    _crate_ctx: &CrateContext,
) -> Result<(
    Vec<CompressibleAccountInfo>,
    Option<Vec<TokenSeedSpec>>,
    Option<Vec<TokenSeedSpec>>,
    Vec<InstructionDataSpec>,
    bool,
    bool,
    TokenStream,
)> {
    let mut compressible_accounts = Vec::new();
    let mut pda_seed_specs = Vec::new();
    let mut token_seed_specs = Vec::new();
    let mut instruction_data_specs = Vec::new();
    let has_mint_fields = false;
    let mut has_ata_fields = false;
    let mut pda_variant_code = TokenStream::new();

    // Track data field names we've already added to instruction_data
    let mut seen_data_fields = std::collections::HashSet::new();

    for variant in variants {
        match &variant.kind {
            ManualVariantKind::Pda => {
                let inner_type = variant.inner_type.as_ref().unwrap();

                // Build CompressibleAccountInfo
                compressible_accounts.push(CompressibleAccountInfo {
                    account_type: inner_type.clone(),
                    is_zero_copy: variant.is_zero_copy,
                });

                // Build ClassifiedSeeds for VariantBuilder
                let classified_seeds: Vec<ClassifiedSeed> = variant
                    .seeds
                    .iter()
                    .map(manual_seed_to_classified)
                    .collect();

                // Build ExtractedSeedSpec for VariantBuilder
                let extracted_spec = ExtractedSeedSpec {
                    variant_name: variant.ident.clone(),
                    inner_type: inner_type.clone(),
                    seeds: classified_seeds,
                    is_zero_copy: variant.is_zero_copy,
                    struct_name: variant.ident.to_string(),
                    module_path: "crate".to_string(),
                };

                // Generate variant code
                let builder = VariantBuilder::from_extracted_spec(&extracted_spec);
                pda_variant_code.extend(builder.build());

                // Build TokenSeedSpec for PDA seeds
                let seed_elements = manual_seeds_to_punctuated(&variant.seeds);
                let dummy_eq: syn::Token![=] = syn::parse_quote!(=);
                pda_seed_specs.push(TokenSeedSpec {
                    variant: variant.ident.clone(),
                    _eq: dummy_eq,
                    is_token: Some(false),
                    seeds: seed_elements,
                    owner_seeds: None,
                    inner_type: Some(inner_type.clone()),
                    is_zero_copy: variant.is_zero_copy,
                });

                // Extract data fields for InstructionDataSpec
                for seed in &variant.seeds {
                    if let ManualSeed::DataField(ident) = seed {
                        let name = ident.to_string();
                        if seen_data_fields.insert(name) {
                            // Default to Pubkey type for data seeds without conversion
                            instruction_data_specs.push(InstructionDataSpec {
                                field_name: ident.clone(),
                                field_type: syn::parse_quote!(Pubkey),
                            });
                        }
                    }
                }
            }

            ManualVariantKind::Token => {
                let seed_elements = manual_seeds_to_punctuated(&variant.seeds);
                let owner_seeds_elements = variant
                    .owner_seeds
                    .as_ref()
                    .map(|os| manual_seeds_to_seed_elements_vec(os));

                let dummy_eq: syn::Token![=] = syn::parse_quote!(=);
                token_seed_specs.push(TokenSeedSpec {
                    variant: variant.ident.clone(),
                    _eq: dummy_eq,
                    is_token: Some(true),
                    seeds: seed_elements,
                    owner_seeds: owner_seeds_elements,
                    inner_type: None,
                    is_zero_copy: false,
                });
            }

            ManualVariantKind::Ata => {
                has_ata_fields = true;
            }
        }
    }

    let pda_seeds = if pda_seed_specs.is_empty() {
        None
    } else {
        Some(pda_seed_specs)
    };

    let token_seeds = if token_seed_specs.is_empty() {
        None
    } else {
        Some(token_seed_specs)
    };

    Ok((
        compressible_accounts,
        pda_seeds,
        token_seeds,
        instruction_data_specs,
        has_mint_fields,
        has_ata_fields,
        pda_variant_code,
    ))
}

// =============================================================================
// ENTRY POINT
// =============================================================================

/// Main entry point for `#[derive(LightProgram)]`.
pub fn derive_light_program_impl(input: DeriveInput) -> Result<TokenStream> {
    // 1. Parse the enum variants
    let variants = parse_enum_variants(&input)?;

    if variants.is_empty() {
        return Err(syn::Error::new_spanned(
            &input,
            "#[derive(LightProgram)] enum must have at least one variant",
        ));
    }

    // 2. Parse crate context for struct field lookup
    let crate_ctx = CrateContext::parse_from_manifest()?;

    // 3. Build intermediate types
    let (
        compressible_accounts,
        pda_seeds,
        token_seeds,
        instruction_data,
        has_mint_fields,
        has_ata_fields,
        pda_variant_code,
    ) = build_intermediate_types(&variants, &crate_ctx)?;

    // 4. Generate all items using the shared function
    let enum_name = &input.ident;
    let items = generate_light_program_items(
        compressible_accounts,
        pda_seeds,
        token_seeds,
        instruction_data,
        &crate_ctx,
        has_mint_fields,
        has_ata_fields,
        pda_variant_code,
        Some(enum_name),
    )?;

    // 5. Combine into single TokenStream
    // The derive output appears at the call site, so add the anchor import
    let anchor_import = quote! {
        use anchor_lang::prelude::*;
    };

    let mut output = TokenStream::new();
    output.extend(anchor_import);
    for item in items {
        output.extend(item);
    }

    Ok(output)
}

/// Main entry point for `#[derive(LightProgramPinocchio)]`.
///
/// Same logic as `derive_light_program_impl()` but generates pinocchio-compatible code:
/// - `BorshSerialize/BorshDeserialize` instead of `AnchorSerialize/AnchorDeserialize`
/// - `light_account_pinocchio::` instead of `light_account::`
/// - No `use anchor_lang::prelude::*;` import
/// - Config/compress/decompress as enum associated functions
pub fn derive_light_program_pinocchio_impl(input: DeriveInput) -> Result<TokenStream> {
    // 1. Parse the enum variants (reused)
    let variants = parse_enum_variants(&input)?;

    if variants.is_empty() {
        return Err(syn::Error::new_spanned(
            &input,
            "#[derive(LightProgramPinocchio)] enum must have at least one variant",
        ));
    }

    // 2. Parse crate context for struct field lookup
    let crate_ctx = CrateContext::parse_from_manifest()?;

    // 3. Build intermediate types (reused)
    let (
        compressible_accounts,
        pda_seeds,
        token_seeds,
        instruction_data,
        has_mint_fields,
        has_ata_fields,
        _pda_variant_code, // We'll regenerate with pinocchio derives
    ) = build_intermediate_types(&variants, &crate_ctx)?;

    // 3b. Re-generate PDA variant code with pinocchio derives
    let pda_variant_code_pinocchio: TokenStream = variants
        .iter()
        .filter(|v| matches!(v.kind, ManualVariantKind::Pda))
        .map(|variant| {
            let spec = manual_variant_to_extracted_spec(variant, &crate_ctx);
            VariantBuilder::from_extracted_spec(&spec).build_for_pinocchio()
        })
        .collect();

    // 4. Generate all items using the pinocchio orchestration function
    let enum_name = &input.ident;
    let items = super::instructions::generate_light_program_pinocchio_items(
        compressible_accounts,
        pda_seeds,
        token_seeds,
        instruction_data,
        &crate_ctx,
        has_mint_fields,
        has_ata_fields,
        pda_variant_code_pinocchio,
        Some(enum_name),
    )?;

    // 5. Combine into single TokenStream (NO anchor import)
    let mut output = TokenStream::new();
    for item in items {
        output.extend(item);
    }

    Ok(output)
}

/// Convert a ParsedManualVariant to ExtractedSeedSpec for VariantBuilder.
fn manual_variant_to_extracted_spec(
    variant: &ParsedManualVariant,
    _crate_ctx: &CrateContext,
) -> ExtractedSeedSpec {
    let seeds: Vec<ClassifiedSeed> = variant
        .seeds
        .iter()
        .map(manual_seed_to_classified)
        .collect();

    ExtractedSeedSpec {
        struct_name: variant.ident.to_string(),
        variant_name: variant.ident.clone(),
        inner_type: variant
            .inner_type
            .clone()
            .unwrap_or_else(|| syn::parse_quote!(())),
        seeds,
        is_zero_copy: variant.is_zero_copy,
        module_path: String::new(),
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use quote::format_ident;

    use super::*;

    fn parse_derive_input(input: &str) -> DeriveInput {
        syn::parse_str(input).expect("Failed to parse derive input")
    }

    // =========================================================================
    // PARSING TESTS: new #[light_account(...)] namespace syntax
    // =========================================================================

    #[test]
    fn test_parse_pda_variant() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"record", ctx.owner])]
                Record(MinimalRecord),
            }
            "#,
        );

        let variants = parse_enum_variants(&input).expect("should parse");
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].ident.to_string(), "Record");
        assert!(matches!(variants[0].kind, ManualVariantKind::Pda));
        assert!(!variants[0].is_zero_copy);
        assert_eq!(variants[0].seeds.len(), 2);
        assert!(variants[0].inner_type.is_some());
    }

    #[test]
    fn test_parse_zero_copy_variant() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"zc_record", ctx.owner], pda::zero_copy)]
                ZcRecord(ZeroCopyRecord),
            }
            "#,
        );

        let variants = parse_enum_variants(&input).expect("should parse");
        assert_eq!(variants.len(), 1);
        assert!(variants[0].is_zero_copy);
    }

    #[test]
    fn test_parse_token_variant() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(token::seeds = [VAULT_SEED, ctx.mint], token::owner_seeds = [AUTH_SEED])]
                Vault,
            }
            "#,
        );

        let variants = parse_enum_variants(&input).expect("should parse");
        assert_eq!(variants.len(), 1);
        assert!(matches!(variants[0].kind, ManualVariantKind::Token));
        assert!(variants[0].inner_type.is_none());
        assert_eq!(variants[0].seeds.len(), 2);
        assert!(variants[0].owner_seeds.is_some());
    }

    #[test]
    fn test_parse_ata_variant() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(associated_token)]
                Ata,
            }
            "#,
        );

        let variants = parse_enum_variants(&input).expect("should parse");
        assert_eq!(variants.len(), 1);
        assert!(matches!(variants[0].kind, ManualVariantKind::Ata));
    }

    #[test]
    fn test_parse_mixed_enum() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"record", ctx.owner])]
                Record(MinimalRecord),

                #[light_account(token::seeds = [VAULT_SEED, ctx.mint], token::owner_seeds = [AUTH_SEED])]
                Vault,

                #[light_account(associated_token)]
                Ata,
            }
            "#,
        );

        let variants = parse_enum_variants(&input).expect("should parse");
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn test_error_missing_inner_type_for_pda() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"record", ctx.owner])]
                Record,
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("tuple variant"), "Error: {}", err_msg);
    }

    #[test]
    fn test_error_missing_seeds_for_pda() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::zero_copy)]
                Record(MinimalRecord),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("pda::seeds"), "Error: {}", err_msg);
    }

    #[test]
    fn test_error_missing_attribute() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                Record(MinimalRecord),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_seed_classification() {
        // Test byte literal
        let byte_lit: syn::LitByteStr = syn::parse_quote!(b"seed");
        let seed = ManualSeed::ByteLiteral(byte_lit);
        let classified = manual_seed_to_classified(&seed);
        assert!(matches!(classified, ClassifiedSeed::Literal(_)));

        // Test string literal
        let str_lit: syn::LitStr = syn::parse_quote!("seed");
        let seed = ManualSeed::StrLiteral(str_lit);
        let classified = manual_seed_to_classified(&seed);
        assert!(matches!(classified, ClassifiedSeed::Literal(_)));

        // Test constant
        let path: syn::Path = syn::parse_quote!(MY_SEED);
        let seed = ManualSeed::Constant(path);
        let classified = manual_seed_to_classified(&seed);
        assert!(matches!(classified, ClassifiedSeed::Constant { .. }));

        // Test ctx field
        let ident = format_ident!("owner");
        let seed = ManualSeed::CtxField(ident);
        let classified = manual_seed_to_classified(&seed);
        assert!(matches!(classified, ClassifiedSeed::CtxRooted { .. }));

        // Test data field
        let ident = format_ident!("owner");
        let seed = ManualSeed::DataField(ident);
        let classified = manual_seed_to_classified(&seed);
        assert!(matches!(classified, ClassifiedSeed::DataRooted { .. }));
    }

    #[test]
    fn test_string_literal_seeds() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = ["record_seed", ctx.owner])]
                Record(MinimalRecord),
            }
            "#,
        );

        let variants = parse_enum_variants(&input).expect("should parse");
        assert_eq!(variants[0].seeds.len(), 2);
        assert!(matches!(variants[0].seeds[0], ManualSeed::StrLiteral(_)));
    }

    #[test]
    fn test_data_field_seeds() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"record", data.some_key])]
                Record(MinimalRecord),
            }
            "#,
        );

        let variants = parse_enum_variants(&input).expect("should parse");
        assert_eq!(variants[0].seeds.len(), 2);
        assert!(matches!(variants[0].seeds[1], ManualSeed::DataField(_)));
    }

    #[test]
    fn test_constant_path_seeds() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [RECORD_SEED, ctx.owner])]
                Record(MinimalRecord),
            }
            "#,
        );

        let variants = parse_enum_variants(&input).expect("should parse");
        assert!(matches!(variants[0].seeds[0], ManualSeed::Constant(_)));
    }

    // =========================================================================
    // BUILDER TESTS: verify build_intermediate_types for each configuration
    // =========================================================================

    #[allow(clippy::type_complexity)]
    fn parse_and_build(
        input_str: &str,
    ) -> (
        Vec<CompressibleAccountInfo>,
        Option<Vec<TokenSeedSpec>>,
        Option<Vec<TokenSeedSpec>>,
        Vec<InstructionDataSpec>,
        bool,
        bool,
        TokenStream,
    ) {
        let input = parse_derive_input(input_str);
        let variants = parse_enum_variants(&input).expect("should parse");
        let crate_ctx = CrateContext::empty();
        build_intermediate_types(&variants, &crate_ctx).expect("should build")
    }

    /// 1 PDA: verify compressible_accounts, pda_seeds, no token/mint/ata
    #[test]
    fn test_build_single_pda() {
        let (accounts, pda_seeds, token_seeds, _instr_data, has_mint, has_ata, _variant_code) =
            parse_and_build(
                r#"
                #[derive(LightProgram)]
                pub enum ProgramAccounts {
                    #[light_account(pda::seeds = [b"minimal_record", ctx.owner])]
                    MinimalRecord(MinimalRecord),
                }
                "#,
            );

        assert_eq!(accounts.len(), 1, "should have 1 compressible account");
        assert!(!accounts[0].is_zero_copy, "should not be zero_copy");
        assert!(pda_seeds.is_some(), "should have pda_seeds");
        assert_eq!(pda_seeds.as_ref().unwrap().len(), 1);
        assert_eq!(
            pda_seeds.as_ref().unwrap()[0].variant.to_string(),
            "MinimalRecord"
        );
        assert!(token_seeds.is_none(), "should have no token_seeds");
        assert!(!has_mint, "should not have mint");
        assert!(!has_ata, "should not have ata");
    }

    /// 1 ATA: verify has_ata_fields=true, nothing else
    #[test]
    fn test_build_single_ata() {
        let (accounts, pda_seeds, token_seeds, _instr_data, has_mint, has_ata, _variant_code) =
            parse_and_build(
                r#"
                #[derive(LightProgram)]
                pub enum ProgramAccounts {
                    #[light_account(associated_token)]
                    Ata,
                }
                "#,
            );

        assert!(accounts.is_empty(), "should have no compressible accounts");
        assert!(pda_seeds.is_none(), "should have no pda_seeds");
        assert!(token_seeds.is_none(), "should have no token_seeds");
        assert!(!has_mint, "should not have mint");
        assert!(has_ata, "should have ata");
    }

    /// 1 token PDA: verify token_seeds, no pda_seeds/mint/ata
    #[test]
    fn test_build_single_token_pda() {
        let (accounts, pda_seeds, token_seeds, _instr_data, has_mint, has_ata, _variant_code) =
            parse_and_build(
                r#"
                #[derive(LightProgram)]
                pub enum ProgramAccounts {
                    #[light_account(token::seeds = [VAULT_SEED, ctx.mint], token::owner_seeds = [VAULT_AUTH_SEED])]
                    Vault,
                }
                "#,
            );

        assert!(accounts.is_empty(), "should have no compressible accounts");
        assert!(pda_seeds.is_none(), "should have no pda_seeds");
        assert!(token_seeds.is_some(), "should have token_seeds");
        assert_eq!(token_seeds.as_ref().unwrap().len(), 1);
        let ts = &token_seeds.as_ref().unwrap()[0];
        assert_eq!(ts.variant.to_string(), "Vault");
        assert_eq!(ts.is_token, Some(true));
        assert!(ts.owner_seeds.is_some(), "should have owner_seeds");
        assert!(!has_mint, "should not have mint");
        assert!(!has_ata, "should not have ata");
    }

    /// 1 account loader (zero_copy PDA): verify is_zero_copy flag
    #[test]
    fn test_build_single_account_loader() {
        let (accounts, pda_seeds, token_seeds, _instr_data, has_mint, has_ata, _variant_code) =
            parse_and_build(
                r#"
                #[derive(LightProgram)]
                pub enum ProgramAccounts {
                    #[light_account(pda::seeds = [RECORD_SEED, ctx.owner], pda::zero_copy)]
                    ZeroCopyRecord(ZeroCopyRecord),
                }
                "#,
            );

        assert_eq!(accounts.len(), 1, "should have 1 compressible account");
        assert!(accounts[0].is_zero_copy, "should be zero_copy");
        assert!(pda_seeds.is_some(), "should have pda_seeds");
        assert_eq!(pda_seeds.as_ref().unwrap().len(), 1);
        assert!(
            pda_seeds.as_ref().unwrap()[0].is_zero_copy,
            "seed spec should be zero_copy"
        );
        assert!(token_seeds.is_none(), "should have no token_seeds");
        assert!(!has_mint, "should not have mint");
        assert!(!has_ata, "should not have ata");
    }

    /// Combined: 1 pda + 1 ata + 1 token pda + 1 account loader
    #[test]
    fn test_parse_full_combined() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"minimal_record", ctx.owner])]
                MinimalRecord(MinimalRecord),

                #[light_account(associated_token)]
                Ata,

                #[light_account(token::seeds = [VAULT_SEED, ctx.mint], token::owner_seeds = [VAULT_AUTH_SEED])]
                Vault,

                #[light_account(pda::seeds = [RECORD_SEED, ctx.owner], pda::zero_copy)]
                ZeroCopyRecord(ZeroCopyRecord),
            }
            "#,
        );

        let variants = parse_enum_variants(&input).expect("should parse");
        assert_eq!(variants.len(), 4);

        assert!(matches!(variants[0].kind, ManualVariantKind::Pda));
        assert!(!variants[0].is_zero_copy);

        assert!(matches!(variants[1].kind, ManualVariantKind::Ata));

        assert!(matches!(variants[2].kind, ManualVariantKind::Token));
        assert!(variants[2].owner_seeds.is_some());

        assert!(matches!(variants[3].kind, ManualVariantKind::Pda));
        assert!(variants[3].is_zero_copy);
    }

    #[test]
    fn test_build_full_combined() {
        let (accounts, pda_seeds, token_seeds, _instr_data, has_mint, has_ata, _variant_code) =
            parse_and_build(
                r#"
                #[derive(LightProgram)]
                pub enum ProgramAccounts {
                    #[light_account(pda::seeds = [b"minimal_record", ctx.owner])]
                    MinimalRecord(MinimalRecord),

                    #[light_account(associated_token)]
                    Ata,

                    #[light_account(token::seeds = [VAULT_SEED, ctx.mint], token::owner_seeds = [VAULT_AUTH_SEED])]
                    Vault,

                    #[light_account(pda::seeds = [RECORD_SEED, ctx.owner], pda::zero_copy)]
                    ZeroCopyRecord(ZeroCopyRecord),
                }
                "#,
            );

        // 2 PDA variants (one regular, one zero_copy)
        assert_eq!(accounts.len(), 2, "should have 2 compressible accounts");
        assert!(!accounts[0].is_zero_copy, "first account is regular PDA");
        assert!(accounts[1].is_zero_copy, "second account is zero_copy");

        // PDA seeds
        assert!(pda_seeds.is_some(), "should have pda_seeds");
        let pda = pda_seeds.as_ref().unwrap();
        assert_eq!(pda.len(), 2, "should have 2 pda seed specs");
        assert_eq!(pda[0].variant.to_string(), "MinimalRecord");
        assert!(!pda[0].is_zero_copy);
        assert_eq!(pda[1].variant.to_string(), "ZeroCopyRecord");
        assert!(pda[1].is_zero_copy);

        // Token seeds
        assert!(token_seeds.is_some(), "should have token_seeds");
        let tok = token_seeds.as_ref().unwrap();
        assert_eq!(tok.len(), 1, "should have 1 token seed spec");
        assert_eq!(tok[0].variant.to_string(), "Vault");
        assert_eq!(tok[0].is_token, Some(true));
        assert!(tok[0].owner_seeds.is_some());

        // Flags
        assert!(!has_mint, "should not have mint");
        assert!(has_ata, "should have ata");
    }

    // =========================================================================
    // SEED ELEMENT CONVERSION TESTS
    // =========================================================================

    #[test]
    fn test_seed_element_conversions() {
        // ByteLiteral -> SeedElement::Expression
        let byte_seed = ManualSeed::ByteLiteral(syn::parse_quote!(b"test"));
        let elem = manual_seed_to_seed_element(&byte_seed);
        assert!(
            matches!(elem, SeedElement::Expression(_)),
            "byte literal -> Expression"
        );

        // StrLiteral -> SeedElement::Literal
        let str_seed = ManualSeed::StrLiteral(syn::parse_quote!("test"));
        let elem = manual_seed_to_seed_element(&str_seed);
        assert!(
            matches!(elem, SeedElement::Literal(_)),
            "str literal -> Literal"
        );

        // Constant -> SeedElement::Expression
        let const_seed = ManualSeed::Constant(syn::parse_quote!(MY_CONSTANT));
        let elem = manual_seed_to_seed_element(&const_seed);
        assert!(
            matches!(elem, SeedElement::Expression(_)),
            "constant -> Expression"
        );

        // CtxField -> SeedElement::Expression
        let ctx_seed = ManualSeed::CtxField(format_ident!("owner"));
        let elem = manual_seed_to_seed_element(&ctx_seed);
        assert!(
            matches!(elem, SeedElement::Expression(_)),
            "ctx field -> Expression"
        );

        // DataField -> SeedElement::Expression
        let data_seed = ManualSeed::DataField(format_ident!("key"));
        let elem = manual_seed_to_seed_element(&data_seed);
        assert!(
            matches!(elem, SeedElement::Expression(_)),
            "data field -> Expression"
        );
    }

    #[test]
    fn test_manual_seeds_to_punctuated() {
        let seeds = vec![
            ManualSeed::ByteLiteral(syn::parse_quote!(b"prefix")),
            ManualSeed::CtxField(format_ident!("owner")),
            ManualSeed::Constant(syn::parse_quote!(EXTRA_SEED)),
        ];

        let punctuated = manual_seeds_to_punctuated(&seeds);
        assert_eq!(punctuated.len(), 3);
    }

    // =========================================================================
    // ERROR CASE TESTS
    // =========================================================================

    /// Token variant without seeds should fail
    #[test]
    fn test_error_token_missing_seeds() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(token::owner_seeds = [AUTH_SEED])]
                Vault,
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("token::seeds"), "Error: {}", err_msg);
    }

    /// Token variant without owner_seeds should fail
    #[test]
    fn test_error_token_missing_owner_seeds() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(token::seeds = [VAULT_SEED])]
                Vault,
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("token::owner_seeds"), "Error: {}", err_msg);
    }

    /// PDA variant with unit type (no tuple field) should fail
    #[test]
    fn test_error_pda_unit_variant() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"test"])]
                Record,
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
    }

    /// Token variant with fields should fail
    #[test]
    fn test_error_token_with_fields() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(token::seeds = [SEED], token::owner_seeds = [AUTH])]
                Vault(SomeType),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("unit variant"), "Error: {}", err_msg);
    }

    /// ATA variant with fields should fail
    #[test]
    fn test_error_ata_with_fields() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(associated_token)]
                Ata(SomeType),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
    }

    /// Standalone mint keyword should fail (mints handled by Light Token Program)
    #[test]
    fn test_error_mint_keyword_rejected() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(mint)]
                MintAccount,
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Unknown keyword") || err_msg.contains("Light Token Program"),
            "Error: {}",
            err_msg
        );
    }

    /// Unknown keyword should fail
    #[test]
    fn test_error_unknown_keyword() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(unknown)]
                Something,
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Unknown keyword"), "Error: {}", err_msg);
    }

    /// Derive on struct (not enum) should fail
    #[test]
    fn test_error_struct_not_enum() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub struct NotAnEnum {
                pub field: u64,
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("can only be applied to enums"),
            "Error: {}",
            err_msg
        );
    }

    /// Empty enum should parse but derive_light_program_impl should fail
    #[test]
    fn test_error_empty_enum() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {}
            "#,
        );

        let variants = parse_enum_variants(&input).expect("empty enum parses");
        assert!(variants.is_empty());
    }

    // =========================================================================
    // NAMESPACE VALIDATION TESTS
    // =========================================================================

    /// Mixed namespaces on same variant should fail
    #[test]
    fn test_error_mixed_namespaces() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"test"], token::seeds = [SEED])]
                Mixed(SomeType),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Mixed namespaces"), "Error: {}", err_msg);
    }

    /// Unknown namespace should fail
    #[test]
    fn test_error_unknown_namespace() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(foo::seeds = [b"test"])]
                Something(SomeType),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Unknown namespace") || err_msg.contains("foo"),
            "Error: {}",
            err_msg
        );
    }

    /// Unknown key within valid namespace should fail
    #[test]
    fn test_error_unknown_key_in_namespace() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::invalid = [b"test"])]
                Something(SomeType),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("invalid") && err_msg.contains("pda"),
            "Error: {}",
            err_msg
        );
    }

    /// Duplicate keys should fail
    #[test]
    fn test_error_duplicate_key() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"a"], pda::seeds = [b"b"])]
                Something(SomeType),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Duplicate"), "Error: {}", err_msg);
    }

    /// Bare seeds keyword (without namespace) should fail
    #[test]
    fn test_error_bare_seeds_keyword() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(seeds = [b"test"])]
                Something(SomeType),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Unknown keyword") || err_msg.contains("seeds"),
            "Error: {}",
            err_msg
        );
    }

    /// Bare pda keyword (old syntax) should fail
    #[test]
    fn test_error_bare_pda_keyword() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda, seeds = [b"test"])]
                Something(SomeType),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("pda")
                && (err_msg.contains("Unknown keyword") || err_msg.contains("namespaced")),
            "Error: {}",
            err_msg
        );
    }

    /// associated_token standalone keyword works
    #[test]
    fn test_associated_token_keyword() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(associated_token)]
                Ata,
            }
            "#,
        );

        let variants = parse_enum_variants(&input).expect("should parse");
        assert_eq!(variants.len(), 1);
        assert!(matches!(variants[0].kind, ManualVariantKind::Ata));
    }

    /// Bare keyword in middle position should fail
    #[test]
    fn test_error_bare_keyword_in_middle() {
        let input = parse_derive_input(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"test"], zero_copy)]
                Something(SomeType),
            }
            "#,
        );

        let result = parse_enum_variants(&input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Unexpected keyword") || err_msg.contains("namespaced"),
            "Error: {}",
            err_msg
        );
    }

    // =========================================================================
    // DATA FIELD EXTRACTION TESTS
    // =========================================================================

    /// PDA with data.field seeds should generate InstructionDataSpecs
    #[test]
    fn test_build_data_field_extraction() {
        let (_, _, _, instr_data, _, _, _) = parse_and_build(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"record", data.some_key, data.another_key])]
                Record(MinimalRecord),
            }
            "#,
        );

        assert_eq!(instr_data.len(), 2, "should extract 2 data fields");
        let names: Vec<String> = instr_data
            .iter()
            .map(|s| s.field_name.to_string())
            .collect();
        assert!(names.contains(&"some_key".to_string()));
        assert!(names.contains(&"another_key".to_string()));
    }

    /// Duplicate data fields across variants should be deduplicated
    #[test]
    fn test_build_dedup_data_fields() {
        let (_, _, _, instr_data, _, _, _) = parse_and_build(
            r#"
            #[derive(LightProgram)]
            pub enum ProgramAccounts {
                #[light_account(pda::seeds = [b"record_a", data.owner])]
                RecordA(RecordA),

                #[light_account(pda::seeds = [b"record_b", data.owner])]
                RecordB(RecordB),
            }
            "#,
        );

        assert_eq!(
            instr_data.len(),
            1,
            "duplicate data.owner should be deduplicated"
        );
        assert_eq!(instr_data[0].field_name.to_string(), "owner");
    }
}
