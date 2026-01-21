//! Anchor seed extraction from #[account(seeds = [...])] attributes.
//!
//! This module extracts PDA seeds from Anchor's attribute syntax and classifies them
//! into the categories needed for compression: literals, ctx fields, data fields, etc.

use std::collections::HashSet;

use syn::{Expr, Ident, ItemStruct, Type};

use crate::{
    light_pdas::shared_utils::{extract_terminal_ident, is_constant_identifier},
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

/// Classified seed element from Anchor's seeds array
#[derive(Clone, Debug)]
pub enum ClassifiedSeed {
    /// b"literal" or "string" - hardcoded bytes
    Literal(Vec<u8>),
    /// CONSTANT - uppercase identifier, resolved as crate::CONSTANT
    Constant(syn::Path),
    /// account.key().as_ref() - reference to account in struct
    CtxAccount(Ident),
    /// params.field.as_ref() or params.field.to_le_bytes().as_ref()
    DataField {
        field_name: Ident,
        /// Method like to_le_bytes, or None for direct .as_ref()
        conversion: Option<Ident>,
    },
    /// Function call like max_key(&a.key(), &b.key())
    FunctionCall {
        func: syn::Path,
        /// Account references used as arguments
        ctx_args: Vec<Ident>,
    },
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
}

/// All extracted info from an Accounts struct
#[derive(Clone, Debug)]
pub struct ExtractedAccountsInfo {
    pub struct_name: Ident,
    pub pda_fields: Vec<ExtractedSeedSpec>,
    pub token_fields: Vec<ExtractedTokenSpec>,
    /// True if struct has any #[light_account(init)] fields
    pub has_light_mint_fields: bool,
}

/// Extract rentfree field info from an Accounts struct
pub fn extract_from_accounts_struct(
    item: &ItemStruct,
    instruction_args: &InstructionArgSet,
) -> syn::Result<Option<ExtractedAccountsInfo>> {
    let fields = match &item.fields {
        syn::Fields::Named(named) => &named.named,
        _ => return Ok(None),
    };

    let mut pda_fields = Vec::new();
    let mut token_fields = Vec::new();
    let mut has_light_mint_fields = false;

    for field in fields {
        let field_ident = match &field.ident {
            Some(id) => id.clone(),
            None => continue,
        };

        // Check for #[light_account(...)] attribute and determine its type
        let (has_light_account_pda, has_light_account_mint) =
            check_light_account_type(&field.attrs);

        if has_light_account_mint {
            has_light_mint_fields = true;
        }

        // Check for #[light_account(token, ...)] attribute
        let token_attr = extract_light_token_attr(&field.attrs, instruction_args)?;

        if has_light_account_pda {
            // Extract inner type from Account<'info, T> or Box<Account<'info, T>>
            // Note: is_boxed is not needed for ExtractedSeedSpec, only inner_type
            let (_, inner_type) = match extract_account_inner_type(&field.ty) {
                Some(result) => result,
                None => {
                    return Err(syn::Error::new_spanned(
                        &field.ty,
                        "#[light_account(init)] requires Account<'info, T> or Box<Account<'info, T>>",
                    ));
                }
            };

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
            });
        }
    }

    // If no rentfree/light_mint fields found, return None
    if pda_fields.is_empty() && token_fields.is_empty() && !has_light_mint_fields {
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
    }))
}

/// Check #[light_account(...)] attributes for PDA or mint type.
/// Returns (has_pda, has_mint) indicating which type was detected.
fn check_light_account_type(attrs: &[syn::Attribute]) -> (bool, bool) {
    for attr in attrs {
        if attr.path().is_ident("light_account") {
            // Parse the content to determine if it's init-only (PDA) or init+mint (Mint)
            let tokens = match &attr.meta {
                syn::Meta::List(list) => list.tokens.clone(),
                _ => continue,
            };

            // Single pass to check for both "init" and "mint" keywords
            let mut has_mint = false;
            let mut has_init = false;
            for token in tokens {
                if let proc_macro2::TokenTree::Ident(ident) = token {
                    if ident == "mint" {
                        has_mint = true;
                    } else if ident == "init" {
                        has_init = true;
                    }
                }
            }

            if has_init {
                // If has mint, it's a mint field; otherwise it's a PDA
                return (!has_mint, has_mint);
            }
        }
    }
    (false, false)
}

