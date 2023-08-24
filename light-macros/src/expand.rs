use bs58::decode;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::Parse, parse_quote, punctuated::Punctuated, token::Brace, Error, Expr, Field, Fields,
    FieldsNamed, ItemStruct, LitStr, Result,
};

const PUBKEY_LEN: usize = 32;

pub(crate) struct PubkeyArgs {
    pub(crate) pubkey: LitStr,
}

impl Parse for PubkeyArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self {
            pubkey: input.parse()?,
        })
    }
}

pub(crate) fn pubkey(args: PubkeyArgs) -> Result<TokenStream> {
    let v = decode(args.pubkey.value())
        .into_vec()
        .map_err(|_| Error::new(args.pubkey.span(), "Invalid base58 string"))?;
    let v_len = v.len();

    let arr: [u8; PUBKEY_LEN] = v.try_into().map_err(|_| {
        Error::new(
            args.pubkey.span(),
            format!(
                "Invalid size of decoded public key, expected 32, got {}",
                v_len,
            ),
        )
    })?;

    Ok(quote! {
        ::anchor_lang::prelude::Pubkey::new_from_array([ #(#arr),* ])
    })
}

pub(crate) struct LightVerifierAccountsArgs {
    sol: bool,
    spl: bool,
    signing_address: Option<Expr>,
    verifier_program_id: Option<Expr>,
}

impl Parse for LightVerifierAccountsArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut sol = false;
        let mut spl = false;
        let mut signing_address = None;
        let mut verifier_program_id = None;

        while !input.is_empty() {
            // Try to parse an ident from the stream
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "sol" => sol = true,
                "spl" => spl = true,
                "signing_address" => {
                    let _eq_token: syn::Token![=] = input.parse()?;
                    let expr: Expr = input.parse()?;
                    signing_address = Some(expr);
                }
                "verifier_program_id" => {
                    let _eq_token: syn::Token![=] = input.parse()?;
                    let expr: Expr = input.parse()?;
                    verifier_program_id = Some(expr);
                }
                _ => return Err(input.error("Unexpected identifier")),
            }

            // If there's a comma, consume it, otherwise break out of the loop
            if input.peek(syn::token::Comma) {
                let _ = input.parse::<syn::token::Comma>();
            } else {
                break;
            }
        }

        Ok(Self {
            sol,
            spl,
            signing_address,
            verifier_program_id,
        })
    }
}

