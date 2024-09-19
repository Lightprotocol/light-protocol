use std::ops::{Deref, DerefMut};

use anchor_lang::{context::Context, prelude::Pubkey, Bumps, Key, Result};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    compressed_account::{LightAccounts, PackedCompressedAccountWithMerkleContext},
    constants::CPI_AUTHORITY_PDA_SEED,
    merkle_context::PackedAddressMerkleContext,
    proof::ProofWithIndices,
    traits::{
        InvokeAccounts, InvokeCpiAccounts, InvokeCpiContextAccount, LightSystemAccount,
        SignerAccounts,
    },
    verify::{verify, InstructionDataInvokeCpi},
};

/// Provides non-argument inputs to the program, including light accounts and
/// regular accounts.
///
/// # Example
/// ```ignore
/// pub fn set_data(ctx: Context<SetData>, age: u64, other_data: u32) -> Result<()> {
///     // Set account data like this
///     (*ctx.accounts.my_account).age = age;
///     (*ctx.accounts.my_account).other_data = other_data;
///     // or like this
///     let my_account = &mut ctx.account.my_account;
///     my_account.age = age;
///     my_account.other_data = other_data;
///     Ok(())
/// }
/// ```
pub struct LightContext<'a, 'b, 'c, 'info, T, U>
where
    T: Bumps,
    U: LightAccounts,
{
    /// Context provided by Anchor.
    pub anchor_context: Context<'a, 'b, 'c, 'info, T>,
    /// Compressed accounts provided by the called in the instruction.
    pub compressed_accounts: LightCompressedAccounts,
    /// Parsed light accounts.
    pub light_accounts: U,
}

impl<'a, 'b, 'c, 'info, T, U> Deref for LightContext<'a, 'b, 'c, 'info, T, U>
where
    T: Bumps,
    U: LightAccounts,
{
    type Target = Context<'a, 'b, 'c, 'info, T>;

    fn deref(&self) -> &Self::Target {
        &self.anchor_context
    }
}

impl<'a, 'b, 'c, 'info, T, U> DerefMut for LightContext<'a, 'b, 'c, 'info, T, U>
where
    T: Bumps,
    U: LightAccounts,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.anchor_context
    }
}

impl<'a, 'b, 'c, 'info, T, U> LightContext<'a, 'b, 'c, 'info, T, U>
where
    T: Bumps
        + InvokeAccounts<'info>
        + InvokeCpiAccounts<'info>
        + InvokeCpiContextAccount<'info>
        + LightSystemAccount<'info>
        + SignerAccounts<'info>,
    U: LightAccounts,
{
    pub fn new(
        anchor_context: Context<'a, 'b, 'c, 'info, T>,
        compressed_accounts: LightCompressedAccounts,
    ) -> Result<Self> {
        let light_accounts =
            U::try_light_accounts(&compressed_accounts, anchor_context.remaining_accounts)?;
        Ok(Self {
            anchor_context,
            compressed_accounts,
            light_accounts,
        })
    }

    pub fn verify(&mut self) -> Result<()> {
        let bump = Pubkey::find_program_address(
            &[CPI_AUTHORITY_PDA_SEED],
            &self.anchor_context.accounts.get_invoking_program().key(),
        )
        .1;
        let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];

        let new_address_params = self.light_accounts.new_address_params();
        let input_compressed_accounts_with_merkle_context =
            self.compressed_accounts.accounts.clone();
        let output_compressed_accounts = self
            .light_accounts
            .output_accounts(self.anchor_context.remaining_accounts)?;

        let instruction = InstructionDataInvokeCpi {
            proof: self
                .compressed_accounts
                .proof
                .as_ref()
                .map(|proof| proof.proof.clone()),
            new_address_params,
            relay_fee: None,
            input_compressed_accounts_with_merkle_context,
            output_compressed_accounts,
            compress_or_decompress_lamports: None,
            is_compress: false,
            cpi_context: None,
        };

        verify(
            &self.anchor_context,
            &instruction,
            &[signer_seeds.as_slice()],
        )?;

        Ok(())
    }
}

/// Format in which compressed accounts are provided in instructions.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct LightCompressedAccounts {
    pub proof: Option<ProofWithIndices>,
    pub accounts: Vec<PackedCompressedAccountWithMerkleContext>,
    pub new_addresses: Vec<PackedAddressMerkleContext>,
}

#[cfg(feature = "idl-build")]
impl anchor_lang::IdlBuild for LightCompressedAccounts {}