/// Parsed #[light_account(token, ...)] attribute
struct LightTokenAttr {
    /// Optional variant name - if None, derived from field name
    variant_name: Option<Ident>,
    authority_seeds: Option<Vec<ClassifiedSeed>>,
}

/// Extract #[light_account(token, authority = [...])] attribute
/// Variant name is derived from field name, not specified in attribute
/// Returns Err if the attribute exists but has malformed syntax
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

            // Check if "token" keyword is present (without requiring "init")
            let has_token = tokens
                .clone()
                .into_iter()
                .any(|t| matches!(&t, proc_macro2::TokenTree::Ident(ident) if ident == "token"));

            if has_token {
                // Parse authority = [...] - propagate errors instead of swallowing them
                let parsed = parse_light_token_list(&tokens, instruction_args)?;
                return Ok(Some(parsed));
            }
        }
    }
    Ok(None)
}

/// Parse light_account(token, authority = [...]) content
fn parse_light_token_list(
    tokens: &proc_macro2::TokenStream,
    instruction_args: &InstructionArgSet,
) -> syn::Result<LightTokenAttr> {
    use syn::parse::Parser;

    // Capture instruction_args for use in closure
    let instruction_args = instruction_args.clone();
    let parser = move |input: syn::parse::ParseStream| -> syn::Result<LightTokenAttr> {
        let mut authority_seeds = None;

        // Parse comma-separated items looking for "token" and "authority = [...]"
        while !input.is_empty() {
            if input.peek(Ident) {
                let ident: Ident = input.parse()?;

                if ident == "token" {
                    // Skip the token keyword, continue parsing
                } else if ident == "authority" {
                    // Parse authority = [...]
                    input.parse::<syn::Token![=]>()?;
                    let array: syn::ExprArray = input.parse()?;
                    let mut seeds = Vec::new();
                    for elem in &array.elems {
                        if let Ok(seed) = classify_seed_expr(elem, &instruction_args) {
                            seeds.push(seed);
                        }
                    }
                    authority_seeds = Some(seeds);
                }
            }

            // Skip comma if present
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            } else if !input.is_empty() {
                // Skip unexpected tokens
                let _: proc_macro2::TokenTree = input.parse()?;
            }
        }

        Ok(LightTokenAttr {
            variant_name: None, // Variant name is always derived from field name
            authority_seeds,
        })
    };

    parser.parse2(tokens.clone())
}

/// Extract inner type T from Account<'info, T>, Box<Account<'info, T>>,
/// AccountLoader<'info, T>, or InterfaceAccount<'info, T>
///
/// Returns the full type path (e.g., `crate::module::MyRecord`) to preserve
/// module qualification for code generation.
pub fn extract_account_inner_type(ty: &Type) -> Option<(bool, Type)> {
    match ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last()?;
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
                                            return Some((false, inner_ty.clone()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None
                }
                "Box" => {
                    // Check for Box<Account<'info, T>>
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            // Check for nested Box<Box<...>> which is not supported
                            if let Type::Path(inner_path) = inner_ty {
                                if let Some(inner_seg) = inner_path.path.segments.last() {
                                    if inner_seg.ident == "Box" {
                                        // Nested Box detected - return None to signal unsupported type
                                        return None;
                                    }
                                }
                            }

                            if let Some((_, inner_type)) = extract_account_inner_type(inner_ty) {
                                return Some((true, inner_type));
                            }
                        }
                    }
                    None
                }
                _ => None,
            }
        }
        _ => None,
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

