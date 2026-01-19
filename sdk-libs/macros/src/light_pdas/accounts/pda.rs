//! PDA block code generation for rent-free accounts.
//!
//! This module handles the generation of compression blocks for PDA fields
//! marked with `#[light_account(init)]`. Each PDA field generates code for:
//! - Account extraction (get account info and key bytes)
//! - New address params struct creation
//! - Address derivation from seed and merkle tree
//! - Compression info preparation and collection

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use super::parse::ParsedPdaField;

/// Generated identifier names for a PDA field.
pub(super) struct PdaIdents {
    pub idx: u8,
    pub new_addr_params: Ident,
    pub compressed_infos: Ident,
    pub address: Ident,
    pub account_info: Ident,
    pub account_key: Ident,
    pub account_data: Ident,
}

impl PdaIdents {
    pub fn new(idx: usize) -> Self {
        Self {
            idx: idx as u8,
            new_addr_params: format_ident!("__new_addr_params_{}", idx),
            compressed_infos: format_ident!("__compressed_infos_{}", idx),
            address: format_ident!("__address_{}", idx),
            account_info: format_ident!("__account_info_{}", idx),
            account_key: format_ident!("__account_key_{}", idx),
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

    /// Returns the identifier used for new address params (for collecting in array).
    pub fn new_addr_ident(&self) -> TokenStream {
        let ident = &self.idents.new_addr_params;
        quote! { #ident }
    }

    /// Generate account extraction (get account info and key bytes).
    fn account_extraction(&self) -> TokenStream {
        let ident = &self.field.ident;
        let account_info = &self.idents.account_info;
        let account_key = &self.idents.account_key;

        quote! {
            let #account_info = self.#ident.to_account_info();
            let #account_key = #account_info.key.to_bytes();
        }
    }

    /// Generate new address params struct.
    fn new_addr_params(&self) -> TokenStream {
        let addr_tree_info = &self.field.address_tree_info;
        let new_addr_params = &self.idents.new_addr_params;
        let account_key = &self.idents.account_key;
        let idx = self.idents.idx;

        quote! {
            let #new_addr_params = {
                // Explicit type annotation ensures clear error if wrong type is provided.
                // Must be PackedAddressTreeInfo (with indices), not AddressTreeInfo (with Pubkeys).
                // If you have AddressTreeInfo, pack it client-side using pack_address_tree_info().
                let tree_info: &light_sdk_types::instruction::PackedAddressTreeInfo = &#addr_tree_info;
                light_compressed_account::instruction_data::data::NewAddressParamsAssignedPacked {
                    seed: #account_key,
                    address_merkle_tree_account_index: tree_info.address_merkle_tree_pubkey_index,
                    address_queue_account_index: tree_info.address_queue_pubkey_index,
                    address_merkle_tree_root_index: tree_info.root_index,
                    assigned_to_account: true,
                    assigned_account_index: #idx,
                }
            };
        }
    }

    /// Generate address derivation from seed and merkle tree.
    fn address_derivation(&self) -> TokenStream {
        let address = &self.idents.address;
        let new_addr_params = &self.idents.new_addr_params;

        quote! {
            let #address = light_compressed_account::address::derive_address(
                &#new_addr_params.seed,
                &cpi_accounts
                    .get_tree_account_info(#new_addr_params.address_merkle_tree_account_index as usize)?
                    .key()
                    .to_bytes(),
                &crate::ID.to_bytes(),
            );
        }
    }

    /// Generate mutable reference to account data (handles Box<Account> vs Account).
    fn account_data_extraction(&self) -> TokenStream {
        let ident = &self.field.ident;
        let account_data = &self.idents.account_data;

        let deref_expr = if self.field.is_boxed {
            quote! { &mut **self.#ident }
        } else {
            quote! { &mut *self.#ident }
        };

        quote! {
            let #account_data = #deref_expr;
        }
    }

    /// Generate compression info preparation and collection.
    fn compression_info(&self) -> TokenStream {
        let inner_type = &self.field.inner_type;
        let output_tree = &self.field.output_tree;
        let account_info = &self.idents.account_info;
        let account_data = &self.idents.account_data;
        let address = &self.idents.address;
        let new_addr_params = &self.idents.new_addr_params;
        let compressed_infos = &self.idents.compressed_infos;

        quote! {
            let #compressed_infos = light_sdk::interface::prepare_compressed_account_on_init::<#inner_type>(
                &#account_info,
                #account_data,
                &compression_config_data,
                #address,
                #new_addr_params,
                #output_tree,
                &cpi_accounts,
                &compression_config_data.address_space,
                false, // at init, we do not compress_and_close the pda, we just "register" the empty compressed account with the derived address.
            )?;
            all_compressed_infos.push(#compressed_infos);
        }
    }

    /// Build the complete compression block for this PDA field.
    pub fn build(&self) -> TokenStream {
        let account_extraction = self.account_extraction();
        let new_addr_params = self.new_addr_params();
        let address_derivation = self.address_derivation();
        let account_data_extraction = self.account_data_extraction();
        let compression_info = self.compression_info();

        quote! {
            // Get account info early before any mutable borrows
            #account_extraction
            #new_addr_params
            // Derive the compressed address
            #address_derivation
            // Get mutable reference to inner account data
            #account_data_extraction
            #compression_info
        }
    }
}

/// Generate compression blocks for PDA fields using PdaBlockBuilder.
///
/// Returns a tuple of:
/// - Vector of TokenStreams for compression blocks
/// - Vector of TokenStreams for new address parameter identifiers
pub(super) fn generate_pda_compress_blocks(
    fields: &[ParsedPdaField],
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut blocks = Vec::with_capacity(fields.len());
    let mut addr_idents = Vec::with_capacity(fields.len());

    for (idx, field) in fields.iter().enumerate() {
        let builder = PdaBlockBuilder::new(field, idx);
        addr_idents.push(builder.new_addr_ident());
        blocks.push(builder.build());
    }

    (blocks, addr_idents)
}
