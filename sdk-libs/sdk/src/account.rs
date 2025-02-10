use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::{AnchorDeserialize, AnchorSerialize, Result};
use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
    },
    instruction_data::data::{
        NewAddressParamsPacked as PackedNewAddressParams, OutputCompressedAccountWithPackedContext,
    },
};
use light_hasher::{DataHasher, Discriminator, Poseidon};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

use crate::{account_info::LightAccountInfo, account_meta::LightAccountMeta, error::LightSdkError};
pub trait LightAccounts<'a>: Sized {
    fn try_light_accounts(accounts: &'a [LightAccountInfo]) -> Result<Self>;
}

// TODO(vadorovsky): Implement `LightAccountLoader`.

/// A wrapper which abstracts away the UTXO model.
pub struct LightAccount<'info, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    /// State of the output account which can be modified by the developer in
    /// the program code.
    account_state: T,
    /// Account information.
    account_info: LightAccountInfo<'info>,
}

impl<'info, T> LightAccount<'info, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    pub fn from_meta_init(
        meta: &'info LightAccountMeta,
        discriminator: [u8; 8],
        new_address: [u8; 32],
        new_address_seed: [u8; 32],
        owner: &'info Pubkey,
    ) -> Result<Self> {
        let account_state = T::default();
        let account_info = LightAccountInfo::from_meta_init_without_output_data(
            meta,
            discriminator,
            new_address,
            new_address_seed,
            owner,
        )?;
        Ok(Self {
            account_state,
            account_info,
        })
    }

    pub fn from_meta_mut(
        meta: &'info LightAccountMeta,
        discriminator: [u8; 8],
        owner: &'info Pubkey,
    ) -> Result<Self> {
        let mut account_info =
            LightAccountInfo::from_meta_without_output_data(meta, discriminator, owner)?;
        let account_state = T::try_from_slice(
            meta.data
                .as_ref()
                .ok_or(LightSdkError::ExpectedData)?
                .as_slice(),
        )?;
        let input_hash = account_state
            .hash::<Poseidon>()
            .map_err(ProgramError::from)?;

        // Set the input account hash.
        //
        // PANICS: At this point we are sure `input` is `Some`
        account_info.input.as_mut().unwrap().data_hash = Some(input_hash);

        Ok(Self {
            account_state,
            account_info,
        })
    }

    pub fn from_meta_close(
        meta: &'info LightAccountMeta,
        discriminator: [u8; 8],
        owner: &'info Pubkey,
    ) -> Result<Self> {
        let mut account_info =
            LightAccountInfo::from_meta_without_output_data(meta, discriminator, owner)?;
        let account_state = T::try_from_slice(
            meta.data
                .as_ref()
                .ok_or(LightSdkError::ExpectedData)?
                .as_slice(),
        )?;
        let input_hash = account_state
            .hash::<Poseidon>()
            .map_err(ProgramError::from)?;

        // Set the input account hash.
        //
        // PANICS: At this point we are sure `input` is `Some`
        account_info.input.as_mut().unwrap().data_hash = Some(input_hash);

        Ok(Self {
            account_state,
            account_info,
        })
    }

    pub fn new_address_params(&self) -> Option<PackedNewAddressParams> {
        self.account_info.new_address_params
    }

    pub fn input_compressed_account(
        &self,
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>> {
        self.account_info.input_compressed_account()
    }

    pub fn output_compressed_account(
        &self,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>> {
        match self.account_info.output_merkle_tree_index {
            Some(merkle_tree_index) => {
                let data = {
                    let discriminator = T::discriminator();
                    let data_hash = self
                        .account_state
                        .hash::<Poseidon>()
                        .map_err(ProgramError::from)?;
                    Some(CompressedAccountData {
                        discriminator,
                        data: self.account_state.try_to_vec()?,
                        data_hash,
                    })
                };
                Ok(Some(OutputCompressedAccountWithPackedContext {
                    compressed_account: CompressedAccount {
                        owner: *self.account_info.owner,
                        lamports: self.account_info.lamports.unwrap_or(0),
                        address: self.account_info.address,
                        data,
                    },
                    merkle_tree_index,
                }))
            }
            None => Ok(None),
        }
    }
}

impl<T> Deref for LightAccount<'_, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account_state
    }
}

impl<T> DerefMut for LightAccount<'_, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account_state
    }
}
