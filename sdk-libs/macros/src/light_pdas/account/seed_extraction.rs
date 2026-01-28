//! Anchor seed extraction from #[account(seeds = [...])] attributes.
//!
//! This module extracts PDA seeds from Anchor's attribute syntax and classifies them
//! into the categories needed for compression: literals, ctx fields, data fields, etc.

use std::collections::HashSet;

use syn::{Expr, Ident, ItemStruct, Type};

use super::validation::{type_name, AccountTypeError};
use crate::{
    light_pdas::{
        light_account_keywords::{
            is_standalone_keyword, unknown_key_error, valid_keys_for_namespace,
        },
        shared_utils::is_constant_identifier,
    },
    utils::snake_to_camel_case,
};

/// Set of instruction argument names for Format 2 detection.
///
/// Anchor supports two formats for `#[instruction(...)]`:
/// - Format 1: `#[instruction(params: SomeStruct)]` - users write `params.field`
/// - Format 2: `#[instruction(owner: Pubkey, amount: u64)]` - users write bare `owner`
///
/// This struct holds the names from Format 2 so we can recognize them in seed expressions.
#[derive(Clone, Debug, Default)]
pub struct InstructionArgSet {
    /// Names of instruction args (e.g., {"owner", "amount", "bump"})
    pub names: HashSet<String>,
}

impl InstructionArgSet {
    /// Create an empty arg set (used when no #[instruction] attribute present)
    pub fn empty() -> Self {
        Self {
            names: HashSet::new(),
        }
    }

    /// Create from a list of argument names
    pub fn from_names(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            names: names.into_iter().collect(),
        }
    }

    /// Check if a name is a known instruction argument
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }
}

/// Parse #[instruction(...)] attribute from a struct's attributes and return InstructionArgSet
pub fn parse_instruction_arg_names(attrs: &[syn::Attribute]) -> syn::Result<InstructionArgSet> {
    for attr in attrs {
        if attr.path().is_ident("instruction") {
            let content = attr.parse_args_with(|input: syn::parse::ParseStream| {
                let args: syn::punctuated::Punctuated<InstructionArg, syn::Token![,]> =
                    syn::punctuated::Punctuated::parse_terminated(input)?;
                Ok(args
                    .into_iter()
                    .map(|a| a.name.to_string())
                    .collect::<Vec<_>>())
            })?;
            return Ok(InstructionArgSet::from_names(content));
        }
    }
    Ok(InstructionArgSet::empty())
}

/// Helper struct for parsing instruction args
struct InstructionArg {
    name: syn::Ident,
    #[allow(dead_code)]
    ty: syn::Type,
}

impl syn::parse::Parse for InstructionArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        let ty = input.parse()?;
        Ok(Self { name, ty })
    }
}

/// Classified seed element from Anchor's seeds array.
///
/// Uses prefix detection + passthrough strategy:
/// - Identifies the root (ctx/data/constant/literal) to determine which namespace
/// - Passes through the full expression unchanged for code generation
/// - Complex expressions like `identity_seed::<12>(b"seed")` become Passthrough
#[derive(Clone, Debug)]
pub enum ClassifiedSeed {
    /// b"literal" or "string" - hardcoded bytes
    Literal(Vec<u8>),
    /// CONSTANT or path::CONSTANT - uppercase identifier.
    /// `path` is the extracted constant path (for crate:: qualification).
    /// `expr` is the full original expression (e.g., `SEED.as_bytes()`) for codegen.
    Constant {
        path: syn::Path,
        expr: Box<syn::Expr>,
    },
    /// Expression rooted in ctx account (e.g., authority.key().as_ref())
    /// `account` is the root identifier
    CtxRooted { account: Ident },
    /// Expression rooted in instruction arg (e.g., params.owner.as_ref())
    /// `root` is the instruction arg name, `expr` is the full expression for codegen
    DataRooted { root: Ident, expr: Box<syn::Expr> },
    /// Function call with dynamic arguments (e.g., crate::max_key(&params.key_a, &params.key_b).as_ref())
    /// Detected when `Expr::Call` or `Expr::MethodCall(receiver=Expr::Call)` has args
    /// rooted in instruction data or ctx accounts.
    FunctionCall {
        /// The full function call expression (without trailing .as_ref()/.as_bytes())
        func_expr: Box<syn::Expr>,
        /// Classified arguments to the function
        args: Vec<ClassifiedFnArg>,
        /// Whether the original expression had trailing .as_ref() or .as_bytes()
        has_as_ref: bool,
    },
    /// Everything else - pass through unchanged
    Passthrough(Box<syn::Expr>),
}

/// A classified argument to a function call seed.
#[derive(Clone, Debug)]
pub struct ClassifiedFnArg {
    /// The field name extracted from the argument (e.g., `key_a` from `&params.key_a`)
    pub field_name: Ident,
    /// Whether this is a ctx account or instruction data field
    pub kind: FnArgKind,
}

/// Classification of a function call argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FnArgKind {
    /// Argument is rooted in a ctx account field
    CtxAccount,
    /// Argument is rooted in instruction data
    DataField,
}

