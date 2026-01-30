//! Seed extraction from Anchor account attributes.
//!
//! This module handles parsing `#[account(seeds = [...], bump)]` attributes
//! and extracting field information from Accounts structs.

use syn::{Ident, ItemStruct, Type};

use super::{
    anchor_extraction::extract_anchor_seeds,
    classification::classify_seed_expr,
    instruction_args::InstructionArgSet,
    types::{ClassifiedSeed, ExtractedAccountsInfo, ExtractedSeedSpec, ExtractedTokenSpec},
};
use crate::{
    light_pdas::{
        account::validation::{type_name, AccountTypeError},
        light_account_keywords::{
            is_standalone_keyword, unknown_key_error, valid_keys_for_namespace,
        },
    },
    utils::snake_to_camel_case,
};

// =============================================================================
// ACCOUNT TYPE EXTRACTION
// =============================================================================

/// Extract inner type from `Account<'info, T>`, `Box<Account<'info, T>>`,
/// `AccountLoader<'info, T>`, or `InterfaceAccount<'info, T>`.
///
/// Returns `(is_boxed, inner_type)` preserving the full type path.
///
/// # Errors
/// - `AccountTypeError::WrongType` if the type is not a recognized account wrapper
/// - `AccountTypeError::NestedBox` if nested Box<Box<...>> is detected
/// - `AccountTypeError::ExtractionFailed` if generic arguments couldn't be extracted
pub fn extract_account_inner_type(ty: &Type) -> Result<(bool, Type), AccountTypeError> {
    match ty {
        Type::Path(type_path) => {
            let segment = type_path
                .path
                .segments
                .last()
                .ok_or_else(|| AccountTypeError::WrongType { got: type_name(ty) })?;
            let ident_str = segment.ident.to_string();

            match ident_str.as_str() {
                "Account" | "AccountLoader" | "InterfaceAccount" => {
                    // Extract T from Account<'info, T>
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                // Skip lifetime 'info by checking if this is a path type
                                if let Type::Path(inner_path) = inner_ty {
                                    if let Some(inner_seg) = inner_path.path.segments.last() {
                                        // Skip lifetime 'info
                                        if inner_seg.ident != "info" {
                                            // Return the full type, preserving the path
                                            return Ok((false, inner_ty.clone()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(AccountTypeError::ExtractionFailed)
                }
                "Box" => {
                    // Check for Box<Account<'info, T>>
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            // Check for nested Box<Box<...>> which is not supported
                            if let Type::Path(inner_path) = inner_ty {
                                if let Some(inner_seg) = inner_path.path.segments.last() {
                                    if inner_seg.ident == "Box" {
                                        // Nested Box detected - explicit error
                                        return Err(AccountTypeError::NestedBox);
                                    }
                                }
                            }

                            // Recurse to extract from Box<Account<...>>
                            return match extract_account_inner_type(inner_ty) {
                                Ok((_, inner_type)) => Ok((true, inner_type)),
                                Err(e) => Err(e),
                            };
                        }
                    }
                    Err(AccountTypeError::ExtractionFailed)
                }
                _ => Err(AccountTypeError::WrongType { got: type_name(ty) }),
            }
        }
        _ => Err(AccountTypeError::WrongType { got: type_name(ty) }),
    }
}

/// Check #[light_account(...)] attributes for PDA, mint, token, or ATA type.
/// Returns (has_pda, has_mint, has_ata, has_zero_copy) indicating which type was detected.
///
/// Types:
/// - PDA: `#[light_account(init)]` only (no namespace prefix)
/// - Mint: `#[light_account(init, mint::...)]`
/// - Token: `#[light_account(init, token::...)]` or `#[light_account(token::...)]`
/// - ATA: `#[light_account(init, associated_token::...)]` or `#[light_account(associated_token::...)]`
/// - Zero-copy: `#[light_account(init, zero_copy)]` - only valid with PDA
fn check_light_account_type(attrs: &[syn::Attribute]) -> (bool, bool, bool, bool) {
    for attr in attrs {
        if attr.path().is_ident("light_account") {
            // Parse the content to determine if it's init-only (PDA) or init+mint (Mint)
            let tokens = match &attr.meta {
                syn::Meta::List(list) => list.tokens.clone(),
                _ => continue,
            };

            let token_vec: Vec<_> = tokens.clone().into_iter().collect();

            // Helper to check for a namespace prefix (e.g., "mint", "token", "associated_token")
            let has_namespace_prefix = |namespace: &str| {
                token_vec.windows(2).any(|window| {
                    matches!(
                        (&window[0], &window[1]),
                        (
                            proc_macro2::TokenTree::Ident(ident),
                            proc_macro2::TokenTree::Punct(punct)
                        ) if ident == namespace && punct.as_char() == ':'
                    )
                })
            };

            let has_mint_namespace = has_namespace_prefix("mint");
            let has_token_namespace = has_namespace_prefix("token");
            let has_ata_namespace = has_namespace_prefix("associated_token");

            // Check for init keyword
            let has_init = token_vec
                .iter()
                .any(|t| matches!(t, proc_macro2::TokenTree::Ident(ident) if ident == "init"));

            // Check for zero_copy keyword
            let has_zero_copy = token_vec
                .iter()
                .any(|t| matches!(t, proc_macro2::TokenTree::Ident(ident) if ident == "zero_copy"));

            if has_init {
                // If has mint namespace, it's a mint field
                if has_mint_namespace {
                    return (false, true, false, false);
                }
                // If has associated_token namespace, it's an ATA field
                if has_ata_namespace {
                    return (false, false, true, false);
                }
                // If has token namespace, it's NOT a PDA (handled separately)
                if has_token_namespace {
                    return (false, false, false, false);
                }
                // Otherwise it's a plain PDA init
                return (true, false, false, has_zero_copy);
            }
        }
    }
    (false, false, false, false)
}

// =============================================================================
// TOKEN ATTRIBUTE PARSING
// =============================================================================

/// Parsed #[light_account(token, ...)] or #[light_account(associated_token, ...)] attribute
struct LightTokenAttr {
    /// Optional variant name - if None, derived from field name
    variant_name: Option<Ident>,
    /// Owner PDA seeds - used when the token owner is a PDA that needs to sign.
    /// Must contain only constant values (byte literals, const references).
    owner_seeds: Option<Vec<ClassifiedSeed>>,
}

/// Extract #[light_account(token::..., ...)] attribute
/// Variant name is derived from field name, not specified in attribute
/// Returns Err if the attribute exists but has malformed syntax
///
/// Note: This function currently only handles `token` accounts, not `associated_token`.
/// Associated token accounts are handled differently (they use `authority` instead of `owner`).
/// The ExtractedTokenSpec struct is designed for token accounts with authority seeds.
fn extract_light_token_attr(
    attrs: &[syn::Attribute],
    instruction_args: &InstructionArgSet,
) -> syn::Result<Option<LightTokenAttr>> {
    for attr in attrs {
        if attr.path().is_ident("light_account") {
            let tokens = match &attr.meta {
                syn::Meta::List(list) => list.tokens.clone(),
                _ => continue,
            };

            // Check for token namespace (token::...) - new syntax
            // Look for pattern: ident "token" followed by "::"
            let token_vec: Vec<_> = tokens.clone().into_iter().collect();
            let has_token_namespace = token_vec.windows(2).any(|window| {
                matches!(
                    (&window[0], &window[1]),
                    (
                        proc_macro2::TokenTree::Ident(ident),
                        proc_macro2::TokenTree::Punct(punct)
                    ) if ident == "token" && punct.as_char() == ':'
                )
            });

            if has_token_namespace {
                // Parse attribute content - propagate errors instead of swallowing them
                let parsed = parse_light_token_list(&tokens, instruction_args, "token")?;
                return Ok(Some(parsed));
            }
        }
    }
    Ok(None)
}

/// Parse light_account(token::..., ...) content with namespace::key syntax
fn parse_light_token_list(
    tokens: &proc_macro2::TokenStream,
    instruction_args: &InstructionArgSet,
    account_type: &str,
) -> syn::Result<LightTokenAttr> {
    use syn::parse::Parser;

    // Capture instruction_args and account_type for use in closure
    let instruction_args = instruction_args.clone();
    let account_type_owned = account_type.to_string();
    let valid_keys = valid_keys_for_namespace(account_type);

    let parser = move |input: syn::parse::ParseStream| -> syn::Result<LightTokenAttr> {
        let mut owner_seeds = None;

        // Parse comma-separated items
        while !input.is_empty() {
            if input.peek(Ident) {
                let ident: Ident = input.parse()?;
                let ident_str = ident.to_string();

                // Check for namespace::key syntax FIRST (before standalone keywords)
                // because "token" can be both a standalone keyword and a namespace prefix
                if input.peek(syn::Token![:]) {
                    // Namespace::key syntax (e.g., token::owner_seeds = [...])
                    // Parse first colon
                    input.parse::<syn::Token![:]>()?;
                    // Parse second colon
                    if input.peek(syn::Token![:]) {
                        input.parse::<syn::Token![:]>()?;
                    }

                    let key: Ident = input.parse()?;
                    let key_str = key.to_string();

                    // Validate namespace matches expected account type
                    if ident_str != account_type_owned {
                        // Different namespace, skip (might be associated_token::)
                        // Just consume any value after =
                        if input.peek(syn::Token![=]) {
                            input.parse::<syn::Token![=]>()?;
                            let _expr: syn::Expr = input.parse()?;
                        }
                    } else {
                        // Validate key for this namespace
                        if !valid_keys.contains(&key_str.as_str()) {
                            return Err(syn::Error::new_spanned(
                                &key,
                                unknown_key_error(&account_type_owned, &key_str),
                            ));
                        }

                        // Check if value follows
                        if input.peek(syn::Token![=]) {
                            input.parse::<syn::Token![=]>()?;

                            if key_str == "owner_seeds" {
                                // Parse owner_seeds = [...] array
                                // The array is represented as a Group(Bracket) in proc_macro2
                                // Use input.step to manually handle the Group
                                let array_content = input.step(|cursor| {
                                    if let Some((group, _span, rest)) =
                                        cursor.group(proc_macro2::Delimiter::Bracket)
                                    {
                                        Ok((group.token_stream(), rest))
                                    } else {
                                        Err(cursor.error("expected bracketed array"))
                                    }
                                })?;

                                // Parse the array content
                                let elems: syn::punctuated::Punctuated<syn::Expr, syn::Token![,]> =
                                    syn::parse::Parser::parse2(
                                        syn::punctuated::Punctuated::parse_terminated,
                                        array_content,
                                    )?;
                                let mut seeds = Vec::new();
                                for elem in &elems {
                                    let seed = classify_seed_expr(elem, &instruction_args)
                                        .map_err(|e| {
                                            syn::Error::new_spanned(
                                                elem,
                                                format!("invalid owner seed: {}", e),
                                            )
                                        })?;
                                    seeds.push(seed);
                                }
                                owner_seeds = Some(seeds);
                            } else {
                                // Other keys (mint, owner, bump) - just consume the value
                                let _expr: syn::Expr = input.parse()?;
                            }
                        }
                        // If no = follows for shorthand keys, it's fine - we don't need the value
                    }
                } else if is_standalone_keyword(&ident_str) {
                    // Standalone keywords (init, token, associated_token, mint)
                    // Just continue - these don't require values
                } else {
                    // Unknown standalone identifier (not a keyword, not namespace::key)
                    return Err(syn::Error::new_spanned(
                        &ident,
                        format!(
                            "Unknown keyword `{}` in #[light_account(...)]. \
                             Use namespaced syntax: `{}::owner_seeds = [...]`, `{}::mint`, etc.",
                            ident_str, account_type_owned, account_type_owned
                        ),
                    ));
                }
            } else {
                // Non-identifier token - error
                let valid_kw_str = valid_keys.join(", ");
                return Err(syn::Error::new(
                    input.span(),
                    format!(
                        "Expected keyword in #[light_account(...)]. \
                         Valid namespaced keys: {}::{{{}}}, or standalone: init",
                        account_type_owned, valid_kw_str
                    ),
                ));
            }

            // Consume comma if present
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        // Validate that owner_seeds contain only constants
        if let Some(ref seeds) = owner_seeds {
            validate_owner_seeds_are_constants(seeds)?;
        }

        Ok(LightTokenAttr {
            variant_name: None, // Variant name is always derived from field name
            owner_seeds,
        })
    };

    parser.parse2(tokens.clone())
}

/// Validate that owner_seeds contain only constant values.
///
/// owner_seeds must only contain:
/// - `Literal(Vec<u8>)` - byte literals like b"seed"
/// - `Constant { path, expr }` - constant references like SEED.as_bytes()
///
/// The following are NOT allowed (they are dynamic values):
/// - `CtxRooted { account }` - ctx account references
/// - `DataRooted { root, expr }` - instruction data references
/// - `FunctionCall { ... }` - dynamic function calls
/// - `Passthrough(expr)` - unknown expressions
fn validate_owner_seeds_are_constants(seeds: &[ClassifiedSeed]) -> syn::Result<()> {
    for seed in seeds {
        match seed {
            ClassifiedSeed::Literal(_) | ClassifiedSeed::Constant { .. } => {
                // These are allowed - they are compile-time constants
                continue;
            }
            ClassifiedSeed::CtxRooted { account } => {
                return Err(syn::Error::new(
                    account.span(),
                    "owner_seeds must be constants only. \
                     Dynamic ctx account references like `authority.key()` are not allowed. \
                     Use only byte literals (b\"seed\") or const references (SEED.as_bytes()).",
                ));
            }
            ClassifiedSeed::DataRooted { root, .. } => {
                return Err(syn::Error::new(
                    root.span(),
                    "owner_seeds must be constants only. \
                     Instruction data references like `params.owner` are not allowed. \
                     Use only byte literals (b\"seed\") or const references (SEED.as_bytes()).",
                ));
            }
            ClassifiedSeed::FunctionCall { func_expr, .. } => {
                return Err(syn::Error::new_spanned(
                    func_expr,
                    "owner_seeds must be constants only. \
                     Dynamic function calls are not allowed. \
                     Use only byte literals (b\"seed\") or const references (SEED.as_bytes()).",
                ));
            }
            ClassifiedSeed::Passthrough(expr) => {
                return Err(syn::Error::new_spanned(
                    expr,
                    "owner_seeds must be constants only. \
                     This expression type is not recognized as a constant. \
                     Use only byte literals (b\"seed\") or const references (SEED.as_bytes()).",
                ));
            }
        }
    }
    Ok(())
}

// =============================================================================
// MAIN EXTRACTION FUNCTIONS
// =============================================================================

/// Extract light account field info from an Accounts struct.
///
/// This is the main extraction function used by `#[light_program]` that returns
/// richer metadata including variant names, struct names, and module paths.
///
/// Returns `None` if the struct has no light account fields.
pub fn extract_from_accounts_struct(
    item: &ItemStruct,
    instruction_args: &InstructionArgSet,
    module_path: &str,
) -> syn::Result<Option<ExtractedAccountsInfo>> {
    let fields = match &item.fields {
        syn::Fields::Named(named) => &named.named,
        _ => return Ok(None),
    };

    let mut pda_fields = Vec::new();
    let mut token_fields = Vec::new();
    let mut has_light_mint_fields = false;
    let mut has_light_ata_fields = false;

    for field in fields {
        let field_ident = match &field.ident {
            Some(id) => id.clone(),
            None => continue,
        };

        // Check for #[light_account(...)] attribute and determine its type
        let (has_light_account_pda, has_light_account_mint, has_light_account_ata, has_zero_copy) =
            check_light_account_type(&field.attrs);

        if has_light_account_mint {
            has_light_mint_fields = true;
        }
        if has_light_account_ata {
            has_light_ata_fields = true;
        }

        // Check for #[light_account(token, ...)] attribute
        let token_attr = extract_light_token_attr(&field.attrs, instruction_args)?;

        if has_light_account_pda {
            // Extract inner type from Account<'info, T> or Box<Account<'info, T>>
            // Note: is_boxed is not needed for ExtractedSeedSpec, only inner_type
            let (_, inner_type) =
                extract_account_inner_type(&field.ty).map_err(|e| e.into_syn_error(&field.ty))?;

            // Extract seeds from #[account(seeds = [...])]
            let seeds = extract_anchor_seeds(&field.attrs, instruction_args)?;

            // Derive variant name from field name: snake_case -> CamelCase
            let variant_name = {
                let camel = snake_to_camel_case(&field_ident.to_string());
                Ident::new(&camel, field_ident.span())
            };

            pda_fields.push(ExtractedSeedSpec {
                variant_name,
                inner_type,
                seeds,
                is_zero_copy: has_zero_copy,
                struct_name: item.ident.to_string(),
                module_path: module_path.to_string(),
            });
        } else if let Some(token_attr) = token_attr {
            // Token field - derive variant name from field name if not provided
            let seeds = extract_anchor_seeds(&field.attrs, instruction_args)?;

            // Derive variant name: snake_case field -> CamelCase variant
            let variant_name = token_attr.variant_name.unwrap_or_else(|| {
                let camel = snake_to_camel_case(&field_ident.to_string());
                Ident::new(&camel, field_ident.span())
            });

            token_fields.push(ExtractedTokenSpec {
                field_name: field_ident,
                variant_name,
                seeds,
                // Use owner_seeds from attribute if provided
                owner_seeds: token_attr.owner_seeds,
                module_path: module_path.to_string(),
            });
        }
    }

    // If no light account fields found, return None
    if pda_fields.is_empty()
        && token_fields.is_empty()
        && !has_light_mint_fields
        && !has_light_ata_fields
    {
        return Ok(None);
    }

    // Validate that all token fields have owner_seeds (required for decompression)
    for token in &token_fields {
        if token.owner_seeds.is_none() {
            return Err(syn::Error::new(
                token.field_name.span(),
                format!(
                    "Token account field '{}' requires owner_seeds. \
                     The owner must be a PDA derived from constant seeds for decompression.\n\
                     Add `token::owner_seeds = [b\"seed\", CONSTANT.as_bytes()]` to the #[light_account(...)] attribute.",
                    token.field_name,
                ),
            ));
        }
    }

    Ok(Some(ExtractedAccountsInfo {
        struct_name: item.ident.clone(),
        pda_fields,
        token_fields,
        has_light_mint_fields,
        has_light_ata_fields,
    }))
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::{
        super::{instruction_args::InstructionArgSet, types::ClassifiedSeed},
        *,
    };

    #[test]
    fn test_extract_account_inner_type() {
        let ty: syn::Type = parse_quote!(Account<'info, UserRecord>);
        let result = extract_account_inner_type(&ty);
        assert!(result.is_ok(), "Should extract Account inner type");
        let (is_boxed, inner) = result.unwrap();
        assert!(!is_boxed);

        if let syn::Type::Path(path) = inner {
            assert_eq!(
                path.path.segments.last().unwrap().ident.to_string(),
                "UserRecord"
            );
        } else {
            panic!("Expected path type");
        }
    }

    #[test]
    fn test_extract_account_inner_type_boxed() {
        let ty: syn::Type = parse_quote!(Box<Account<'info, UserRecord>>);
        let result = extract_account_inner_type(&ty);
        assert!(result.is_ok(), "Should extract Box<Account> inner type");
        let (is_boxed, inner) = result.unwrap();
        assert!(is_boxed);

        if let syn::Type::Path(path) = inner {
            assert_eq!(
                path.path.segments.last().unwrap().ident.to_string(),
                "UserRecord"
            );
        } else {
            panic!("Expected path type");
        }
    }

    #[test]
    fn test_extract_account_inner_type_nested_box_fails() {
        let ty: syn::Type = parse_quote!(Box<Box<Account<'info, UserRecord>>>);
        let result = extract_account_inner_type(&ty);
        assert!(
            matches!(result, Err(AccountTypeError::NestedBox)),
            "Nested Box should return NestedBox error"
        );
    }

    #[test]
    fn test_extract_account_inner_type_wrong_type_fails() {
        let ty: syn::Type = parse_quote!(String);
        let result = extract_account_inner_type(&ty);
        assert!(
            matches!(result, Err(AccountTypeError::WrongType { .. })),
            "Wrong type should return WrongType error"
        );
    }

    #[test]
    fn test_check_light_account_type_mint_namespace() {
        // Test that mint:: namespace is detected correctly
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[light_account(init,
                mint::signer = mint_signer,
                mint::authority = fee_payer,
                mint::decimals = 6
            )]
        )];
        let (has_pda, has_mint, has_ata, has_zero_copy) = check_light_account_type(&attrs);
        assert!(!has_pda, "Should NOT be detected as PDA");
        assert!(has_mint, "Should be detected as mint");
        assert!(!has_ata, "Should NOT be detected as ATA");
        assert!(!has_zero_copy, "Should NOT be detected as zero_copy");
    }

    #[test]
    fn test_check_light_account_type_pda_only() {
        // Test that plain init (no mint::) is detected as PDA
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[light_account(init)]
        )];
        let (has_pda, has_mint, has_ata, has_zero_copy) = check_light_account_type(&attrs);
        assert!(has_pda, "Should be detected as PDA");
        assert!(!has_mint, "Should NOT be detected as mint");
        assert!(!has_ata, "Should NOT be detected as ATA");
        assert!(!has_zero_copy, "Should NOT be detected as zero_copy");
    }

    #[test]
    fn test_extract_from_accounts_struct() {
        let item: syn::ItemStruct = parse_quote!(
            #[derive(Accounts)]
            #[instruction(params: CreateParams)]
            pub struct Create<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,

                #[account(
                    init,
                    payer = fee_payer,
                    space = 100,
                    seeds = [b"user", authority.key().as_ref()],
                    bump
                )]
                #[light_account(init)]
                pub user_record: Account<'info, UserRecord>,

                pub authority: Signer<'info>,
            }
        );

        let instruction_args = InstructionArgSet::from_names(["params".to_string()]);
        let result = extract_from_accounts_struct(&item, &instruction_args, "crate::instructions")
            .expect("should extract");

        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.struct_name.to_string(), "Create");
        assert_eq!(info.pda_fields.len(), 1);
        assert_eq!(info.pda_fields[0].variant_name.to_string(), "UserRecord");
        assert_eq!(info.pda_fields[0].seeds.len(), 2);
        assert!(!info.has_light_mint_fields);
        assert!(!info.has_light_ata_fields);
    }
}