pub(crate) fn light_verifier_accounts(
    args: LightVerifierAccountsArgs,
    strct: ItemStruct,
) -> Result<TokenStream> {
    let sol_fields = if args.sol {
        quote! {
            /// CHECK: Is checked depending on deposit or withdrawal.
            #[account(mut)]
            pub sender_sol: UncheckedAccount<'info>,
            /// CHECK: Is checked depending on deposit or withdrawal.
            #[account(mut)]
            pub recipient_sol: UncheckedAccount<'info>,
        }
    } else {
        quote! {}
    };

    let spl_fields = if args.spl {
        quote! {
            pub token_program: Program<'info, ::anchor_spl::token::Token>,
            /// CHECK: Is checked when it is used during spl withdrawals.
            #[account(
                mut,
                seeds=[::merkle_tree_program::utils::constants::TOKEN_AUTHORITY_SEED],
                bump,
                seeds::program=::merkle_tree_program::program::MerkleTreeProgram::id())]
            pub token_authority: AccountInfo<'info>,
            /// CHECK: Is checked depending on deposit or withdrawal.
            #[account(mut)]
            pub sender_spl: UncheckedAccount<'info>,
            /// CHECK: Is checked depending on deposit or withdrawal.
            #[account(mut)]
            pub recipient_spl: UncheckedAccount<'info>,
        }
    } else {
        quote! {}
    };

    let signing_address_cond = match args.signing_address {
        Some(signing_address) => quote! {
            address = #signing_address
        },
        None => quote! {},
    };

    let authority_seeds_program = match args.verifier_program_id {
        Some(ref verifier_program_id) => quote! {
            seeds::program = #verifier_program_id
        },
        None => quote! {},
    };

    let registered_verifier_pda_seeds = match args.verifier_program_id {
        Some(ref verifier_program_id) => quote! {
            seeds = [#verifier_program_id.to_bytes().as_ref()]
        },
        None => quote! {
            seeds = [__program_id.key().to_bytes().as_ref()]
        },
    };

    // This `anchor_syn::AccountsStruct` instance is created only to provide
    // our own common fields (which we want to append to the original struct
    // provided as the `item` argument). We define our fields there and then
    // parse them with `parse_quote!` macro.
    let common_fields_strct: ItemStruct = parse_quote! {
        pub struct CommonFields {
            #[account(
                mut,
                #signing_address_cond
            )]
            pub signing_address: Signer<'info>,
            pub system_program: Program<'info, System>,
            pub program_merkle_tree: Program<'info, ::merkle_tree_program::program::MerkleTreeProgram>,
            /// CHECK: Is the same as in integrity hash.
            #[account(mut)]
            pub transaction_merkle_tree: AccountLoader<'info, ::merkle_tree_program::transaction_merkle_tree::state::TransactionMerkleTree>,
            /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
            #[account(
                mut,
                seeds = [
                    ::merkle_tree_program::program::MerkleTreeProgram::id().to_bytes().as_ref()
                ],
                bump,
                #authority_seeds_program
            )]
            pub authority: UncheckedAccount<'info>,

            /// CHECK: Is not checked the relayer has complete freedom.
            #[account(mut)]
            pub relayer_recipient_sol: UncheckedAccount<'info>,

            #sol_fields

            #spl_fields

            /// Verifier config pda which needs to exist.
            #[account(
                mut,
                #registered_verifier_pda_seeds,
                bump,
                seeds::program = MerkleTreeProgram::id()
            )]
            pub registered_verifier_pda: Account<
                'info,
                ::merkle_tree_program::config_accounts::register_verifier::RegisteredVerifier
            >,
            /// CHECK: It gets checked inside the event_call.
            pub log_wrapper: UncheckedAccount<'info>,
            #[account(mut)]
            pub event_merkle_tree: AccountLoader<'info, ::merkle_tree_program::event_merkle_tree::EventMerkleTree>,
        }
    };

    let mut fields = Punctuated::new();

    for field in common_fields_strct.fields.iter() {
        let field = Field {
            attrs: field.attrs.clone(),
            vis: field.vis.clone(),
            ident: field.ident.clone(),
            colon_token: field.colon_token.clone(),
            ty: field.ty.clone(),
        };
        fields.push(field);
    }
    for field in strct.fields.iter() {
        let field = Field {
            attrs: field.attrs.clone(),
            vis: field.vis.clone(),
            ident: field.ident.clone(),
            colon_token: field.colon_token.clone(),
            ty: field.ty.clone(),
        };
        fields.push(field);
    }

    let fields = Fields::Named(FieldsNamed {
        brace_token: Brace {
            span: Span::call_site(),
        },
        named: fields,
    });

    let strct = ItemStruct {
        attrs: strct.attrs,
        vis: strct.vis,
        struct_token: strct.struct_token,
        ident: strct.ident,
        generics: strct.generics,
        fields,
        semi_token: strct.semi_token,
    };

    Ok(strct.into_token_stream())
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_pubkey() {
        let res = pubkey(parse_quote! { "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK" });
        assert_eq!(
            res.unwrap().to_string(),
            ":: anchor_lang :: prelude :: Pubkey :: new_from_array ([9u8 , 42u8 \
             , 19u8 , 238u8 , 149u8 , 196u8 , 28u8 , 186u8 , 8u8 , 166u8 , \
             127u8 , 90u8 , 198u8 , 126u8 , 141u8 , 247u8 , 225u8 , 218u8 , \
             17u8 , 98u8 , 94u8 , 29u8 , 100u8 , 19u8 , 127u8 , 143u8 , 79u8 , \
             35u8 , 131u8 , 3u8 , 127u8 , 20u8])",
        );
    }

    #[test]
    fn test_light_verifier_accounts() {
        let strct: ItemStruct = parse_quote! {
            #[derive(Accounts)]
            struct LightInstruction {
                pub verifier_state: Signer<'info>,
            }
        };

        let res_no_args = light_verifier_accounts(parse_quote! {}, strct.clone())
            .expect("Failed to expand light_verifier_accounts")
            .to_string();

        assert!(res_no_args.contains("pub program_merkle_tree"));
        assert!(res_no_args.contains("pub transaction_merkle_tree"));
        assert!(res_no_args.contains("pub event_merkle_tree"));
        assert!(res_no_args.contains("seeds = [__program_id . key () . to_bytes () . as_ref ()]"));
        assert!(!res_no_args.contains("pub sender_sol"));
        assert!(!res_no_args.contains("pub recipient_sol"));
        assert!(!res_no_args.contains("pub sender_spl"));
        assert!(!res_no_args.contains("pub recipient_spl"));

        let res_sol = light_verifier_accounts(parse_quote! { sol }, strct.clone())
            .expect("Failed to expand light_verifier_accounts")
            .to_string();

        assert!(res_sol.contains("pub sender_sol"));
        assert!(res_sol.contains("pub recipient_sol"));
        assert!(!res_sol.contains("pub sender_spl"));
        assert!(!res_sol.contains("pub recipient_spl"));

        let res_sol_spl = light_verifier_accounts(parse_quote! { sol, spl }, strct.clone())
            .expect("Failed to expand light_verifier_accounts")
            .to_string();

        assert!(res_sol_spl.contains("pub sender_sol"));
        assert!(res_sol_spl.contains("pub recipient_sol"));
        assert!(res_sol_spl.contains("pub sender_spl"));
        assert!(res_sol_spl.contains("pub recipient_spl"));

        let res_signing_address = light_verifier_accounts(
            parse_quote! { signing_address = verifier_state.signer },
            strct.clone(),
        )
        .expect("Failed to expand light_verifier_accounts")
        .to_string();

        assert!(
            res_signing_address.contains("# [account (mut , address = verifier_state . signer)]")
        );

        let res_verifier_program_id = light_verifier_accounts(
            parse_quote! { verifier_program_id = VerifierProgramTwo::id() },
            strct,
        )
        .expect("Failed to expand light_verifier_accounts")
        .to_string();

        assert!(res_verifier_program_id.contains("seeds :: program = VerifierProgramTwo :: id ()"));
        assert!(res_verifier_program_id
            .contains("seeds = [VerifierProgramTwo :: id () . to_bytes () . as_ref ()]"))
    }
}