/// Extracted seed specification for a compressible field
#[derive(Clone, Debug)]
pub struct ExtractedSeedSpec {
    /// The variant name derived from field_name (snake_case -> CamelCase)
    /// Note: Currently unused as we use inner_type for seed spec correlation,
    /// but kept for potential future use cases (e.g., custom variant naming).
    #[allow(dead_code)]
    pub variant_name: Ident,
    /// The inner type (e.g., crate::state::UserRecord from Account<'info, UserRecord>)
    /// Preserves the full type path for code generation.
    pub inner_type: Type,
    /// Classified seeds from #[account(seeds = [...])]
    pub seeds: Vec<ClassifiedSeed>,
    /// True if the field uses zero-copy serialization (AccountLoader)
    pub is_zero_copy: bool,
    /// The instruction struct name this field was extracted from (for error messages)
    pub struct_name: String,
    /// The full module path where this struct was defined (e.g., "crate::instructions::create")
    /// Used to qualify bare constant/function names in seed expressions.
    pub module_path: String,
}

/// Extracted token specification for a #[light_account(token, ...)] field
#[derive(Clone, Debug)]
pub struct ExtractedTokenSpec {
    /// The field name in the Accounts struct
    pub field_name: Ident,
    /// The variant name derived from field name
    pub variant_name: Ident,
    /// Seeds from #[account(seeds = [...])]
    pub seeds: Vec<ClassifiedSeed>,
    /// Authority field name (if specified or auto-detected)
    pub authority_field: Option<Ident>,
    /// Authority seeds (from the authority field's #[account(seeds)])
    pub authority_seeds: Option<Vec<ClassifiedSeed>>,
    /// The full module path where this struct was defined (e.g., "crate::instructions::create")
    /// Used to qualify bare constant/function names in seed expressions.
    pub module_path: String,
}

/// All extracted info from an Accounts struct
#[derive(Clone, Debug)]
pub struct ExtractedAccountsInfo {
    pub struct_name: Ident,
    pub pda_fields: Vec<ExtractedSeedSpec>,
    pub token_fields: Vec<ExtractedTokenSpec>,
    /// True if struct has any #[light_account(init, mint::...)] fields
    pub has_light_mint_fields: bool,
    /// True if struct has any #[light_account(init, associated_token::...)] fields
    pub has_light_ata_fields: bool,
}

/// Extract rentfree field info from an Accounts struct
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
            let (_, inner_type) = extract_account_inner_type(&field.ty)
                .map_err(|e| e.into_syn_error(&field.ty))?;

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
                authority_field: None,
                // Use authority from attribute if provided
                authority_seeds: token_attr.authority_seeds,
                module_path: module_path.to_string(),
            });
        }
    }

    // If no rentfree/light_mint/ata fields found, return None
    if pda_fields.is_empty()
        && token_fields.is_empty()
        && !has_light_mint_fields
        && !has_light_ata_fields
    {
        return Ok(None);
    }

    // Resolve authority for token fields (only if not already provided in attribute)
    for token in &mut token_fields {
        // Skip if authority was already provided in the attribute
        if token.authority_seeds.is_some() {
            continue;
        }

        // Try to find authority field by convention: {field_name}_authority or vault_authority
        let authority_candidates = [
            format!("{}_authority", token.field_name),
            "vault_authority".to_string(),
            "authority".to_string(),
        ];

        for candidate in &authority_candidates {
            // Search fields directly instead of using a separate all_fields collection
            if let Some(auth_field_info) = fields
                .iter()
                .find(|f| f.ident.as_ref().map(|i| i.to_string()) == Some(candidate.clone()))
            {
                if let Some(auth_ident) = &auth_field_info.ident {
                    token.authority_field = Some(auth_ident.clone());

                    // Try to extract authority seeds from the authority field
                    if let Ok(auth_seeds) =
                        extract_anchor_seeds(&auth_field_info.attrs, instruction_args)
                    {
                        if !auth_seeds.is_empty() {
                            token.authority_seeds = Some(auth_seeds);
                        }
                    }
                    break;
                }
            }
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

/// Check #[light_account(...)] attributes for PDA, mint, or ATA type.
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

/// Parsed #[light_account(token, ...)] or #[light_account(associated_token, ...)] attribute
struct LightTokenAttr {
    /// Optional variant name - if None, derived from field name
    variant_name: Option<Ident>,
    authority_seeds: Option<Vec<ClassifiedSeed>>,
    /// The account type: "token" or "associated_token"
    #[allow(dead_code)]
    account_type: String,
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
        let mut authority_seeds = None;

        // Parse comma-separated items
        while !input.is_empty() {
            if input.peek(Ident) {
                let ident: Ident = input.parse()?;
                let ident_str = ident.to_string();

                // Check for namespace::key syntax FIRST (before standalone keywords)
                // because "token" can be both a standalone keyword and a namespace prefix
                if input.peek(syn::Token![:]) {
                    // Namespace::key syntax (e.g., token::authority = [...])
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

                            if key_str == "authority" {
                                // Parse authority = [...] array
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
                                                format!("invalid authority seed: {}", e),
                                            )
                                        })?;
                                    seeds.push(seed);
                                }
                                authority_seeds = Some(seeds);
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
                             Use namespaced syntax: `{}::authority = [...]`, `{}::mint`, etc.",
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

        Ok(LightTokenAttr {
            variant_name: None, // Variant name is always derived from field name
            authority_seeds,
            account_type: account_type_owned.clone(),
        })
    };

    parser.parse2(tokens.clone())
}

/// Extract inner type T from Account<'info, T>, Box<Account<'info, T>>,
/// AccountLoader<'info, T>, or InterfaceAccount<'info, T>
///
/// Returns the full type path (e.g., `crate::module::MyRecord`) to preserve
/// module qualification for code generation.
///
/// # Returns
/// - `Ok((is_boxed, inner_type))` on success
/// - `Err(AccountTypeError::WrongType)` if the type is not Account/Box/AccountLoader/InterfaceAccount
/// - `Err(AccountTypeError::NestedBox)` if nested Box<Box<...>> is detected
/// - `Err(AccountTypeError::ExtractionFailed)` if generic arguments couldn't be extracted
pub fn extract_account_inner_type(ty: &Type) -> Result<(bool, Type), AccountTypeError> {
    match ty {
        Type::Path(type_path) => {
            let segment = type_path
                .path
                .segments
                .last()
                .ok_or_else(|| AccountTypeError::WrongType {
                    got: type_name(ty),
                })?;
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
                                        // Skip lifetime 'info TODO: add a helper that is generalized to strip lifetimes or check whether a crate already has this
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
                _ => Err(AccountTypeError::WrongType {
                    got: type_name(ty),
                }),
            }
        }
        _ => Err(AccountTypeError::WrongType {
            got: type_name(ty),
        }),
    }
}

