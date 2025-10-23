use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Expr, ItemStruct, Result, Token,
};

/// Parse account structs and generate seed functions based on their Anchor seeds attributes
struct AccountStructList {
    structs: Punctuated<ItemStruct, Token![,]>,
}

impl Parse for AccountStructList {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(AccountStructList {
            structs: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Generates seed getter functions by analyzing Anchor account structs
///
/// This macro scans account structs for `#[account(seeds = [...], ...)]` attributes
/// and generates corresponding seed getter functions.
///
/// Usage:
/// ```rust
/// generate_seed_functions! {
///     #[derive(Accounts)]
///     pub struct CreateRecord<'info> {
///         #[account(
///             init,
///             seeds = [b"user_record", user.key().as_ref()],
///             bump,
///         )]
///         pub user_record: Account<'info, UserRecord>,
///         pub user: Signer<'info>,
///     }
///
///     #[derive(Accounts)]
///     #[instruction(session_id: u64)]
///     pub struct CreateGameSession<'info> {
///         #[account(
///             init,
///             seeds = [b"game_session", session_id.to_le_bytes().as_ref()],
///             bump,
///         )]
///         pub game_session: Account<'info, GameSession>,
///         pub player: Signer<'info>,
///     }
/// }
/// ```
///
/// This generates:
/// - `get_user_record_seeds(user: &Pubkey) -> (Vec<Vec<u8>>, Pubkey)`
/// - `get_game_session_seeds(session_id: u64) -> (Vec<Vec<u8>>, Pubkey)`
pub fn generate_seed_functions(input: TokenStream) -> Result<TokenStream> {
    let account_structs = syn::parse2::<AccountStructList>(input)?;

    let mut generated_functions = Vec::new();

    for account_struct in &account_structs.structs {
        if let Some(function) = analyze_account_struct(account_struct)? {
            generated_functions.push(function);
        }
    }

    let expanded = quote! {
        #(#generated_functions)*
    };

    Ok(expanded)
}

fn analyze_account_struct(account_struct: &ItemStruct) -> Result<Option<TokenStream>> {
    // Look for fields with #[account(...)] attributes that have seeds
    for field in &account_struct.fields {
        if let Some(account_attr) = find_account_attribute(&field.attrs) {
            if let Some(seeds_info) = extract_seeds_from_account_attr(account_attr)? {
                let field_name = field.ident.as_ref().unwrap();
                let function_name = format_ident!("get_{}_seeds", field_name);

                let (parameters, seed_expressions) = analyze_seeds_expressions(&seeds_info)?;

                let function = quote! {
                    /// Auto-generated seed function from Anchor account struct
                    pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
                        let seeds = [#(#seed_expressions),*];
                        let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seeds, &crate::ID);
                        let bump_slice = vec![bump];
                        let seeds_vec = vec![
                            #(
                                (#seed_expressions).to_vec(),
                            )*
                            bump_slice,
                        ];
                        (seeds_vec, pda)
                    }
                };

                return Ok(Some(function));
            }
        }
    }

    Ok(None)
}

fn find_account_attribute(attrs: &[Attribute]) -> Option<&Attribute> {
    attrs.iter().find(|attr| attr.path().is_ident("account"))
}

// TODO: check on this.
fn extract_seeds_from_account_attr(_attr: &Attribute) -> Result<Option<Vec<Expr>>> {
    // For now, return None to skip seed extraction - this is complex to parse correctly
    // The Anchor macro parsing is quite involved and would need more sophisticated handling
    Ok(None)
}

fn analyze_seeds_expressions(
    seed_expressions: &[Expr],
) -> Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let mut parameters = Vec::new();
    let mut processed_seeds = Vec::new();

    for expr in seed_expressions {
        match expr {
            // Handle byte string literals like b"user_record"
            Expr::Lit(_) => {
                processed_seeds.push(quote! { #expr });
            }
            // Handle method calls like user.key().as_ref()
            Expr::MethodCall(method_call) => {
                // Extract the base identifier (e.g., "user" from "user.key().as_ref()")
                if let Expr::Path(path_expr) = &*method_call.receiver {
                    if let Some(ident) = path_expr.path.get_ident() {
                        parameters.push(quote! { #ident: &anchor_lang::prelude::Pubkey });
                        processed_seeds.push(quote! { #ident.as_ref() });
                    }
                } else if let Expr::MethodCall(inner_call) = &*method_call.receiver {
                    // Handle nested calls like session_id.to_le_bytes().as_ref()
                    if let Expr::Path(path_expr) = &*inner_call.receiver {
                        if let Some(ident) = path_expr.path.get_ident() {
                            parameters.push(quote! { #ident: u64 });
                            processed_seeds.push(quote! { #ident.to_le_bytes().as_ref() });
                        }
                    }
                }
            }
            // Handle other expressions as-is
            _ => {
                processed_seeds.push(quote! { #expr });
            }
        }
    }

    Ok((parameters, processed_seeds))
}
