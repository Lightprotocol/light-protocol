use anchor_lang::anchor_syn::idl::types::{
    IdlField, IdlType, IdlTypeDefinition, IdlTypeDefinitionTy,
};
use borsh::{BorshDeserialize, BorshSerialize};
use light_indexed_merkle_tree::array::IndexedElement;
use num_bigint::BigUint;
use solana_program::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub hash: [u8; 32],
    pub leaf_index: u64,
    pub merkle_tree: Pubkey,
    pub proof: Vec<[u8; 32]>,
    pub root_seq: u64,
}

// For consistency with the Photon API.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct NewAddressProofWithContext {
    pub merkle_tree: Pubkey,
    pub root: [u8; 32],
    pub root_seq: u64,
    pub low_address_index: u64,
    pub low_address_value: [u8; 32],
    pub low_address_next_index: u64,
    pub low_address_next_value: [u8; 32],
    pub low_address_proof: [[u8; 32]; 16],
    pub new_low_element: Option<IndexedElement<usize>>,
    pub new_element: Option<IndexedElement<usize>>,
    pub new_element_next_value: Option<BigUint>,
}
#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

#[cfg(feature = "idl-build")]
impl anchor_lang::IdlBuild for CompressedProof {
    fn __anchor_private_full_path() -> String {
        format!("{}::{}", "light_sdk::proof", "CompressedProof")
    }

    fn __anchor_private_gen_idl_type(
    ) -> Option<anchor_lang::anchor_syn::idl::types::IdlTypeDefinition> {
        Some(IdlTypeDefinition {
            name: "CompressedProof".to_string(),
            generics: None,
            docs: None,
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "a".into(),
                        docs: None,
                        ty: IdlType::Array(Box::new(IdlType::U8), 32),
                    },
                    IdlField {
                        name: "b".into(),
                        docs: None,
                        ty: IdlType::Array(Box::new(IdlType::U8), 64),
                    },
                    IdlField {
                        name: "c".into(),
                        docs: None,
                        ty: IdlType::Array(Box::new(IdlType::U8), 32),
                    },
                ],
            },
        })
    }

    fn __anchor_private_insert_idl_defined(
        defined_types: &mut std::collections::HashMap<
            String,
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        >,
    ) {
        if let Some(ty) = Self::__anchor_private_gen_idl_type() {
            defined_types.insert(Self::__anchor_private_full_path(), ty);
        }
    }
}

#[derive(Debug)]
pub struct ProofRpcResult {
    pub proof: CompressedProof,
    pub root_indices: Vec<u16>,
    pub address_root_indices: Vec<u16>,
}

#[cfg(feature = "idl-build")]
impl anchor_lang::IdlBuild for ProofRpcResult {
    fn __anchor_private_full_path() -> String {
        format!("{}::{}", "light_sdk::proof", "ProofRpcResult")
    }
    fn __anchor_private_gen_idl_type(
    ) -> Option<anchor_lang::anchor_syn::idl::types::IdlTypeDefinition> {
        Some(IdlTypeDefinition {
            name: "ProofRpcResult".to_string(),
            generics: None,
            docs: None,
            ty: IdlTypeDefinitionTy::Struct {
                fields: vec![
                    IdlField {
                        name: "proof".into(),
                        docs: None,
                        ty: IdlType::Defined("light_sdk::proof::CompressedProof".to_string()),
                    },
                    IdlField {
                        name: "root_indices".into(),
                        docs: None,
                        ty: IdlType::Vec(Box::new(IdlType::U16)),
                    },
                    IdlField {
                        name: "address_root_indices".into(),
                        docs: None,
                        ty: IdlType::Vec(Box::new(IdlType::U16)),
                    },
                ],
            },
        })
    }

    fn __anchor_private_insert_idl_defined(
        defined_types: &mut std::collections::HashMap<
            String,
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        >,
    ) {
        if let Some(ty) = Self::__anchor_private_gen_idl_type() {
            defined_types.insert(Self::__anchor_private_full_path(), ty);
        }
    }
}