/// Extract seeds from #[account(seeds = [...], bump)] attribute
pub fn extract_anchor_seeds(
    attrs: &[syn::Attribute],
    instruction_args: &InstructionArgSet,
) -> syn::Result<Vec<ClassifiedSeed>> {
    for attr in attrs {
        if !attr.path().is_ident("account") {
            continue;
        }

        // Parse the attribute as a token stream and look for seeds = [...]
        let tokens = match &attr.meta {
            syn::Meta::List(list) => list.tokens.clone(),
            _ => continue,
        };

        // Parse as comma-separated key-value pairs
        let parsed: syn::Result<syn::punctuated::Punctuated<AccountAttrItem, syn::Token![,]>> =
            syn::parse::Parser::parse2(
                syn::punctuated::Punctuated::parse_terminated,
                tokens.clone(),
            );

        if let Ok(items) = &parsed {
            for item in items {
                if item.key == "seeds" {
                    return classify_seeds_array(&item.value, instruction_args);
                }
            }
        }
    }

    Ok(Vec::new())
}

/// Helper struct for parsing account attribute items
struct AccountAttrItem {
    key: Ident,
    value: Expr,
}

impl syn::parse::Parse for AccountAttrItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Handle keywords like `mut` as well as identifiers
        let key: Ident = if input.peek(syn::Token![mut]) {
            input.parse::<syn::Token![mut]>()?;
            Ident::new("mut", proc_macro2::Span::call_site())
        } else {
            input.parse()?
        };

        // Handle bare identifiers like `mut`, `init`, `bump`
        if !input.peek(syn::Token![=]) {
            return Ok(AccountAttrItem {
                key: key.clone(),
                value: syn::parse_quote!(true),
            });
        }

        input.parse::<syn::Token![=]>()?;
        let value: Expr = input.parse()?;

        Ok(AccountAttrItem { key, value })
    }
}

/// Classify seeds from an array expression [seed1, seed2, ...]
fn classify_seeds_array(
    expr: &Expr,
    instruction_args: &InstructionArgSet,
) -> syn::Result<Vec<ClassifiedSeed>> {
    let array = match expr {
        Expr::Array(arr) => arr,
        Expr::Reference(r) => {
            if let Expr::Array(arr) = &*r.expr {
                arr
            } else {
                return Err(syn::Error::new_spanned(expr, "Expected seeds array"));
            }
        }
        _ => return Err(syn::Error::new_spanned(expr, "Expected seeds array")),
    };

    let mut seeds = Vec::new();
    for elem in &array.elems {
        seeds.push(classify_seed_expr(elem, instruction_args)?);
    }

    Ok(seeds)
}

/// Classify a single seed expression using prefix detection + passthrough.
///
/// Strategy:
/// 1. Byte literals -> Literal
/// 2. Uppercase paths -> Constant
/// 3. Check if rooted in instruction arg -> DataRooted (pass through full expr)
/// 4. Check if rooted in ctx account -> CtxRooted (pass through full expr)
/// 5. Function calls with dynamic args -> FunctionCall
/// 6. Everything else -> Passthrough
pub fn classify_seed_expr(
    expr: &Expr,
    instruction_args: &InstructionArgSet,
) -> syn::Result<ClassifiedSeed> {
    // Handle byte string literals
    if let Some(bytes) = extract_byte_literal(expr) {
        return Ok(ClassifiedSeed::Literal(bytes));
    }

    // Handle constants (uppercase paths)
    if let Some(path) = extract_constant_path(expr) {
        return Ok(ClassifiedSeed::Constant {
            path,
            expr: Box::new(expr.clone()),
        });
    }

    // Check if rooted in instruction arg
    if let Some(root) = get_instruction_arg_root(expr, instruction_args) {
        return Ok(ClassifiedSeed::DataRooted {
            root,
            expr: Box::new(expr.clone()),
        });
    }

    // Check if rooted in ctx account
    if let Some(account) = get_ctx_account_root(expr) {
        return Ok(ClassifiedSeed::CtxRooted { account });
    }

    // Check for function calls with dynamic arguments
    if let Some(fc) = classify_function_call(expr, instruction_args) {
        return Ok(fc);
    }

    // Everything else: passthrough
    Ok(ClassifiedSeed::Passthrough(Box::new(expr.clone())))
}

