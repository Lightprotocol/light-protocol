//! Anchor seed extraction from #[account(seeds = [...])] attributes.
//!
//! This module extracts PDA seeds from Anchor's attribute syntax and classifies them
//! into the categories needed for compression: literals, ctx fields, data fields, etc.

use syn::{Expr, Ident, ItemStruct, Type};

use crate::{
    rentfree::shared_utils::{extract_terminal_ident, is_constant_identifier},
    utils::snake_to_camel_case,
};

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
    pub variant_name: Ident,
    /// The inner type (e.g., UserRecord from Account<'info, UserRecord>)
    pub inner_type: Ident,
    /// Classified seeds from #[account(seeds = [...])]
    pub seeds: Vec<ClassifiedSeed>,
}

/// Extracted token specification for a #[rentfree_token = Variant] field
#[derive(Clone, Debug)]
pub struct ExtractedTokenSpec {
    /// The field name in the Accounts struct
    pub field_name: Ident,
    /// The variant name from #[rentfree_token = Variant]
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
}

/// Extract rentfree field info from an Accounts struct
pub fn extract_from_accounts_struct(
    item: &ItemStruct,
) -> syn::Result<Option<ExtractedAccountsInfo>> {
    let fields = match &item.fields {
        syn::Fields::Named(named) => &named.named,
        _ => return Ok(None),
    };

    let mut pda_fields = Vec::new();
    let mut token_fields = Vec::new();

    for field in fields {
        let field_ident = match &field.ident {
            Some(id) => id.clone(),
            None => continue,
        };

        // Check for #[rentfree] attribute
        let has_rentfree = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("rentfree"));

        // Check for #[rentfree_token(...)] attribute
        let token_attr = extract_rentfree_token_attr(&field.attrs);

        if has_rentfree {
            // Extract inner type from Account<'info, T> or Box<Account<'info, T>>
            // Note: is_boxed is not needed for ExtractedSeedSpec, only inner_type
            let (_, inner_type) = match extract_account_inner_type(&field.ty) {
                Some(result) => result,
                None => {
                    return Err(syn::Error::new_spanned(
                        &field.ty,
                        "#[rentfree] requires Account<'info, T> or Box<Account<'info, T>>",
                    ));
                }
            };

            // Extract seeds from #[account(seeds = [...])]
            let seeds = extract_anchor_seeds(&field.attrs)?;

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
            let seeds = extract_anchor_seeds(&field.attrs)?;

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

    // If no rentfree fields found, return None
    if pda_fields.is_empty() && token_fields.is_empty() {
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
                    if let Ok(auth_seeds) = extract_anchor_seeds(&auth_field_info.attrs) {
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
    }))
}

/// Parsed #[rentfree_token(...)] attribute
struct RentFreeTokenAttr {
    /// Optional variant name - if None, derived from field name
    variant_name: Option<Ident>,
    authority_seeds: Option<Vec<ClassifiedSeed>>,
}

/// Extract #[rentfree_token(authority = [...])] attribute
/// Variant name is now derived from field name, not specified in attribute
fn extract_rentfree_token_attr(attrs: &[syn::Attribute]) -> Option<RentFreeTokenAttr> {
    for attr in attrs {
        if attr.path().is_ident("rentfree_token") {
            match &attr.meta {
                // #[rentfree_token = Variant] (deprecated but still supported)
                syn::Meta::NameValue(nv) => {
                    if let Expr::Path(path) = &nv.value {
                        if let Some(ident) = path.path.get_ident() {
                            return Some(RentFreeTokenAttr {
                                variant_name: Some(ident.clone()),
                                authority_seeds: None,
                            });
                        }
                    }
                }
                // #[rentfree_token(authority = [...])] or #[rentfree_token(Variant, authority = [...])]
                syn::Meta::List(list) => {
                    if let Ok(parsed) = parse_rentfree_token_list(&list.tokens) {
                        return Some(parsed);
                    }
                    // Fallback: try parsing as just an identifier (deprecated)
                    if let Ok(ident) = syn::parse2::<Ident>(list.tokens.clone()) {
                        return Some(RentFreeTokenAttr {
                            variant_name: Some(ident),
                            authority_seeds: None,
                        });
                    }
                }
                // #[rentfree_token] with no arguments
                syn::Meta::Path(_) => {
                    return Some(RentFreeTokenAttr {
                        variant_name: None,
                        authority_seeds: None,
                    });
                }
            }
        }
    }
    None
}

