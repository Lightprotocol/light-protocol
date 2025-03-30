use std::ops::{Deref, DerefMut};

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::pubkey::Pubkey;
use light_hasher::{DataHasher, Discriminator, Poseidon};

use crate::{
    account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
    error::LightSdkError,
    instruction::account_meta::CompressedAccountMetaTrait,
};

#[derive(Debug, PartialEq)]
pub struct CBorshAccount<
    'a,
    A: BorshSerialize + BorshDeserialize + Discriminator + DataHasher + Default,
> {
    owner: &'a Pubkey,
    pub account: A,
    account_info: CompressedAccountInfo,
}

impl<'a, A: BorshSerialize + BorshDeserialize + Discriminator + DataHasher + Default>
    CBorshAccount<'a, A>
{
    pub fn new_init(
        owner: &'a Pubkey,
        address: Option<[u8; 32]>,
        output_merkle_tree_index: u8,
    ) -> Self {
        let output_account_info = OutAccountInfo {
            output_merkle_tree_index,
            ..Default::default()
        };
        Self {
            owner,
            account: A::default(),
            account_info: CompressedAccountInfo {
                discriminator: A::discriminator(),
                address,
                input: None,
                output: Some(output_account_info),
            },
        }
    }

    pub fn new_mut(
        owner: &'a Pubkey,
        input_account_meta: &impl CompressedAccountMetaTrait,
        input_account: A,
    ) -> Result<Self, LightSdkError> {
        let input_account_info = {
            let input_data_hash = input_account.hash::<Poseidon>()?;
            InAccountInfo {
                data_hash: input_data_hash,
                lamports: input_account_meta.get_lamports().unwrap_or_default(),
                merkle_context: *input_account_meta.get_merkle_context(),
                root_index: input_account_meta.get_root_index().unwrap_or_default(),
            }
        };
        let output_account_info = {
            OutAccountInfo {
                lamports: input_account_meta.get_lamports().unwrap_or_default(),
                output_merkle_tree_index: input_account_meta.get_output_merkle_tree_index(),
                ..Default::default()
            }
        };

        Ok(Self {
            owner,
            account: input_account,
            account_info: CompressedAccountInfo {
                discriminator: A::discriminator(),
                address: input_account_meta.get_address(),
                input: Some(input_account_info),
                output: Some(output_account_info),
            },
        })
    }

    pub fn new_close(
        owner: &'a Pubkey,
        input_account_meta: &impl CompressedAccountMetaTrait,
        input_account: A,
    ) -> Result<Self, LightSdkError> {
        let input_account_info = {
            let input_data_hash = input_account.hash::<Poseidon>()?;
            InAccountInfo {
                data_hash: input_data_hash,
                lamports: input_account_meta.get_lamports().unwrap_or_default(),
                merkle_context: *input_account_meta.get_merkle_context(),
                root_index: input_account_meta.get_root_index().unwrap_or_default(),
            }
        };
        Ok(Self {
            owner,
            account: input_account,
            account_info: CompressedAccountInfo {
                discriminator: A::discriminator(),
                address: input_account_meta.get_address(),
                input: Some(input_account_info),
                output: None,
            },
        })
    }

    pub fn discriminator(&self) -> &[u8; 8] {
        &self.account_info.discriminator
    }

    pub fn lamports(&self) -> u64 {
        if let Some(output) = self.account_info.output.as_ref() {
            output.lamports
        } else if let Some(input) = self.account_info.input.as_ref() {
            input.lamports
        } else {
            0
        }
    }

    pub fn lamports_mut(&mut self) -> &mut u64 {
        if let Some(output) = self.account_info.output.as_mut() {
            &mut output.lamports
        } else if let Some(input) = self.account_info.input.as_mut() {
            &mut input.lamports
        } else {
            panic!("No lamports field available in account_info")
        }
    }

    pub fn address(&self) -> &Option<[u8; 32]> {
        &self.account_info.address
    }

    pub fn owner(&self) -> &Pubkey {
        self.owner
    }

    pub fn in_account_info(&self) -> &Option<InAccountInfo> {
        &self.account_info.input
    }

    pub fn out_account_info(&mut self) -> &Option<OutAccountInfo> {
        &self.account_info.output
    }

    /// 1. Serializes the account data and sets the output data hash.
    /// 2. Returns CompressedAccountInfo.
    ///
    /// Note this is an expensive operation
    /// that should only be called once per instruction.
    pub fn to_account_info(mut self) -> Result<CompressedAccountInfo, LightSdkError> {
        if let Some(output) = self.account_info.output.as_mut() {
            output.data_hash = self.account.hash::<Poseidon>()?;
            output.data = self
                .account
                .try_to_vec()
                .map_err(|_| LightSdkError::Borsh)?;
        }
        Ok(self.account_info)
    }
}

impl<A: BorshSerialize + BorshDeserialize + Discriminator + DataHasher + Default> Deref
    for CBorshAccount<'_, A>
{
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}

impl<A: BorshSerialize + BorshDeserialize + Discriminator + DataHasher + Default> DerefMut
    for CBorshAccount<'_, A>
{
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.account
    }
}