/// Attempt to classify an expression as a FunctionCall seed.
///
/// Detects patterns like:
/// - `func(arg1, arg2)` -> Expr::Call
/// - `func(arg1, arg2).as_ref()` -> Expr::MethodCall(receiver=Expr::Call)
///
/// Returns `Some(ClassifiedSeed::FunctionCall{...})` if:
/// - The expression contains an `Expr::Call` (at top-level or as receiver of `.as_ref()`)
/// - At least one argument is rooted in instruction data or ctx accounts
///
/// Returns `None` if:
/// - Not a function call pattern
/// - No dynamic arguments (falls through to Passthrough)
fn classify_function_call(
    expr: &Expr,
    instruction_args: &InstructionArgSet,
) -> Option<ClassifiedSeed> {
    // Strip trailing .as_ref() / .as_bytes() to find the call expression
    let (call_expr, has_as_ref) = strip_trailing_as_ref(expr);

    // Check if the (possibly stripped) expression is a function call
    let call = match call_expr {
        Expr::Call(c) => c,
        _ => return None,
    };

    // Classify each argument
    let mut classified_args = Vec::new();
    let mut has_dynamic = false;

    for arg in &call.args {
        // Unwrap references for classification
        let inner = unwrap_references(arg);

        // Check if rooted in instruction arg
        if let Some(root) = get_instruction_arg_root(inner, instruction_args) {
            // Extract terminal field name (e.g., key_a from params.key_a)
            let field_name = extract_terminal_field_name(inner).unwrap_or(root);
            classified_args.push(ClassifiedFnArg {
                field_name,
                kind: FnArgKind::DataField,
            });
            has_dynamic = true;
            continue;
        }

        // Check if rooted in ctx account
        if let Some(account) = get_ctx_account_root(inner) {
            classified_args.push(ClassifiedFnArg {
                field_name: account,
                kind: FnArgKind::CtxAccount,
            });
            has_dynamic = true;
            continue;
        }

        // Not dynamic -- skip this arg (will be inlined as-is in codegen)
    }

    if !has_dynamic {
        return None;
    }

    Some(ClassifiedSeed::FunctionCall {
        func_expr: Box::new(Expr::Call(call.clone())),
        args: classified_args,
        has_as_ref,
    })
}

/// Strip trailing `.as_ref()` or `.as_bytes()` method calls from an expression.
/// Returns the inner expression and a flag indicating whether stripping occurred.
fn strip_trailing_as_ref(expr: &Expr) -> (&Expr, bool) {
    if let Expr::MethodCall(mc) = expr {
        let method = mc.method.to_string();
        if (method == "as_ref" || method == "as_bytes") && mc.args.is_empty() {
            return (&mc.receiver, true);
        }
    }
    (expr, false)
}

/// Unwrap reference expressions (&expr, &mut expr) to get the inner expression.
fn unwrap_references(expr: &Expr) -> &Expr {
    match expr {
        Expr::Reference(r) => unwrap_references(&r.expr),
        _ => expr,
    }
}

/// Extract the terminal (deepest) field name from an expression.
/// For `params.key_a.as_ref()` returns `key_a`.
/// For `params.key_a` returns `key_a`.
/// For bare `owner` returns `owner`.
fn extract_terminal_field_name(expr: &Expr) -> Option<Ident> {
    match expr {
        Expr::Field(field) => {
            if let syn::Member::Named(name) = &field.member {
                Some(name.clone())
            } else {
                None
            }
        }
        Expr::MethodCall(mc) => extract_terminal_field_name(&mc.receiver),
        Expr::Reference(r) => extract_terminal_field_name(&r.expr),
        Expr::Path(path) => path.path.get_ident().cloned(),
        _ => None,
    }
}

/// Extract byte literal from expression.
/// Handles: b"literal", "string", b"literal"[..]
fn extract_byte_literal(expr: &Expr) -> Option<Vec<u8>> {
    match expr {
        Expr::Lit(lit) => {
            if let syn::Lit::ByteStr(bs) = &lit.lit {
                return Some(bs.value());
            }
            if let syn::Lit::Str(s) = &lit.lit {
                return Some(s.value().into_bytes());
            }
            None
        }
        // Handle b"literal"[..] - full range slice
        Expr::Index(idx) => {
            if let Expr::Range(range) = &*idx.index {
                if range.start.is_none() && range.end.is_none() {
                    if let Expr::Lit(lit) = &*idx.expr {
                        if let syn::Lit::ByteStr(bs) = &lit.lit {
                            return Some(bs.value());
                        }
                    }
                }
            }
            None
        }
        // Unwrap references
        Expr::Reference(r) => extract_byte_literal(&r.expr),
        _ => None,
    }
}

