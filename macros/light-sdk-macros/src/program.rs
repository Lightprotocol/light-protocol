use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_quote, visit_mut::VisitMut, Attribute, FnArg, GenericArgument, Ident, Item, ItemFn,
    ItemMod, PathArguments, Result, Stmt, Type,
};

#[derive(Default)]
struct LightProgramTransform {}

impl VisitMut for LightProgramTransform {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        // Add `#[allow(clippy::too_many_arguments)]` attribute in case. We are
        // injecting many arguments in this macro and they can easily go over
        // the limit.
        let clippy_attr: Attribute = parse_quote! {
            #[allow(clippy::too_many_arguments)]
        };
        i.attrs.push(clippy_attr);

        // Find the `ctx` argument.
        let ctx_arg = i.sig.inputs.first_mut().unwrap();

        // Retrieve the type of `ctx`.
        let pat_type = match ctx_arg {
            FnArg::Typed(pat_type) => pat_type,
            _ => return,
        };

        // Get the last path segment of `ctx` type.
        // It should be `LightContext`...
        let type_path = match pat_type.ty.as_mut() {
            Type::Path(type_path) => type_path,
            _ => return,
        };
        let ctx_segment = &mut type_path.path.segments.last_mut().unwrap();
        // ...and we replace it with Anchor's `Context`
        ctx_segment.ident = Ident::new("Context", Span::call_site());

        // Figure out what's are the names of:
        //
        // - The struct with Anchor accounts (implementing `anchor_lang::Accounts`) -
        //   it's specified as the last generic argument in `ctx`, e.g. `MyInstruction`.
        // - The struct with compressed accounds (implementing `LightAccounts`) -
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

        // Inject an `inputs: Vec<Vec<u8>>` argument to all instructions. The
        // purpose of that additional argument is passing compressed accounts.
        let inputs_arg: FnArg = parse_quote! { inputs: Vec<Vec<u8>> };
        i.sig.inputs.insert(1, inputs_arg);

        // Inject Merkle context related arguments.
        let proof_arg: FnArg = parse_quote! { proof: CompressedProof };
        i.sig.inputs.insert(2, proof_arg);
        let merkle_context_arg: FnArg = parse_quote! { merkle_context: PackedMerkleContext };
        i.sig.inputs.insert(3, merkle_context_arg);
        let merkle_tree_root_index_arg: FnArg = parse_quote! { merkle_tree_root_index: u16 };
        i.sig.inputs.insert(4, merkle_tree_root_index_arg);
        let address_merkle_context_arg: FnArg =
            parse_quote! { address_merkle_context: PackedAddressMerkleContext };
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

        // Inject `check_constraints` call right after.
        let check_constraints_stmt: Stmt = parse_quote! {
            ctx.check_constraints()?;
        };
        i.block.stmts.insert(1, check_constraints_stmt);

        // Inject `derive_address_seeds` and  `verify` statements at the end of
        // the function.
        let stmts_len = i.block.stmts.len();
        let derive_address_seed_stmt: Stmt = parse_quote! {
            ctx.derive_address_seeds(address_merkle_context);
        };
        i.block
            .stmts
            .insert(stmts_len - 1, derive_address_seed_stmt);
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

    Ok(quote! {
        pub trait LightContextExt {
            fn check_constraints(&self) -> Result<()>;
            fn derive_address_seeds(&mut self, address_merkle_context: PackedAddressMerkleContext);
        }

        #[program]
        #input
    })
}
