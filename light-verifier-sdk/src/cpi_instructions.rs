use anchor_lang::prelude::*;

use crate::light_transaction::{Config, Transaction};

impl<T: Config, const NR_LEAVES: usize, const NR_NULLIFIERS: usize>
    Transaction<'_, '_, '_, NR_LEAVES, NR_NULLIFIERS, T>
{
    pub fn insert_nullifiers_cpi(&self) -> Result<()> {
        let (seed, bump) = self.get_seeds()?;
        let bump = &[bump];
        let seeds = &[&[seed.as_slice(), bump][..]];
        let accounts = merkle_tree_program::cpi::accounts::InitializeNullifiers {
            authority: self.accounts.authority.to_account_info().clone(),
            system_program: self.accounts.system_program.to_account_info(),
            registered_verifier_pda: self.accounts.registered_verifier_pda.to_account_info(),
        };

        let mut cpi_ctx = CpiContext::new_with_signer(
            self.accounts.program_merkle_tree.to_account_info(),
            accounts,
            seeds,
        );
        cpi_ctx = cpi_ctx.with_remaining_accounts(self.accounts.remaining_accounts.to_vec());

        merkle_tree_program::cpi::initialize_nullifiers(cpi_ctx, self.verifier_state.nullifiers.to_vec())
    }

    pub fn withdraw_sol_cpi<'a, 'b>(&self, pub_amount_checked: u64) -> Result<()> {
        let (seed, bump) = self.get_seeds()?;
        let bump = &[bump];
        let seeds = &[&[seed.as_slice(), bump][..]];

        let accounts = merkle_tree_program::cpi::accounts::WithdrawSol {
            authority: self.accounts.authority.to_account_info(),
            merkle_tree_token: self.accounts.sender_sol.as_ref().unwrap().clone(),
            registered_verifier_pda: self.accounts.registered_verifier_pda.to_account_info(),
            recipient: self.accounts.recipient_sol.as_ref().unwrap().clone(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            self.accounts.program_merkle_tree.to_account_info(),
            accounts,
            seeds,
        );
        merkle_tree_program::cpi::withdraw_sol(cpi_ctx, pub_amount_checked)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn withdraw_spl_cpi<'a, 'b>(&self, pub_amount_checked: u64) -> Result<()> {
        let (seed, bump) = self.get_seeds()?;
        let bump = &[bump];
        let seeds = &[&[seed.as_slice(), bump][..]];

        let accounts = merkle_tree_program::cpi::accounts::WithdrawSpl {
            authority: self.accounts.authority.to_account_info(),
            merkle_tree_token: self.accounts.sender_spl.as_ref().unwrap().clone(),
            token_authority: self.accounts.token_authority.as_ref().unwrap().clone(),
            token_program: self.accounts.token_program.unwrap().to_account_info(),
            registered_verifier_pda: self.accounts.registered_verifier_pda.to_account_info(),
            recipient: self
                .accounts
                .recipient_spl
                .as_ref()
                .unwrap()
                .to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            self.accounts.program_merkle_tree.to_account_info(),
            accounts,
            seeds,
        );
        merkle_tree_program::cpi::withdraw_spl(cpi_ctx, pub_amount_checked)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_two_leaves_cpi<'info, 'a, 'c>(
        &self,
        two_leaves_pda_i: usize,
        leaf_left: [u8; 32],
        leaf_right: [u8; 32],
        encrypted_utxos: Vec<u8>,
    ) -> Result<()> {
        let (seed, bump) = self.get_seeds()?;
        let bump = &[bump];
        let seeds = &[&[seed.as_slice(), bump][..]];

        let accounts = merkle_tree_program::cpi::accounts::InsertTwoLeaves {
            authority: self.accounts.authority.to_account_info(),
            two_leaves_pda: self.accounts.remaining_accounts[T::NR_NULLIFIERS + two_leaves_pda_i]
                .to_account_info(),
            system_program: self.accounts.system_program.to_account_info(),
            transaction_merkle_tree: self.accounts.transaction_merkle_tree.to_account_info(),
            registered_verifier_pda: self.accounts.registered_verifier_pda.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            self.accounts.program_merkle_tree.to_account_info(),
            accounts,
            seeds,
        );
        merkle_tree_program::cpi::insert_two_leaves(
            cpi_ctx,
            leaf_left,
            leaf_right,
            [
                encrypted_utxos.to_vec(),
                vec![0u8; 256 - encrypted_utxos.len()],
            ]
            .concat()
            .try_into()
            .unwrap(),
        )
    }

    pub fn get_seeds<'a>(&self) -> Result<([u8; 32], u8)> {
        let (_, bump) = anchor_lang::prelude::Pubkey::find_program_address(
            &[self.accounts.program_id.key().to_bytes().as_ref()],
            self.accounts.program_id,
        );
        let seed = self
            .accounts
            .program_merkle_tree
            .to_account_info()
            .key()
            .to_bytes();
        Ok((seed, bump))
    }
}
