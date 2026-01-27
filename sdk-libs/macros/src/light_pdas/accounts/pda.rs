//! PDA block code generation for rent-free accounts.
//!
//! This module handles the generation of compression blocks for PDA fields
//! marked with `#[light_account(init)]`. Each PDA field generates code for:
//! - Account extraction (get account info and key)
//! - Address derivation and registration via prepare_compressed_account_on_init

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::parse::ParsedPdaField;

/// Generated identifier names for a PDA field.
pub(super) struct PdaIdents {
    pub idx: u8,
    pub account_info: Ident,
    pub account_key: Ident,
    pub address_tree_pubkey: Ident,
    pub account_data: Ident,
}

impl PdaIdents {
    pub fn new(idx: usize) -> Self {
        Self {
            idx: idx as u8,
            account_info: format_ident!("__account_info_{}", idx),
            account_key: format_ident!("__account_key_{}", idx),
            address_tree_pubkey: format_ident!("__address_tree_pubkey_{}", idx),
            account_data: format_ident!("__account_data_{}", idx),
        }
    }
}

/// Builder for PDA compression block code generation.
pub(super) struct PdaBlockBuilder<'a> {
    field: &'a ParsedPdaField,
    idents: PdaIdents,
}

impl<'a> PdaBlockBuilder<'a> {
    pub fn new(field: &'a ParsedPdaField, idx: usize) -> Self {
        Self {
            field,
            idents: PdaIdents::new(idx),
        }
    }

    /// Generate account extraction (get account info and key).
    fn account_extraction(&self) -> TokenStream {
        let ident = &self.field.ident;
        let account_info = &self.idents.account_info;
        let account_key = &self.idents.account_key;

        quote! {
            let #account_info = self.#ident.to_account_info();
            let #account_key = *#account_info.key;
        }
    }

    /// Generate address tree pubkey extraction.
    fn address_tree_extraction(&self) -> TokenStream {
        let addr_tree_info = &self.field.address_tree_info;
        let address_tree_pubkey = &self.idents.address_tree_pubkey;

        quote! {
            let #address_tree_pubkey: solana_pubkey::Pubkey = {
                use light_sdk::light_account_checks::AccountInfoTrait;
                // Explicit type annotation ensures clear error if wrong type is provided.
                let tree_info: &::light_sdk::sdk_types::PackedAddressTreeInfo = &#addr_tree_info;
                cpi_accounts
                    .get_tree_account_info(tree_info.address_merkle_tree_pubkey_index as usize)?
                    .pubkey()
            };
        }
    }

    /// Generate account data initialization (set CompressionInfo).
    fn account_data_init(&self) -> TokenStream {
        let ident = &self.field.ident;
        let account_data = &self.idents.account_data;

        if self.field.is_zero_copy {
            // AccountLoader uses load_init() for newly initialized accounts
            let account_guard = format_ident!("{}_guard", ident);
            quote! {
                {
                    let current_slot = anchor_lang::solana_program::sysvar::clock::Clock::get()?.slot;
                    let mut #account_guard = self.#ident.load_init()
                        .map_err(|_| solana_program_error::ProgramError::InvalidAccountData)?;
                    let #account_data = &mut *#account_guard;
                    // For zero-copy Pod accounts, set compression_info directly
                    #account_data.compression_info =
                        light_sdk::compressible::CompressionInfo::new_from_config(
                            &compression_config_data,
                            current_slot,
                        );
                }
            }
        } else if self.field.is_boxed {
            quote! {
                {
                    use light_sdk::interface::LightAccount;
                    use anchor_lang::AnchorSerialize;
                    let current_slot = anchor_lang::solana_program::sysvar::clock::Clock::get()?.slot;
                    // Get account info BEFORE mutable borrow
                    let account_info = self.#ident.to_account_info();
                    // Scope the mutable borrow
                    {
                        let #account_data = &mut **self.#ident;
                        // Initialize CompressionInfo using v2 LightAccount trait
                        #account_data.set_decompressed(&compression_config_data, current_slot);
                    }
                    // Now serialize - the mutable borrow above is released
                    let mut data = account_info
                        .try_borrow_mut_data()
                        .map_err(|_| light_sdk::error::LightSdkError::ConstraintViolation)?;
                    self.#ident.serialize(&mut &mut data[8..])
                        .map_err(|_| light_sdk::error::LightSdkError::ConstraintViolation)?;
                }
            }
        } else {
            quote! {
                {
                    use light_sdk::interface::LightAccount;
                    use anchor_lang::AnchorSerialize;
                    let current_slot = anchor_lang::solana_program::sysvar::clock::Clock::get()?.slot;
                    // Get account info BEFORE mutable borrow
                    let account_info = self.#ident.to_account_info();
                    // Scope the mutable borrow
                    {
                        let #account_data = &mut *self.#ident;
                        // Initialize CompressionInfo using v2 LightAccount trait
                        #account_data.set_decompressed(&compression_config_data, current_slot);
                    }
                    // Now serialize - the mutable borrow above is released
                    let mut data = account_info
                        .try_borrow_mut_data()
                        .map_err(|_| light_sdk::error::LightSdkError::ConstraintViolation)?;
                    self.#ident.serialize(&mut &mut data[8..])
                        .map_err(|_| light_sdk::error::LightSdkError::ConstraintViolation)?;
                }
            }
        }
    }

    /// Generate the call to prepare_compressed_account_on_init.
    fn prepare_call(&self) -> TokenStream {
        let addr_tree_info = &self.field.address_tree_info;
        let output_tree = &self.field.output_tree;
        let account_key = &self.idents.account_key;
        let address_tree_pubkey = &self.idents.address_tree_pubkey;
        let idx = self.idents.idx;

        quote! {
            {
                // Explicit type annotation for tree_info
                let tree_info: &::light_sdk::sdk_types::PackedAddressTreeInfo = &#addr_tree_info;

                ::light_sdk::interface::prepare_compressed_account_on_init(
                    &#account_key,
                    &#address_tree_pubkey,
                    tree_info,
                    #output_tree,
                    #idx,
                    &crate::ID,
                    &mut all_new_address_params,
                    &mut all_compressed_infos,
                )?;
            }
        }
    }

    /// Build the complete compression block for this PDA field.
    pub fn build(&self) -> TokenStream {
        let account_extraction = self.account_extraction();
        let address_tree_extraction = self.address_tree_extraction();
        let account_data_init = self.account_data_init();
        let prepare_call = self.prepare_call();

        quote! {
            // Get account info early before any mutable borrows
            #account_extraction
            // Extract address tree pubkey
            #address_tree_extraction
            // Initialize CompressionInfo in account data
            #account_data_init
            // Register compressed address
            #prepare_call
        }
    }
}

/// Generate compression blocks for PDA fields using PdaBlockBuilder.
///
/// Returns a vector of TokenStreams for compression blocks.
/// The blocks push into `all_new_address_params` and `all_compressed_infos` vectors.
pub(super) fn generate_pda_compress_blocks(
    fields: &[ParsedPdaField],
) -> Vec<TokenStream> {
    fields
        .iter()
        .enumerate()
        .map(|(idx, field)| PdaBlockBuilder::new(field, idx).build())
        .collect()
}
