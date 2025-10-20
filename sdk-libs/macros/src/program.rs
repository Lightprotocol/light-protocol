use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse_quote, visit_mut::VisitMut, Attribute, FnArg, GenericArgument, Ident, Item, ItemFn,
    ItemMod, ItemStruct, Pat, PathArguments, Result, Stmt, Token, Type,
};

// A single instruction parameter provided as an argument to the Anchor program
// function. It consists of the name an the type, e.g.: `name: String`.
#[derive(Clone)]
struct InstructionParam {
    name: Ident,
    ty: Type,
}

impl ToTokens for InstructionParam {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        Token![:](self.name.span()).to_tokens(tokens);
        self.ty.to_tokens(tokens);
    }
}

/// Map which stores instruction parameters for all instructions in the parsed
/// program.
///
/// # Example
///
/// For the program with the following instructions:
///
/// ```ignore
/// #[light_program]
/// pub mode my_program {
///     use super::*;
///
///     pub fn instruction_one(
///         ctx: LightContext<'_, '_, '_, 'info, InstructionOne<'info>>,
///         name: String,
///         num: u32,
///     ) -> Result<()> {}
///
///     pub fn instruction_two(
///         ctx: LightContext<'_, '_, '_, 'info, InstructionTwo<'info>>,
///         num_one: u32,
///         num_two: u64,
///     ) -> Result<()> {}
/// }
/// ```
///
/// The mapping is going to look like:
///
/// ```ignore
/// instruction_one -> - name: name
///                      ty: String
///                    - name: num
///                      ty: u32
///
/// instruction_two -> - name: num_one
///                      ty: u32
///                    - name: num_two
///                      ty: u64
/// ```
#[derive(Default)]
struct InstructionParams(HashMap<String, Vec<InstructionParam>>);

impl Deref for InstructionParams {
    type Target = HashMap<String, Vec<InstructionParam>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for InstructionParams {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Implementation of `ToTokens` which allows to convert the
/// instruction-parameter mapping to structs, which we later use for packing
/// of parameters for convenient usage in `LightContext` extensions produced in
/// `accounts.rs` - precisely, in the `check_constraints` and
/// `derive_address_seeds` methods.
impl ToTokens for InstructionParams {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (name, inputs) in self.0.iter() {
            let name = Ident::new(name, Span::call_site());
            let strct: ItemStruct = parse_quote! {
                pub struct #name {
                    #(#inputs),*
                }
            };
            strct.to_tokens(tokens);
        }
    }
}

#[derive(Default)]
struct LightProgramTransform {
    /// Mapping of instructions to their parameters in the program.
    instruction_params: InstructionParams,
}

impl VisitMut for LightProgramTransform {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        // Add `#[allow(clippy::too_many_arguments)]` attribute in case. We are
        // injecting many arguments in this macro and they can easily go over
        // the limit.
        let clippy_attr: Attribute = parse_quote! {
            #[allow(clippy::too_many_arguments)]
        };
        i.attrs.push(clippy_attr);

        // Gather names instruction parameters (arguments other than `ctx`).
        // They are going to be used to generate `Inputs*` structs.
        let mut instruction_params = Vec::with_capacity(i.sig.inputs.len() - 1);
        for input in i.sig.inputs.iter().skip(1) {
            if let FnArg::Typed(input) = input {
                if let Pat::Ident(ref pat_ident) = *input.pat {
                    instruction_params.push(InstructionParam {
                        name: pat_ident.ident.clone(),
                        ty: (*input.ty).clone(),
                    });
                }
            }
        }

        // Find the `ctx` argument.
        let ctx_arg = i.sig.inputs.first_mut().unwrap();

        // Retrieve the type of `ctx`.
        let pat_type = match ctx_arg {
            FnArg::Typed(pat_type) => pat_type,
            _ => return,
        };

        // Get the last path segment of `ctx` type.
        let type_path = match pat_type.ty.as_mut() {
            Type::Path(type_path) => type_path,
            _ => return,
        };
        let ctx_segment = &mut type_path.path.segments.last_mut().unwrap();
        // If the `ctx` is of type `LightContext`, that means that the given
        // instruction uses compressed accounts and we need to inject our code
        // for handling them.
        // Otherwise, stop processing the instruction and assume it's just a
        // regular instruction using only regular accounts.
        if ctx_segment.ident != "LightContext" {
            return;
        }

        // Swap the type of `ctx` to Anchor's `Context` to keep the instruction
        // signature correct. We are going to inject the code converting it to
        // `LightContext` later.
        ctx_segment.ident = Ident::new("Context", Span::call_site());

        // Figure out what's are the names of:
        //
        // - The struct with Anchor accounts (implementing `anchor_lang::Accounts`) -
        //   it's specified as the last generic argument in `ctx`, e.g. `MyInstruction`.
        // - The struct with compressed accounts (implementing `LightAccounts`) -
        //   it's derived by adding the `Light` prefix to the previous struct name,
        //   e.g. `LightMyInstruction`.
        let arguments = match &ctx_segment.arguments {
            PathArguments::AngleBracketed(arguments) => arguments,
            _ => return,
        };
        let last_arg = arguments.args.last().unwrap();
        let last_arg_type = match last_arg {
            GenericArgument::Type(last_arg_type) => last_arg_type,
            _ => return,
        };
        let last_arg_type_path = match last_arg_type {
            Type::Path(last_arg_type_path) => last_arg_type_path,
            _ => return,
        };
        let accounts_segment = &last_arg_type_path.path.segments.last().unwrap();
        let accounts_ident = accounts_segment.ident.clone();
        let light_accounts_name = format!("Light{}", accounts_segment.ident);
        let light_accounts_ident = Ident::new(&light_accounts_name, Span::call_site());