/// Classify a single seed expression
pub fn classify_seed_expr(
    expr: &Expr,
    instruction_args: &InstructionArgSet,
) -> syn::Result<ClassifiedSeed> {
    match expr {
        // b"literal"
        Expr::Lit(lit) => {
            if let syn::Lit::ByteStr(bs) = &lit.lit {
                return Ok(ClassifiedSeed::Literal(bs.value()));
            }
            if let syn::Lit::Str(s) = &lit.lit {
                return Ok(ClassifiedSeed::Literal(s.value().into_bytes()));
            }
            Err(syn::Error::new_spanned(
                expr,
                "Unsupported literal in seeds",
            ))
        }

        // CONSTANT (all uppercase path) or bare instruction arg
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                let name = ident.to_string();

                // Check uppercase constant first
                if is_constant_identifier(&name) {
                    return Ok(ClassifiedSeed::Constant(path.path.clone()));
                }

                // Check if this is a bare instruction arg (Format 2)
                // e.g., #[instruction(owner: Pubkey)] -> seeds = [owner.as_ref()]
                if instruction_args.contains(&name) {
                    return Ok(ClassifiedSeed::DataField {
                        field_name: ident.clone(),
                        conversion: None,
                    });
                }

                // Otherwise treat as ctx account reference
                return Ok(ClassifiedSeed::CtxAccount(ident.clone()));
            }
            // Multi-segment path is a constant
            Ok(ClassifiedSeed::Constant(path.path.clone()))
        }

        // method_call.as_ref() - most common case
        Expr::MethodCall(mc) => classify_method_call(mc, instruction_args),

        // Reference like &account.key()
        Expr::Reference(r) => classify_seed_expr(&r.expr, instruction_args),

        // Field access like params.owner or params.nested.owner - direct field reference
        Expr::Field(field) => {
            if let syn::Member::Named(field_name) = &field.member {
                // Check if root of the expression is an instruction arg
                if is_instruction_arg_rooted(&field.base, instruction_args) {
                    return Ok(ClassifiedSeed::DataField {
                        field_name: field_name.clone(),
                        conversion: None,
                    });
                }
                // ctx.field or account.field - treat as ctx account
                return Ok(ClassifiedSeed::CtxAccount(field_name.clone()));
            }
            Err(syn::Error::new_spanned(
                expr,
                "Unsupported field expression",
            ))
        }

        // Function call like max_key(&a.key(), &b.key()).as_ref()
        Expr::Call(call) => {
            let func = match &*call.func {
                Expr::Path(p) => p.path.clone(),
                _ => {
                    return Err(syn::Error::new_spanned(
                        expr,
                        "Expected path for function call",
                    ))
                }
            };

            let mut ctx_args = Vec::new();
            for arg in &call.args {
                if let Some(ident) = extract_terminal_ident(arg, true) {
                    ctx_args.push(ident);
                }
            }

            Ok(ClassifiedSeed::FunctionCall { func, ctx_args })
        }

        // Index expression - handles two cases:
        // 1. b"literal"[..] - converts [u8; N] to &[u8]
        // 2. params.arrays[2] - array indexing on instruction arg field
        Expr::Index(idx) => {
            // Case 1: Check if the index is a full range (..) on byte literal
            if let Expr::Range(range) = &*idx.index {
                if range.start.is_none() && range.end.is_none() {
                    // This is a full range [..], now check if expr is a byte string literal
                    if let Expr::Lit(lit) = &*idx.expr {
                        if let syn::Lit::ByteStr(bs) = &lit.lit {
                            return Ok(ClassifiedSeed::Literal(bs.value()));
                        }
                    }
                }
            }

            // Case 2: Array indexing on instruction arg field like params.arrays[2]
            if is_instruction_arg_rooted(&idx.expr, instruction_args) {
                if let Some(field_name) = extract_terminal_field(&idx.expr) {
                    return Ok(ClassifiedSeed::DataField {
                        field_name,
                        conversion: None,
                    });
                }
            }

            Err(syn::Error::new_spanned(
                expr,
                format!("Unsupported index expression in seeds: {:?}", expr),
            ))
        }

        _ => Err(syn::Error::new_spanned(
            expr,
            format!("Unsupported seed expression: {:?}", expr),
        )),
    }
}

