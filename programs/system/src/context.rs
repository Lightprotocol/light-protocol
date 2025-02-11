use account_compression::utils::transfer_lamports::transfer_lamports_cpi;
use anchor_lang::{prelude::*, Result};
use light_compressed_account::hash_to_bn254_field_size_be;

use crate::errors::SystemProgramError;

pub struct SystemContext<'info> {
    pub account_indices: Vec<u8>,
    pub accounts: Vec<AccountMeta>,
    // Would be better to store references.
    pub account_infos: Vec<AccountInfo<'info>>,
    // TODO: switch to store account indices once we have new context.
    // TODO: switch to (u8, [u8; 32])
    pub hashed_pubkeys: Vec<(Pubkey, [u8; 32])>,
    // Addresses for deduplication.
    // Try to find a way without storing the addresses.
    pub addresses: Vec<Option<[u8; 32]>>,
    // Index of account and fee to be paid.
    pub rollover_fee_payments: Vec<(u8, u64)>,
    pub address_fee_is_set: bool,
    pub network_fee_is_set: bool,
    pub legacy_merkle_context: Vec<(u8, MerkleTreeContext)>,
    pub invoking_program_id: Option<Pubkey>,
}

/// Helper for legacy trees.
pub struct MerkleTreeContext {
    pub rollover_fee: u64,
    pub hashed_pubkey: [u8; 32],
}

impl SystemContext<'_> {
    pub fn get_legacy_merkle_context(&mut self, index: u8) -> Option<&MerkleTreeContext> {
        self.legacy_merkle_context
            .iter()
            .find(|a| a.0 == index)
            .map(|a| &a.1)
    }
    pub fn set_legacy_merkle_context(&mut self, index: u8, context: MerkleTreeContext) {
        self.legacy_merkle_context.push((index, context));
    }

    pub fn set_address_fee(&mut self, fee: u64, index: u8) {
        msg!("set_rollover_fee");
        msg!("ix_data_index: {:?}", index);
        msg!("fee: {:?}", fee);
        if !self.address_fee_is_set {
            self.address_fee_is_set = true;
            self.rollover_fee_payments.push((index, fee));
        }
    }

    pub fn set_network_fee(&mut self, fee: u64, index: u8) {
        msg!("set_rollover_fee");
        msg!("ix_data_index: {:?}", index);
        msg!("fee: {:?}", fee);
        if !self.network_fee_is_set {
            self.network_fee_is_set = true;
            self.rollover_fee_payments.push((index, fee));
        }
    }

    pub fn get_or_hash_pubkey(&mut self, pubkey: Pubkey) -> [u8; 32] {
        let hashed_pubkey = self
            .hashed_pubkeys
            .iter()
            .find(|a| a.0 == pubkey)
            .map(|a| a.1);
        match hashed_pubkey {
            Some(hashed_pubkey) => hashed_pubkey,
            None => {
                let hashed_pubkey = hash_to_bn254_field_size_be(&pubkey.to_bytes()).unwrap().0;
                self.hashed_pubkeys.push((pubkey, hashed_pubkey));
                hashed_pubkey
            }
        }
    }
}

impl<'info> SystemContext<'info> {
    pub fn get_index_or_insert(
        &mut self,
        ix_data_index: u8,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> u8 {
        let queue_index = self
            .account_indices
            .iter()
            .position(|a| *a == ix_data_index);
        match queue_index {
            Some(index) => index as u8,
            None => {
                self.account_indices.push(ix_data_index);
                let account_info = &remaining_accounts[ix_data_index as usize];
                self.accounts.push(AccountMeta {
                    pubkey: account_info.key(),
                    is_signer: false,
                    is_writable: true,
                });
                self.account_infos.push(account_info.clone());
                self.account_indices.len() as u8 - 1
            }
        }
    }

    pub fn set_rollover_fee(&mut self, ix_data_index: u8, fee: u64) {
        msg!("set_rollover_fee");
        msg!("ix_data_index: {:?}", ix_data_index);
        msg!("fee: {:?}", fee);
        let payment = self
            .rollover_fee_payments
            .iter_mut()
            .find(|a| a.0 == ix_data_index);
        match payment {
            Some(payment) => payment.1 += fee,
            None => self.rollover_fee_payments.push((ix_data_index, fee)),
        };
    }

    /// Network fee distribution:
    /// - if any account is created or modified -> transfer network fee (5000 lamports)
    ///   (Previously we didn't charge for appends now we have to since values go into a queue.)
    /// - if an address is created -> transfer an additional network fee (5000 lamports)
    ///
    /// Examples:
    /// 1. create account with address    network fee 10,000 lamports
    /// 2. token transfer                 network fee 5,000 lamports
    /// 3. mint token                     network fee 5,000 lamports
    ///     Transfers rollover and network fees.
    pub fn transfer_fees(
        &self,
        accounts: &[AccountInfo<'info>],
        fee_payer: &AccountInfo<'info>,
    ) -> Result<()> {
        msg!(
            "self.rollover_fee_payments.len() {}",
            self.rollover_fee_payments.len()
        );
        msg!("fee payer {}", fee_payer.key());
        if self.rollover_fee_payments.len() == 1 {
            transfer_lamports_borrow_account(
                fee_payer,
                &accounts[self.rollover_fee_payments[0].0 as usize],
                self.rollover_fee_payments[0].1,
            )?;
        } else {
            // TODO: if len is 1 don't do a cpi mutate lamports.
            for (i, fee) in self.rollover_fee_payments.iter() {
                msg!("paying fee: {:?}", fee);
                msg!("to account: {:?}", accounts[*i as usize].key());
                transfer_lamports_cpi(fee_payer, &accounts[*i as usize], *fee)?;
            }
        }
        Ok(())
    }
}

/// Probably doesn't work because the recipient is not owned by the program doing the transfer.
pub fn transfer_lamports_borrow_account<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    lamports: u64,
) -> Result<()> {
    msg!("lamports: {}", lamports);
    {
        msg!("pre from lamports {}", from.try_lamports()?);
        msg!("pre to lamports {}", to.try_lamports()?);
    }
    {
        // Get mutable references directly and modify them in place
        // let mut from_lamports = from.try_borrow_mut_lamports()?;
        // let mut to_lamports = to.try_borrow_mut_lamports()?;

        // Perform subtraction and check for underflow
        // **from_lamports -= lamports;
        // // &mut (*from_lamports)
        // // .checked_sub(lamports)
        // // .ok_or(SystemProgramError::ComputeInputSumFailed)?;

        // // Perform addition and check for overflow
        // **to_lamports += lamports;
        **from.try_borrow_mut_lamports()? -= lamports;
        **to.try_borrow_mut_lamports()? += lamports;

        // .checked_add(lamports)
        // .ok_or(SystemProgramError::ComputeOutputSumFailed)?;
    }
    {
        msg!("post from lamports {}", from.try_lamports()?);
        msg!("post to lamports {}", to.try_lamports()?);
    }
    Ok(())
}