        // Add the previously gathered instruction inputs to the mapping of
        // instructions to their parameters (`self.instruction_inputs`).
        let params_name = format!("Params{}", accounts_segment.ident);
        self.instruction_params
            .insert(params_name.clone(), instruction_params.clone());
        let inputs_ident = Ident::new(&params_name, Span::call_site());

        // Inject an `inputs: Vec<Vec<u8>>` argument to all instructions. The
        // purpose of that additional argument is passing compressed accounts.
        let inputs_arg: FnArg = parse_quote! { inputs: Vec<Vec<u8>> };
        i.sig.inputs.insert(1, inputs_arg);

        // Inject Merkle context related arguments.
        let proof_arg: FnArg = parse_quote! { proof: ::light_sdk::proof::CompressedProof };
        i.sig.inputs.insert(2, proof_arg);
        let merkle_context_arg: FnArg =
            parse_quote! { merkle_context: ::light_sdk::merkle_context::PackedMerkleContext };
        i.sig.inputs.insert(3, merkle_context_arg);
        let merkle_tree_root_index_arg: FnArg = parse_quote! { merkle_tree_root_index: u16 };
        i.sig.inputs.insert(4, merkle_tree_root_index_arg);
        let address_merkle_context_arg: FnArg =
            parse_quote! { address_merkle_context: ::light_sdk::tree_info::PackedAddressTreeInfo };
        i.sig.inputs.insert(5, address_merkle_context_arg);
        let address_merkle_tree_root_index_arg: FnArg =
            parse_quote! { address_merkle_tree_root_index: u16 };
        i.sig.inputs.insert(6, address_merkle_tree_root_index_arg);

        // Inject a `LightContext` into the function body.
        let light_context_stmt: Stmt = parse_quote! {
            let mut ctx: ::light_sdk::context::LightContext<
                #accounts_ident,
                #light_accounts_ident
            > = ::light_sdk::context::LightContext::new(
                ctx,
                inputs,
                merkle_context,
                merkle_tree_root_index,
                address_merkle_context,
                address_merkle_tree_root_index,
            )?;
        };
        i.block.stmts.insert(0, light_context_stmt);

        // Pack all instruction inputs in a struct, which then can be used in
        // `check_constrants` and `derive_address_seeds`.
        //
        // We do that, because passing one reference to these methods is more
        // comfortable. Passing references to each input separately would
        // require even messier code...
        //
        // We move the inputs to that struct, so no copies are being made.
        let input_idents = instruction_params
            .iter()
            .map(|input| input.name.clone())
            .collect::<Vec<Ident>>();
        let inputs_pack_stmt: Stmt = parse_quote! {
            let inputs = #inputs_ident { #(#input_idents),* };
        };
        i.block.stmts.insert(1, inputs_pack_stmt);

        // Inject `check_constraints` and `derive_address_seeds` calls right
        // after.
        let check_constraints_stmt: Stmt = parse_quote! {
            ctx.check_constraints(&inputs)?;
        };
        i.block.stmts.insert(2, check_constraints_stmt);
        let derive_address_seed_stmt: Stmt = parse_quote! {
            ctx.derive_address_seeds(address_merkle_context, &inputs);
        };
        i.block.stmts.insert(3, derive_address_seed_stmt);

        // Once we are done with calling `check_constraints` and
        // `derive_address_seeds`, we can unpack the inputs, so developers can
        // use them as regular variables in their code.
        //
        // Unpacking of the struct means moving the values and no copies are
        // being made.
        let inputs_unpack_stmt: Stmt = parse_quote! {
            let #inputs_ident { #(#input_idents),* } = inputs;
        };
        i.block.stmts.insert(4, inputs_unpack_stmt);

        // Inject `verify` statements at the end of the function.
        let stmts_len = i.block.stmts.len();
        let verify_stmt: Stmt = parse_quote! {
            ctx.verify(proof)?;
        };
        i.block.stmts.insert(stmts_len - 1, verify_stmt);
    }

    fn visit_item_mod_mut(&mut self, i: &mut ItemMod) {
        // Search for all functions inside the annotated `mod` and visit them.
        if let Some((_, ref mut content)) = i.content {
            for item in content.iter_mut() {
                if let Item::Fn(item_fn) = item {
                    self.visit_item_fn_mut(item_fn)
                }
            }
        }
    }
}

pub(crate) fn program(mut input: ItemMod) -> Result<TokenStream> {
    let mut transform = LightProgramTransform::default();
    transform.visit_item_mod_mut(&mut input);

    let instruction_params = transform.instruction_params;

    Ok(quote! {
        #instruction_params

        #input
    })
}