/// Classify a method call expression like account.key().as_ref()
fn classify_method_call(
    mc: &syn::ExprMethodCall,
    instruction_args: &InstructionArgSet,
) -> syn::Result<ClassifiedSeed> {
    // Unwrap .as_ref(), .as_bytes(), or .as_slice() at the end - these are terminal conversions
    if mc.method == "as_ref" || mc.method == "as_bytes" || mc.method == "as_slice" {
        return classify_seed_expr(&mc.receiver, instruction_args);
    }

    // Handle instruction_arg.field.to_le_bytes() or instruction_arg.nested.field.to_le_bytes()
    // Also handle bare instruction arg: amount.to_le_bytes() where amount is a direct instruction arg
    if mc.method == "to_le_bytes" || mc.method == "to_be_bytes" {
        // Check for bare instruction arg like amount.to_le_bytes()
        if let Expr::Path(path) = &*mc.receiver {
            if let Some(ident) = path.path.get_ident() {
                if instruction_args.contains(&ident.to_string()) {
                    return Ok(ClassifiedSeed::DataField {
                        field_name: ident.clone(),
                        conversion: Some(mc.method.clone()),
                    });
                }
            }
        }

        // Check for field access on instruction arg
        if is_instruction_arg_rooted(&mc.receiver, instruction_args) {
            if let Some(field_name) = extract_terminal_field(&mc.receiver) {
                return Ok(ClassifiedSeed::DataField {
                    field_name,
                    conversion: Some(mc.method.clone()),
                });
            }
        }
    }

    // Handle account.key()
    if mc.method == "key" {
        if let Some(ident) = extract_terminal_ident(&mc.receiver, false) {
            // Check if it's rooted in an instruction arg
            if is_instruction_arg_rooted(&mc.receiver, instruction_args) {
                if let Some(field_name) = extract_terminal_field(&mc.receiver) {
                    return Ok(ClassifiedSeed::DataField {
                        field_name,
                        conversion: None,
                    });
                }
            }
            return Ok(ClassifiedSeed::CtxAccount(ident));
        }
    }

    // instruction_arg.field or instruction_arg.nested.field - check for instruction-arg-rooted access
    if is_instruction_arg_rooted(&mc.receiver, instruction_args) {
        if let Some(field_name) = extract_terminal_field(&mc.receiver) {
            return Ok(ClassifiedSeed::DataField {
                field_name,
                conversion: None,
            });
        }
    }

    Err(syn::Error::new_spanned(
        mc,
        "Unsupported method call in seeds",
    ))
}

/// Check if an expression is rooted in an instruction argument.
/// Works with ANY instruction arg name, not just "params".
fn is_instruction_arg_rooted(expr: &Expr, instruction_args: &InstructionArgSet) -> bool {
    match expr {
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                instruction_args.contains(&ident.to_string())
            } else {
                false
            }
        }
        Expr::Field(field) => {
            // Recursively check the base
            is_instruction_arg_rooted(&field.base, instruction_args)
        }
        Expr::Index(idx) => {
            // For array indexing like params.arrays[2], check the base
            is_instruction_arg_rooted(&idx.expr, instruction_args)
        }
        _ => false,
    }
}

/// Extract the terminal field name from a nested field access (e.g., params.nested.owner -> owner)
fn extract_terminal_field(expr: &Expr) -> Option<Ident> {
    match expr {
        Expr::Field(field) => {
            if let syn::Member::Named(field_name) = &field.member {
                Some(field_name.clone())
            } else {
                None
            }
        }
        Expr::Index(idx) => {
            // For indexed access, get the field name from the base
            extract_terminal_field(&idx.expr)
        }
        _ => None,
    }
}