/// Parse rentfree_token(authority = [...]) or rentfree_token(Variant, authority = [...]) content
fn parse_rentfree_token_list(tokens: &proc_macro2::TokenStream) -> syn::Result<RentFreeTokenAttr> {
    use syn::parse::Parser;

    let parser = |input: syn::parse::ParseStream| -> syn::Result<RentFreeTokenAttr> {
        let mut variant_name = None;
        let mut authority_seeds = None;

        // Check if first token is authority = [...] or a variant name
        if input.peek(Ident) {
            let ident: Ident = input.parse()?;

            if ident == "authority" {
                // First token is authority, parse the seeds
                input.parse::<syn::Token![=]>()?;
                let array: syn::ExprArray = input.parse()?;
                let mut seeds = Vec::new();
                for elem in &array.elems {
                    if let Ok(seed) = classify_seed_expr(elem) {
                        seeds.push(seed);
                    }
                }
                authority_seeds = Some(seeds);
            } else {
                // First token is variant name (deprecated but supported)
                variant_name = Some(ident);

                // Check for comma and additional args
                while input.peek(syn::Token![,]) {
                    input.parse::<syn::Token![,]>()?;

                    // Look for authority = [...]
                    if input.peek(Ident) {
                        let key: Ident = input.parse()?;
                        if key == "authority" {
                            input.parse::<syn::Token![=]>()?;
                            let array: syn::ExprArray = input.parse()?;
                            let mut seeds = Vec::new();
                            for elem in &array.elems {
                                if let Ok(seed) = classify_seed_expr(elem) {
                                    seeds.push(seed);
                                }
                            }
                            authority_seeds = Some(seeds);
                        }
                    }
                }
            }
        }

        Ok(RentFreeTokenAttr {
            variant_name,
            authority_seeds,
        })
    };

    parser.parse2(tokens.clone())
}

/// Extract inner type T from Account<'info, T>, Box<Account<'info, T>>,
/// AccountLoader<'info, T>, or InterfaceAccount<'info, T>
pub fn extract_account_inner_type(ty: &Type) -> Option<(bool, Ident)> {
    match ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last()?;
            let ident_str = segment.ident.to_string();

            match ident_str.as_str() {
                "Account" | "AccountLoader" | "InterfaceAccount" => {
                    // Extract T from Account<'info, T>
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(Type::Path(inner_path)) = arg {
                                if let Some(inner_seg) = inner_path.path.segments.last() {
                                    // Skip lifetime 'info
                                    if inner_seg.ident != "info" {
                                        return Some((false, inner_seg.ident.clone()));
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
pub fn extract_anchor_seeds(attrs: &[syn::Attribute]) -> syn::Result<Vec<ClassifiedSeed>> {
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
                    return classify_seeds_array(&item.value);
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
fn classify_seeds_array(expr: &Expr) -> syn::Result<Vec<ClassifiedSeed>> {
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
        seeds.push(classify_seed_expr(elem)?);
    }

    Ok(seeds)
}

/// Classify a single seed expression
pub fn classify_seed_expr(expr: &Expr) -> syn::Result<ClassifiedSeed> {
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

        // CONSTANT (all uppercase path)
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                if is_constant_identifier(&ident.to_string()) {
                    return Ok(ClassifiedSeed::Constant(path.path.clone()));
                }
                // Otherwise it's a variable reference - treat as ctx account
                return Ok(ClassifiedSeed::CtxAccount(ident.clone()));
            }
            // Multi-segment path is a constant
            Ok(ClassifiedSeed::Constant(path.path.clone()))
        }

        // method_call.as_ref() - most common case
        Expr::MethodCall(mc) => classify_method_call(mc),

        // Reference like &account.key()
        Expr::Reference(r) => classify_seed_expr(&r.expr),

        // Field access like params.owner - direct field reference
        Expr::Field(field) => {
            if let syn::Member::Named(field_name) = &field.member {
                if let Expr::Path(path) = &*field.base {
                    if let Some(base_ident) = path.path.get_ident() {
                        if base_ident == "params" {
                            return Ok(ClassifiedSeed::DataField {
                                field_name: field_name.clone(),
                                conversion: None,
                            });
                        }
                    }
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

        _ => Err(syn::Error::new_spanned(
            expr,
            format!("Unsupported seed expression: {:?}", expr),
        )),
    }
}

/// Classify a method call expression like account.key().as_ref()
fn classify_method_call(mc: &syn::ExprMethodCall) -> syn::Result<ClassifiedSeed> {
    // Unwrap .as_ref() at the end
    if mc.method == "as_ref" {
        return classify_seed_expr(&mc.receiver);
    }

    // Handle params.field.to_le_bytes() directly
    if mc.method == "to_le_bytes" || mc.method == "to_be_bytes" {
        if let Some((field_name, base)) = extract_params_field(&mc.receiver) {
            if base == "params" {
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
            // Check if it's params.field or ctx.account
            if let Expr::Field(field) = &*mc.receiver {
                if let Expr::Path(path) = &*field.base {
                    if let Some(base_ident) = path.path.get_ident() {
                        if base_ident == "params" {
                            if let syn::Member::Named(field_name) = &field.member {
                                return Ok(ClassifiedSeed::DataField {
                                    field_name: field_name.clone(),
                                    conversion: None,
                                });
                            }
                        }
                    }
                }
            }
            return Ok(ClassifiedSeed::CtxAccount(ident));
        }
    }

    // params.field.as_ref() directly
    if let Some((field_name, base)) = extract_params_field(&mc.receiver) {
        if base == "params" {
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

/// Extract field name from params.field or similar
fn extract_params_field(expr: &Expr) -> Option<(Ident, String)> {
    if let Expr::Field(field) = expr {
        if let syn::Member::Named(field_name) = &field.member {
            if let Expr::Path(path) = &*field.base {
                if let Some(base_ident) = path.path.get_ident() {
                    return Some((field_name.clone(), base_ident.to_string()));
                }
            }
        }
    }
    None
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