/// Extract constant path from expression.
/// Handles: CONSTANT, path::CONSTANT, CONSTANT.as_bytes(), CONSTANT.as_ref()
/// Does NOT handle type-qualified paths like <T as Trait>::CONST (returns None for passthrough)
fn extract_constant_path(expr: &Expr) -> Option<syn::Path> {
    match expr {
        Expr::Path(path) => {
            // Type-qualified paths go to passthrough
            if path.qself.is_some() {
                return None;
            }

            if let Some(ident) = path.path.get_ident() {
                // Single-segment uppercase path
                if is_constant_identifier(&ident.to_string()) {
                    return Some(path.path.clone());
                }
            } else if let Some(last_seg) = path.path.segments.last() {
                // Multi-segment path - check if last segment is uppercase
                if is_constant_identifier(&last_seg.ident.to_string()) {
                    return Some(path.path.clone());
                }
            }
            None
        }
        // Unwrap references
        Expr::Reference(r) => extract_constant_path(&r.expr),
        // Handle method calls on constants: CONSTANT.as_bytes(), CONSTANT.as_ref()
        Expr::MethodCall(mc) => extract_constant_path(&mc.receiver),
        _ => None,
    }
}

/// Get the root instruction arg identifier if expression is rooted in one.
/// Returns the instruction arg name (e.g., "params", "owner", "data").
fn get_instruction_arg_root(expr: &Expr, instruction_args: &InstructionArgSet) -> Option<Ident> {
    match expr {
        // Bare identifier: owner, amount (Format 2)
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                let name = ident.to_string();
                // Skip uppercase (constants) and check if it's an instruction arg
                if !is_constant_identifier(&name) && instruction_args.contains(&name) {
                    return Some(ident.clone());
                }
            }
            None
        }
        // Field access: params.owner, data.field.nested
        Expr::Field(field) => get_instruction_arg_root(&field.base, instruction_args),
        // Method call: params.owner.as_ref(), owner.to_le_bytes()
        Expr::MethodCall(mc) => get_instruction_arg_root(&mc.receiver, instruction_args),
        // Index: params.arrays[0]
        Expr::Index(idx) => get_instruction_arg_root(&idx.expr, instruction_args),
        // Reference: &params.owner
        Expr::Reference(r) => get_instruction_arg_root(&r.expr, instruction_args),
        _ => None,
    }
}

/// Get the root ctx account identifier if expression is rooted in one.
/// Returns the account name (e.g., "authority", "owner").
fn get_ctx_account_root(expr: &Expr) -> Option<Ident> {
    match expr {
        // Bare identifier (not uppercase): authority, owner
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                let name = ident.to_string();
                // Skip uppercase (constants)
                if !is_constant_identifier(&name) {
                    return Some(ident.clone());
                }
            }
            None
        }
        // Field access: authority.key, ctx.accounts.authority
        Expr::Field(field) => {
            // First check if terminal member is named
            if let syn::Member::Named(field_name) = &field.member {
                // If base is a simple path (like ctx.accounts), return the field
                // Otherwise recurse into the base
                match &*field.base {
                    Expr::Path(_) => Some(field_name.clone()),
                    Expr::Field(_) => {
                        // For ctx.accounts.authority - take terminal field
                        Some(field_name.clone())
                    }
                    _ => get_ctx_account_root(&field.base),
                }
            } else {
                None
            }
        }
        // Method call: authority.key().as_ref()
        Expr::MethodCall(mc) => get_ctx_account_root(&mc.receiver),
        // Reference: &authority.key()
        Expr::Reference(r) => get_ctx_account_root(&r.expr),
        _ => None,
    }
}