/// Get data field names from classified seeds
pub fn get_data_fields(seeds: &[ClassifiedSeed]) -> Vec<(Ident, Option<Ident>)> {
    let mut fields = Vec::new();
    for seed in seeds {
        if let ClassifiedSeed::DataField {
            field_name,
            conversion,
        } = seed
        {
            if !fields.iter().any(|(f, _): &(Ident, _)| f == field_name) {
                fields.push((field_name.clone(), conversion.clone()));
            }
        }
    }
    fields
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
            if let Some((field_name, has_conversion)) = extract_data_field_from_expr(expr) {
                let field_str = field_name.to_string();
                // Only include fields that are NOT on the state struct and not already added
                if !state_field_names.contains(&field_str)
                    && !fields
                        .iter()
                        .any(|(f, _, _): &(Ident, _, _)| f == &field_name)
                {
                    let field_type: syn::Type = if has_conversion {
                        syn::parse_quote!(u64)
                    } else {
                        syn::parse_quote!(solana_pubkey::Pubkey)
                    };
                    fields.push((field_name, field_type, has_conversion));
                }
            }
        }
    }
    fields
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
        let args = make_instruction_args(&["owner", "amount"]);
        let expr: syn::Expr = parse_quote!(owner);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::DataField { field_name, .. } if field_name == "owner")
        );
    }

    #[test]
    fn test_bare_primitive_with_to_le_bytes() {
        let args = make_instruction_args(&["amount"]);
        let expr: syn::Expr = parse_quote!(amount.to_le_bytes().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(
            result,
            ClassifiedSeed::DataField {
                field_name,
                conversion: Some(conv)
            } if field_name == "amount" && conv == "to_le_bytes"
        ));
    }

    #[test]
    fn test_custom_struct_param_name() {
        let args = make_instruction_args(&["input"]);
        let expr: syn::Expr = parse_quote!(input.owner.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::DataField { field_name, .. } if field_name == "owner")
        );
    }

    #[test]
    fn test_nested_field_access() {
        let args = make_instruction_args(&["data"]);
        let expr: syn::Expr = parse_quote!(data.inner.key.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::DataField { field_name, .. } if field_name == "key")
        );
    }

    #[test]
    fn test_context_account_not_confused_with_arg() {
        let args = make_instruction_args(&["owner"]); // "authority" is NOT an arg
        let expr: syn::Expr = parse_quote!(authority.key().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(
            result,
            ClassifiedSeed::CtxAccount(ident) if ident == "authority"
        ));
    }

    #[test]
    fn test_empty_instruction_args() {
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(owner);
        let result = classify_seed_expr(&expr, &args).unwrap();
        // Without instruction args, bare ident treated as ctx account
        assert!(matches!(result, ClassifiedSeed::CtxAccount(_)));
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
        assert!(matches!(result, ClassifiedSeed::Constant(_)));
    }

    #[test]
    fn test_standard_params_field_access() {
        // Traditional format: #[instruction(params: CreateParams)]
        let args = make_instruction_args(&["params"]);
        let expr: syn::Expr = parse_quote!(params.owner.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::DataField { field_name, .. } if field_name == "owner")
        );
    }

    #[test]
    fn test_args_naming_format() {
        // Alternative naming: #[instruction(args: MyArgs)]
        let args = make_instruction_args(&["args"]);
        let expr: syn::Expr = parse_quote!(args.key.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::DataField { field_name, .. } if field_name == "key")
        );
    }

    #[test]
    fn test_data_naming_format() {
        // Alternative naming: #[instruction(data: DataInput)]
        let args = make_instruction_args(&["data"]);
        let expr: syn::Expr = parse_quote!(data.value.to_le_bytes().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(
            result,
            ClassifiedSeed::DataField {
                field_name,
                conversion: Some(conv)
            } if field_name == "value" && conv == "to_le_bytes"
        ));
    }

    #[test]
    fn test_format2_multiple_params() {
        // Format 2: #[instruction(owner: Pubkey, amount: u64)]
        let args = make_instruction_args(&["owner", "amount"]);

        let expr1: syn::Expr = parse_quote!(owner.as_ref());
        let result1 = classify_seed_expr(&expr1, &args).unwrap();
        assert!(
            matches!(result1, ClassifiedSeed::DataField { field_name, .. } if field_name == "owner")
        );

        let expr2: syn::Expr = parse_quote!(amount.to_le_bytes().as_ref());
        let result2 = classify_seed_expr(&expr2, &args).unwrap();
        assert!(matches!(
            result2,
            ClassifiedSeed::DataField {
                field_name,
                conversion: Some(_)
            } if field_name == "amount"
        ));
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
}
