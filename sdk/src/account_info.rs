use anchor_lang::prelude::Result;
use solana_program::pubkey::Pubkey;

use crate::{
    account_meta::LightAccountMeta, address::PackedNewAddressParams, error::LightSdkError,
    merkle_context::PackedMerkleContext,
};

/// Information about compressed account which is being initialized.
#[derive(Debug)]
pub struct LightInputAccountInfo {
    /// Lamports.
    pub lamports: Option<u64>,
    /// Address.
    pub address: Option<[u8; 32]>,
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Root index.
    pub root_index: u16,
}

/// Information about compressed account which is being mutated.
#[derive(Debug)]
pub struct LightAccountInfo<'a> {
    /// Input account.
    pub(crate) input: Option<LightInputAccountInfo>,
    /// Owner of the account.
    ///
    /// Defaults to the program ID.
    pub owner: Option<Pubkey>,
    /// Lamports.
    pub lamports: Option<u64>,
    /// Account data.
    pub data: Option<&'a [u8]>,
    /// New Merkle tree index. Set only if you want to change the tree.
    pub output_merkle_tree_index: Option<u8>,
    /// New address parameters.
    pub new_address: Option<PackedNewAddressParams>,
}

impl<'a> LightAccountInfo<'a> {
    pub fn from_meta(meta: &'a LightAccountMeta, program_id: &Pubkey) -> Result<Self> {
        let input = match meta.merkle_context {
            Some(merkle_context) => Some(LightInputAccountInfo {
                lamports: meta.lamports,
                address: meta.address,
                merkle_context,
                root_index: meta
                    .merkle_tree_root_index
                    .ok_or(LightSdkError::ExpectedRootIndex)?,
            }),
            None => None,
        };
        let account_info = LightAccountInfo {
            input,
            owner: Some(*program_id),
            lamports: meta.lamports,
            data: meta.data.as_deref(),
            output_merkle_tree_index: meta.output_merkle_tree_index,
            new_address: match meta.address_merkle_context {
                Some(address_merkle_tree_meta) => {
                    Some(PackedNewAddressParams {
                        // Seed has to be overwritten later.
                        seed: [0u8; 32],
                        address_merkle_tree_account_index: address_merkle_tree_meta
                            .address_merkle_tree_pubkey_index,
                        address_queue_account_index: address_merkle_tree_meta
                            .address_queue_pubkey_index,
                        address_merkle_tree_root_index: meta
                            .address_merkle_tree_root_index
                            .ok_or(LightSdkError::ExpectedAddressRootIndex)?,
                    })
                }
                None => None,
            },
        };
        Ok(account_info)
    }
}

pub fn convert_metas_to_infos<'a, 'b>(
    metas: &'a Option<Vec<LightAccountMeta>>,
    program_id: &'b Pubkey,
) -> Result<Vec<LightAccountInfo<'a>>>
where
    'a: 'b,
{
    match metas {
        Some(metas) => {
            let mut infos = Vec::with_capacity(metas.len());
            for meta in metas {
                let info = LightAccountInfo::from_meta(meta, program_id)?;
                infos.push(info);
            }
            Ok(infos)
        }
        None => Ok(Vec::new()),
    }
}