/// Get data field names from classified seeds.
/// Extracts the terminal field name from DataRooted expressions.
pub fn get_data_fields(seeds: &[ClassifiedSeed]) -> Vec<(Ident, Option<Ident>)> {
    let mut fields = Vec::new();
    for seed in seeds {
        match seed {
            ClassifiedSeed::DataRooted { expr, .. } => {
                if let Some((field_name, conversion)) = extract_data_field_info(expr) {
                    if !fields.iter().any(|(f, _): &(Ident, _)| f == &field_name) {
                        fields.push((field_name, conversion));
                    }
                }
            }
            ClassifiedSeed::FunctionCall { args, .. } => {
                // Include DataField args from function calls (e.g., max_key(&params.key_a, &params.key_b))
                for arg in args {
                    if matches!(arg.kind, FnArgKind::DataField) {
                        let field_name = arg.field_name.clone();
                        if !fields.iter().any(|(f, _): &(Ident, _)| *f == field_name) {
                            // FunctionCall data args are Pubkey by default (no conversion)
                            fields.push((field_name, None));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    fields
}

/// Extract field name and conversion method from a data-rooted expression.
/// Returns (field_name, Some(method)) for expressions like `params.field.to_le_bytes()`.
pub fn extract_data_field_info(expr: &Expr) -> Option<(Ident, Option<Ident>)> {
    match expr {
        // Bare identifier: amount (Format 2 instruction arg used directly)
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                return Some((ident.clone(), None));
            }
            None
        }
        // Field access: params.owner, data.field
        Expr::Field(field) => {
            if let syn::Member::Named(field_name) = &field.member {
                return Some((field_name.clone(), None));
            }
            None
        }
        // Method call: params.field.to_le_bytes(), amount.as_ref()
        Expr::MethodCall(mc) => {
            let method_name = mc.method.to_string();
            // Check for conversion methods
            if method_name == "to_le_bytes" || method_name == "to_be_bytes" {
                if let Some((field_name, _)) = extract_data_field_info(&mc.receiver) {
                    return Some((field_name, Some(mc.method.clone())));
                }
            }
            // Skip .as_ref(), .as_bytes(), etc. and recurse
            if method_name == "as_ref" || method_name == "as_bytes" || method_name == "as_slice" {
                return extract_data_field_info(&mc.receiver);
            }
            None
        }
        // Index: params.arrays[0]
        Expr::Index(idx) => extract_data_field_info(&idx.expr),
        // Reference: &params.owner
        Expr::Reference(r) => extract_data_field_info(&r.expr),
        _ => None,
    }
}

/// Get params-only seed fields from a TokenSeedSpec.
/// This is a convenience wrapper that works with the SeedElement type.
pub fn get_params_only_seed_fields_from_spec(
    spec: &crate::light_pdas::program::instructions::TokenSeedSpec,
    state_field_names: &std::collections::HashSet<String>,
) -> Vec<(Ident, syn::Type, bool)> {
    use crate::light_pdas::program::instructions::SeedElement;

    let mut fields = Vec::new();
    for seed in &spec.seeds {
        if let SeedElement::Expression(expr) = seed {
            // Extract data fields from top-level expressions (e.g., data.owner.as_ref())
            if let Some((field_name, has_conversion)) = extract_data_field_from_expr(expr) {
                add_params_only_field(&field_name, has_conversion, state_field_names, &mut fields);
            }
            // Also extract data fields from function call arguments
            // (e.g., crate::max_key(&data.key_a, &data.key_b).as_ref())
            extract_data_fields_from_nested_calls(expr, state_field_names, &mut fields);
        }
    }
    fields
}

/// Add a params-only field if it's not on the state struct and not already added.
fn add_params_only_field(
    field_name: &Ident,
    has_conversion: bool,
    state_field_names: &std::collections::HashSet<String>,
    fields: &mut Vec<(Ident, syn::Type, bool)>,
) {
    let field_str = field_name.to_string();
    if !state_field_names.contains(&field_str)
        && !fields
            .iter()
            .any(|(f, _, _): &(Ident, _, _)| f == field_name)
    {
        let field_type: syn::Type = if has_conversion {
            syn::parse_quote!(u64)
        } else {
            syn::parse_quote!(Pubkey)
        };
        fields.push((field_name.clone(), field_type, has_conversion));
    }
}

/// Recursively extract data fields from function call arguments within an expression.
fn extract_data_fields_from_nested_calls(
    expr: &syn::Expr,
    state_field_names: &std::collections::HashSet<String>,
    fields: &mut Vec<(Ident, syn::Type, bool)>,
) {
    match expr {
        syn::Expr::Call(call) => {
            for arg in &call.args {
                if let Some((field_name, has_conversion)) = extract_data_field_from_expr(arg) {
                    add_params_only_field(&field_name, has_conversion, state_field_names, fields);
                }
                extract_data_fields_from_nested_calls(arg, state_field_names, fields);
            }
        }
        syn::Expr::MethodCall(mc) => {
            extract_data_fields_from_nested_calls(&mc.receiver, state_field_names, fields);
            for arg in &mc.args {
                extract_data_fields_from_nested_calls(arg, state_field_names, fields);
            }
        }
        syn::Expr::Reference(r) => {
            extract_data_fields_from_nested_calls(&r.expr, state_field_names, fields);
        }
        _ => {}
    }
}

/// Extract the terminal field name from a DataRooted seed expression.
///
/// For `params.owner.as_ref()` returns `owner`.
/// For `params.nonce.to_le_bytes()` returns `nonce`.
/// For bare `owner` returns `owner`.
pub fn extract_data_field_name_from_expr(expr: &syn::Expr) -> Option<Ident> {
    // Try extract_data_field_info first (works for most expressions)
    if let Some((field, _)) = extract_data_field_info(expr) {
        return Some(field);
    }
    // Fallback: try extract_data_field_from_expr (handles data.X pattern)
    extract_data_field_from_expr(expr).map(|(name, _)| name)
}

/// Extract data field name and conversion info from an expression.
/// Returns (field_name, has_conversion) if the expression is a data.* field.
fn extract_data_field_from_expr(expr: &syn::Expr) -> Option<(Ident, bool)> {
    use crate::light_pdas::shared_utils::is_base_path;

    match expr {
        syn::Expr::Field(field_expr) => {
            if let syn::Member::Named(field_name) = &field_expr.member {
                if is_base_path(&field_expr.base, "data") {
                    return Some((field_name.clone(), false));
                }
            }
            None
        }
        syn::Expr::MethodCall(method_call) => {
            // Handle data.field.to_le_bytes().as_ref() etc.
            let has_bytes_conversion =
                method_call.method == "to_le_bytes" || method_call.method == "to_be_bytes";
            if has_bytes_conversion {
                return extract_data_field_from_expr(&method_call.receiver)
                    .map(|(name, _)| (name, true));
            }
            // For .as_ref(), recurse without marking conversion
            if method_call.method == "as_ref" || method_call.method == "as_bytes" {
                return extract_data_field_from_expr(&method_call.receiver);
            }
            None
        }
        syn::Expr::Reference(ref_expr) => extract_data_field_from_expr(&ref_expr.expr),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    fn make_instruction_args(names: &[&str]) -> InstructionArgSet {
        InstructionArgSet::from_names(names.iter().map(|s| s.to_string()))
    }

    #[test]
    fn test_bare_pubkey_instruction_arg() {
        // Format 2: bare instruction arg "owner" should be DataRooted
        let args = make_instruction_args(&["owner", "amount"]);
        let expr: syn::Expr = parse_quote!(owner);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "owner"));
    }

    #[test]
    fn test_bare_primitive_with_to_le_bytes() {
        // Format 2: amount.to_le_bytes() should be DataRooted with root "amount"
        let args = make_instruction_args(&["amount"]);
        let expr: syn::Expr = parse_quote!(amount.to_le_bytes().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(
            result,
            ClassifiedSeed::DataRooted { root, .. } if root == "amount"
        ));
    }

    #[test]
    fn test_custom_struct_param_name() {
        // Custom param name "input" - should be DataRooted with root "input"
        let args = make_instruction_args(&["input"]);
        let expr: syn::Expr = parse_quote!(input.owner.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "input"));
    }

    #[test]
    fn test_nested_field_access() {
        // data.inner.key should be DataRooted with root "data"
        let args = make_instruction_args(&["data"]);
        let expr: syn::Expr = parse_quote!(data.inner.key.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "data"));
    }

    #[test]
    fn test_context_account_not_confused_with_arg() {
        let args = make_instruction_args(&["owner"]); // "authority" is NOT an arg
        let expr: syn::Expr = parse_quote!(authority.key().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(
            result,
            ClassifiedSeed::CtxRooted { account, .. } if account == "authority"
        ));
    }

    #[test]
    fn test_empty_instruction_args() {
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(owner);
        let result = classify_seed_expr(&expr, &args).unwrap();
        // Without instruction args, bare ident treated as ctx account
        assert!(matches!(result, ClassifiedSeed::CtxRooted { account, .. } if account == "owner"));
    }

    #[test]
    fn test_literal_seed() {
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(b"seed");
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::Literal(bytes) if bytes == b"seed"));
    }

    #[test]
    fn test_constant_seed() {
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(SEED_PREFIX);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::Constant { .. }));
    }

    #[test]
    fn test_standard_params_field_access() {
        // Traditional format: #[instruction(params: CreateParams)]
        let args = make_instruction_args(&["params"]);
        let expr: syn::Expr = parse_quote!(params.owner.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "params"));
    }

    #[test]
    fn test_args_naming_format() {
        // Alternative naming: #[instruction(args: MyArgs)]
        let args = make_instruction_args(&["args"]);
        let expr: syn::Expr = parse_quote!(args.key.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "args"));
    }

    #[test]
    fn test_data_naming_format() {
        // Alternative naming: #[instruction(data: DataInput)]
        let args = make_instruction_args(&["data"]);
        let expr: syn::Expr = parse_quote!(data.value.to_le_bytes().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(
            result,
            ClassifiedSeed::DataRooted { root, .. } if root == "data"
        ));
    }

    #[test]
    fn test_format2_multiple_params() {
        // Format 2: #[instruction(owner: Pubkey, amount: u64)]
        let args = make_instruction_args(&["owner", "amount"]);

        let expr1: syn::Expr = parse_quote!(owner.as_ref());
        let result1 = classify_seed_expr(&expr1, &args).unwrap();
        assert!(matches!(result1, ClassifiedSeed::DataRooted { root, .. } if root == "owner"));

        let expr2: syn::Expr = parse_quote!(amount.to_le_bytes().as_ref());
        let result2 = classify_seed_expr(&expr2, &args).unwrap();
        assert!(matches!(
            result2,
            ClassifiedSeed::DataRooted { root, .. } if root == "amount"
        ));
    }

    #[test]
    fn test_passthrough_for_complex_expressions() {
        // Type-qualified paths should become Passthrough
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(<Type as Trait>::CONST);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::Passthrough(_)));
    }

    #[test]
    fn test_passthrough_for_generic_function_call() {
        // Complex function calls with no dynamic args should become Passthrough
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(identity_seed::<12>(b"seed"));
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::Passthrough(_)));
    }

    #[test]
    fn test_function_call_with_data_args() {
        // crate::max_key(&params.key_a, &params.key_b).as_ref() should be FunctionCall
        let args = make_instruction_args(&["params"]);
        let expr: syn::Expr = parse_quote!(crate::max_key(&params.key_a, &params.key_b).as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        match result {
            ClassifiedSeed::FunctionCall {
                args: fn_args,
                has_as_ref,
                ..
            } => {
                assert!(has_as_ref, "Should detect trailing .as_ref()");
                assert_eq!(fn_args.len(), 2, "Should have 2 classified args");
                assert_eq!(fn_args[0].field_name.to_string(), "key_a");
                assert_eq!(fn_args[0].kind, FnArgKind::DataField);
                assert_eq!(fn_args[1].field_name.to_string(), "key_b");
                assert_eq!(fn_args[1].kind, FnArgKind::DataField);
            }
            other => panic!("Expected FunctionCall, got {:?}", other),
        }
    }

    #[test]
    fn test_function_call_with_ctx_args() {
        // some_func(&fee_payer, &authority).as_ref() with no instruction args
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(some_func(&fee_payer, &authority).as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        match result {
            ClassifiedSeed::FunctionCall {
                args: fn_args,
                has_as_ref,
                ..
            } => {
                assert!(has_as_ref);
                assert_eq!(fn_args.len(), 2);
                assert_eq!(fn_args[0].kind, FnArgKind::CtxAccount);
                assert_eq!(fn_args[1].kind, FnArgKind::CtxAccount);
            }
            other => panic!("Expected FunctionCall, got {:?}", other),
        }
    }

    #[test]
    fn test_function_call_no_dynamic_args_becomes_passthrough() {
        // crate::id().as_ref() -- no dynamic args -> Passthrough
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(crate::id().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::Passthrough(_)),
            "No-arg function call should be Passthrough, got {:?}",
            result
        );
    }

    #[test]
    fn test_constant_method_call_not_function_call() {
        // SeedHolder::NAMESPACE.as_bytes() should be Constant, not FunctionCall
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(SeedHolder::NAMESPACE.as_bytes());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::Constant { .. }),
            "Method call on constant should be Constant, got {:?}",
            result
        );
    }

    #[test]
    fn test_function_call_mixed_args() {
        // func(&params.key_a, &authority).as_ref() - mixed data + ctx args
        let args = make_instruction_args(&["params"]);
        let expr: syn::Expr = parse_quote!(func(&params.key_a, &authority).as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        match result {
            ClassifiedSeed::FunctionCall {
                args: fn_args,
                has_as_ref,
                ..
            } => {
                assert!(has_as_ref);
                assert_eq!(fn_args.len(), 2);
                assert_eq!(fn_args[0].field_name.to_string(), "key_a");
                assert_eq!(fn_args[0].kind, FnArgKind::DataField);
                assert_eq!(fn_args[1].field_name.to_string(), "authority");
                assert_eq!(fn_args[1].kind, FnArgKind::CtxAccount);
            }
            other => panic!("Expected FunctionCall, got {:?}", other),
        }
    }

    #[test]
    fn test_literal_sliced() {
        // b"literal"[..] - byte literal with full range slice
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(b"literal"[..]);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::Literal(bytes) if bytes == b"literal"));
    }

    #[test]
    fn test_constant_qualified() {
        // crate::path::CONSTANT - qualified constant path
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(crate::state::SEED_CONSTANT);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::Constant { path, .. } if path.segments.last().unwrap().ident == "SEED_CONSTANT")
        );
    }

    #[test]
    fn test_ctx_account_nested() {
        // ctx.accounts.authority.key().as_ref() - nested ctx account access
        // The macro extracts the terminal field "authority" as the account root
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(ctx.accounts.authority.key().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::CtxRooted { account, .. } if account == "authority")
        );
    }

    #[test]
    fn test_parse_instruction_arg_names() {
        // Test that we can parse instruction attributes
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[instruction(owner: Pubkey)])];
        let args = parse_instruction_arg_names(&attrs).unwrap();
        assert!(args.contains("owner"));
    }

    #[test]
    fn test_parse_instruction_arg_names_multiple() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[instruction(owner: Pubkey, amount: u64, flag: bool)])];
        let args = parse_instruction_arg_names(&attrs).unwrap();
        assert!(args.contains("owner"));
        assert!(args.contains("amount"));
        assert!(args.contains("flag"));
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
    fn test_check_light_account_type_token_namespace() {
        // Test that token:: namespace is not detected as mint (it's neither PDA nor mint nor ATA)
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[light_account(token::authority = [b"auth"])]
        )];
        let (has_pda, has_mint, has_ata, has_zero_copy) = check_light_account_type(&attrs);
        assert!(!has_pda, "Should NOT be detected as PDA (no init)");
        assert!(!has_mint, "Should NOT be detected as mint");
        assert!(!has_ata, "Should NOT be detected as ATA");
        assert!(!has_zero_copy, "Should NOT be detected as zero_copy");
    }

    #[test]
    fn test_check_light_account_type_associated_token_init() {
        // Test that associated_token:: with init is detected as ATA
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[light_account(init,
                associated_token::authority = owner,
                associated_token::mint = mint
            )]
        )];
        let (has_pda, has_mint, has_ata, has_zero_copy) = check_light_account_type(&attrs);
        assert!(!has_pda, "Should NOT be detected as PDA");
        assert!(!has_mint, "Should NOT be detected as mint");
        assert!(has_ata, "Should be detected as ATA");
        assert!(!has_zero_copy, "Should NOT be detected as zero_copy");
    }

    #[test]
    fn test_check_light_account_type_token_init() {
        // Test that token:: with init is NOT detected as PDA
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[light_account(init,
                token::authority = [b"vault_auth"],
                token::mint = mint
            )]
        )];
        let (has_pda, has_mint, has_ata, has_zero_copy) = check_light_account_type(&attrs);
        assert!(!has_pda, "Should NOT be detected as PDA");
        assert!(!has_mint, "Should NOT be detected as mint");
        assert!(!has_ata, "Should NOT be detected as ATA");
        assert!(!has_zero_copy, "Should NOT be detected as zero_copy");
    }

    #[test]
    fn test_check_light_account_type_pda_zero_copy() {
        // Test that zero_copy with init is detected correctly
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[light_account(init, zero_copy)]
        )];
        let (has_pda, has_mint, has_ata, has_zero_copy) = check_light_account_type(&attrs);
        assert!(has_pda, "Should be detected as PDA");
        assert!(!has_mint, "Should NOT be detected as mint");
        assert!(!has_ata, "Should NOT be detected as ATA");
        assert!(has_zero_copy, "Should be detected as zero_copy");
    }
}
