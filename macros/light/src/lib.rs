extern crate proc_macro;
use accounts::process_light_accounts;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, ItemFn};
use traits::process_light_traits;
mod accounts;
mod pubkey;
mod traits;

/// Converts a base58 encoded public key into a byte array.
#[proc_macro]
pub fn pubkey(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as pubkey::PubkeyArgs);
    pubkey::pubkey(args)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn heap_neutral(_: TokenStream, input: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(input as ItemFn);

    // Insert memory management code at the beginning of the function
    let init_code: syn::Stmt = parse_quote! {
        #[cfg(target_os = "solana")]
        let pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();
    };
    let msg = format!("pre: {}", function.sig.ident);
    let log_pre: syn::Stmt = parse_quote! {
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        light_heap::GLOBAL_ALLOCATOR.log_total_heap(#msg);
    };
    function.block.stmts.insert(0, init_code);
    function.block.stmts.insert(1, log_pre);

    // Insert memory management code at the end of the function
    let msg = format!("post: {}", function.sig.ident);
    let log_post: syn::Stmt = parse_quote! {
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        light_heap::GLOBAL_ALLOCATOR.log_total_heap(#msg);
    };
    let cleanup_code: syn::Stmt = parse_quote! {
        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.free_heap(pos)?;
    };
    let len = function.block.stmts.len();
    function.block.stmts.insert(len - 1, log_post);
    function.block.stmts.insert(len - 1, cleanup_code);
    TokenStream::from(quote! { #function })
}

/// Adds required fields to your anchor instruction for applying a zk-compressed
/// state transition.
///
/// ## Usage
/// Add `#[light_accounts]` to your struct. Ensure it's applied before Anchor's
/// `#[derive(Accounts)]` and Light's `#[derive(LightTraits)]`.
///
/// ## Example
/// Note: You will have to build your program IDL using Anchor's `idl-build`
/// feature, otherwise your IDL won't include these accounts.
/// ```ignore
/// #[light_accounts]
/// #[derive(Accounts)]
/// pub struct ExampleInstruction<'info> {
///     pub my_program: Program<'info, MyProgram>,
/// }
/// ```
/// This will expand to add the following fields to your struct:
/// - `light_system_program`:           Verifies and applies zk-compression
///                                     state transitions.
/// - `registered_program_pda`:         A light protocol PDA to authenticate
///                                     state tree updates.
/// - `noop_program`:                   The SPL noop program to write
///                                     compressed-account state as calldata to
///                                     the Solana ledger.
/// - `account_compression_authority`:  The authority for account compression
///                                     operations.
/// - `account_compression_program`:    Called by light_system_program. Updates
///                                     state trees.
/// - `system_program`:                 The Solana System program.
#[proc_macro_attribute]
pub fn light_accounts(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match process_light_accounts(input) {
        Ok(token_stream) => token_stream.into(),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

/// Implements traits on the given struct required for invoking The Light system
/// program via CPI.
///
/// ## Usage
/// Add `#[derive(LightTraits)]` to your struct which specifies the accounts
/// required for your Anchor program instruction. Specify the attributes
/// `self_program`, `fee_payer`, `authority`, and optionally `cpi_context` to
/// the relevant fields.
///
/// ### Attributes
/// - `self_program`:   Marks the field that represents the program invoking the
///                     light system program, i.e. your program. You need to
///                     list your program as part of the struct.
/// - `fee_payer`:      Marks the field that represents the account responsible
///                     for paying transaction fees. (Signer)
///
/// - `authority`:      TODO: explain authority.
/// - `cpi_context`:    TODO: explain cpi_context.
///
/// ### Required accounts (must specify exact name).
///
/// - `light_system_program`:           Light systemprogram. verifies & applies
///                                     compression state transitions.
/// - `registered_program_pda`:         Light Systemprogram PDA
/// - `noop_program`:                   SPL noop program
/// - `account_compression_authority`:  TODO: explain.
/// - `account_compression_program`:    Account Compression program.
/// - `system_program`:                 The Solana Systemprogram.
///
/// ### Example
/// ```ignore
/// #[derive(Accounts, LightTraits)]
/// pub struct ExampleInstruction<'info> {
///     #[self_program]
///     pub my_program: Program<'info, MyProgram>,
///     #[fee_payer]
///     pub payer: Signer<'info>,
///     #[authority]
///     pub user: AccountInfo<'info>,
///     #[cpi_context]
///     pub cpi_context_account: Account<'info, CpiContextAccount>,
///     pub light_system_program: Program<'info, LightSystemProgram>,
///     pub registered_program_pda: Account<'info, RegisteredProgram>,
///     pub noop_program: AccountInfo<'info>,
///     pub account_compression_authority: AccountInfo<'info>,
///     pub account_compression_program: Program<'info, AccountCompression>,
///     pub system_program: Program<'info, System>,
/// }
/// ```
#[proc_macro_derive(
    LightTraits,
    attributes(self_program, fee_payer, authority, cpi_context)
)]
pub fn light_traits_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match process_light_traits(input) {
        Ok(token_stream) => token_stream.into(),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

// #[proc_macro_derive(
//     LightTraits,
//     attributes(self_program, fee_payer, authority, cpi_context)
// )]
// pub fn light_traits_derive(input: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(input as DeriveInput);
//     let name = &input.ident;

//     let trait_impls = match input.data {
//         Data::Struct(data_struct) => {
//             match data_struct.fields {
//                 Fields::Named(fields) => {
//                     let mut self_program_field = None;
//                     let mut fee_payer_field = None;
//                     let mut authority_field = None;
//                     let mut light_system_program_field = None;
//                     let mut cpi_context_account_field = None;

//                     // base impl
//                     let mut registered_program_pda_field = None;
//                     let mut noop_program_field = None;
//                     let mut account_compression_authority_field = None;
//                     let mut account_compression_program_field = None;
//                     let mut system_program_field = None;

//                     let compressed_sol_pda_field = fields
//                         .named
//                         .iter()
//                         .find_map(|f| {
//                             if f.ident
//                                 .as_ref()
//                                 .map(|id| id == "compressed_sol_pda")
//                                 .unwrap_or(false)
//                             {
//                                 Some(quote! { self.#f.ident.as_ref() })
//                             } else {
//                                 None
//                             }
//                         })
//                         .unwrap_or(quote! { None });

//                     let compression_recipient_field = fields
//                         .named
//                         .iter()
//                         .find_map(|f| {
//                             if f.ident
//                                 .as_ref()
//                                 .map(|id| id == "compression_recipient")
//                                 .unwrap_or(false)
//                             {
//                                 Some(quote! { self.#f.ident.as_ref() })
//                             } else {
//                                 None
//                             }
//                         })
//                         .unwrap_or(quote! { None });

//                     for f in fields.named.iter() {
//                         for attr in &f.attrs {
//                             if attr.path.is_ident("self_program") {
//                                 self_program_field = Some(f.ident.as_ref().unwrap());
//                             }
//                             if attr.path.is_ident("fee_payer") {
//                                 fee_payer_field = Some(f.ident.as_ref().unwrap());
//                             }
//                             if attr.path.is_ident("authority") {
//                                 authority_field = Some(f.ident.as_ref().unwrap());
//                             }
//                             if attr.path.is_ident("cpi_context") {
//                                 cpi_context_account_field = Some(f.ident.as_ref().unwrap());
//                             }
//                         }
//                         if f.ident
//                             .as_ref()
//                             .map(|id| id == "light_system_program")
//                             .unwrap_or(false)
//                         {
//                             light_system_program_field = Some(f.ident.as_ref().unwrap());
//                         }
//                         if f.ident
//                             .as_ref()
//                             .map(|id| id == "registered_program_pda")
//                             .unwrap_or(false)
//                         {
//                             registered_program_pda_field = Some(f.ident.as_ref().unwrap());
//                         }
//                         if f.ident
//                             .as_ref()
//                             .map(|id| id == "noop_program")
//                             .unwrap_or(false)
//                         {
//                             noop_program_field = Some(f.ident.as_ref().unwrap());
//                         }
//                         if f.ident
//                             .as_ref()
//                             .map(|id| id == "account_compression_authority")
//                             .unwrap_or(false)
//                         {
//                             account_compression_authority_field = Some(f.ident.as_ref().unwrap());
//                         }
//                         if f.ident
//                             .as_ref()
//                             .map(|id| id == "account_compression_program")
//                             .unwrap_or(false)
//                         {
//                             account_compression_program_field = Some(f.ident.as_ref().unwrap());
//                         }
//                         if f.ident
//                             .as_ref()
//                             .map(|id| id == "system_program")
//                             .unwrap_or(false)
//                         {
//                             system_program_field = Some(f.ident.as_ref().unwrap());
//                         }
//                     }

//                     // optional: compressed_sol_pda, compression_recipient,
//                     // cpi_context_account
//                     let missing_required_fields = [
//                         if light_system_program_field.is_none() {
//                             "light_system_program"
//                         } else {
//                             ""
//                         },
//                         if registered_program_pda_field.is_none() {
//                             "registered_program_pda"
//                         } else {
//                             ""
//                         },
//                         if noop_program_field.is_none() {
//                             "noop_program"
//                         } else {
//                             ""
//                         },
//                         if account_compression_authority_field.is_none() {
//                             "account_compression_authority"
//                         } else {
//                             ""
//                         },
//                         if account_compression_program_field.is_none() {
//                             "account_compression_program"
//                         } else {
//                             ""
//                         },
//                         if system_program_field.is_none() {
//                             "system_program"
//                         } else {
//                             ""
//                         },
//                     ]
//                     .iter()
//                     .filter(|&field| !field.is_empty())
//                     .cloned()
//                     .collect::<Vec<_>>();

//                     let missing_required_attributes = [
//                         if self_program_field.is_none() {
//                             "self_program"
//                         } else {
//                             ""
//                         },
//                         if fee_payer_field.is_none() {
//                             "fee_payer"
//                         } else {
//                             ""
//                         },
//                         if authority_field.is_none() {
//                             "authority"
//                         } else {
//                             ""
//                         },
//                     ]
//                     .iter()
//                     .filter(|&attr| !attr.is_empty())
//                     .cloned()
//                     .collect::<Vec<_>>();

//                     if !missing_required_fields.is_empty()
//                         || !missing_required_attributes.is_empty()
//                     {
//                         let error_message = format!(
//                             "Error: Missing required fields: [{}], Missing required attributes: [{}]",
//                             missing_required_fields.join(", "),
//                             missing_required_attributes.join(", ")
//                         );
//                         quote! {
//                             compile_error!(#error_message);
//                         }
//                     } else {
//                         let base_impls = quote! {
//                             impl<'info> InvokeCpiAccounts<'info> for #name<'info> {
//                                 fn get_invoking_program(&self) -> &AccountInfo<'info> {
//                                     &self.#self_program_field
//                                 }
//                             }
//                             impl<'info> SignerAccounts<'info> for #name<'info> {
//                                 fn get_fee_payer(&self) -> &Signer<'info> {
//                                     &self.#fee_payer_field
//                                 }
//                                 fn get_authority(&self) -> &AccountInfo<'info> {
//                                     &self.#authority_field
//                                 }
//                             }
//                             impl<'info> LightSystemAccount<'info> for #name<'info> {
//                                 fn get_light_system_program(&self) -> &Program<'info, LightSystemProgram> {
//                                     &self.#light_system_program_field
//                                 }
//                             }
//                         };
//                         let invoke_accounts_impl = quote! {
//                             impl<'info> InvokeAccounts<'info> for #name<'info> {
//                                 fn get_registered_program_pda(&self) -> &Account<'info, RegisteredProgram> {
//                                     &self.#registered_program_pda_field
//                                 }
//                                 fn get_noop_program(&self) -> &AccountInfo<'info> {
//                                     &self.#noop_program_field
//                                 }
//                                 fn get_account_compression_authority(&self) -> &AccountInfo<'info> {
//                                     &self.#account_compression_authority_field
//                                 }
//                                 fn get_account_compression_program(&self) -> &Program<'info, AccountCompression> {
//                                     &self.#account_compression_program_field
//                                 }
//                                 fn get_system_program(&self) -> &Program<'info, System> {
//                                     &self.#system_program_field
//                                 }
//                                 fn get_compressed_sol_pda(&self) -> Option<&UncheckedAccount<'info>> {
//                                     #compressed_sol_pda_field
//                                 }
//                                 fn get_compression_recipient(&self) -> Option<&UncheckedAccount<'info>> {
//                                     #compression_recipient_field
//                                 }
//                             }
//                         };
//                         if cpi_context_account_field.is_none() {
//                             quote! {
//                                 #base_impls
//                                 #invoke_accounts_impl
//                                 impl<'info> InvokeCpiContextAccount<'info> for #name<'info> {
//                                     fn get_cpi_context_account(&self) -> Option<&Account<'info, CpiContextAccount>> {
//                                         None
//                                     }
//                                 }
//                             }
//                         } else {
//                             quote! {
//                                 #base_impls
//                                 #invoke_accounts_impl
//                                 impl<'info> InvokeCpiContextAccount<'info> for #name<'info> {
//                                     fn get_cpi_context_account(&self) -> Option<&Account<'info, CpiContextAccount>> {
//                                         Some(&self.#cpi_context_account_field)
//                                     }
//                                 }
//                             }
//                         }
//                     }
//                 }
//                 _ => quote! {
//                     compile_error!("Error: Expected named fields but found unnamed or no fields.");
//                 },
//             }
//         }
//         _ => quote! {},
//     };

//     let expanded = quote! {
//         #trait_impls
//     };

//     TokenStream::from(expanded)
// }
