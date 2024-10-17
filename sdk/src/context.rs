use std::ops::{Deref, DerefMut};

use anchor_lang::{
    context::Context, prelude::Pubkey, AnchorDeserialize, AnchorSerialize, Bumps, Discriminator,
    Key, Result,
};
use light_hasher::DataHasher;

use crate::{
    address::PackedNewAddressParams,
    compressed_account::{LightAccount, LightAccounts, PackedCompressedAccountWithMerkleContext},
    constants::CPI_AUTHORITY_PDA_SEED,
    merkle_context::{PackedAddressMerkleContext, PackedMerkleContext},
    proof::{CompressedProof, ProofRpcResult},
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
    U: LightAccounts<'a>,
{
    /// Context provided by Anchor.
    pub anchor_context: Context<'a, 'b, 'c, 'info, T>,
    pub light_accounts: U,
    // pub new_addresses:
    // pub inputs: LightInstructionInputs,
}

impl<'a, 'b, 'c, 'info, T, U> Deref for LightContext<'a, 'b, 'c, 'info, T, U>
where
    T: Bumps,
    U: LightAccounts<'a>,
{
    type Target = Context<'a, 'b, 'c, 'info, T>;

    fn deref(&self) -> &Self::Target {
        &self.anchor_context
    }
}

impl<'a, 'b, 'c, 'info, T, U> DerefMut for LightContext<'a, 'b, 'c, 'info, T, U>
where
    T: Bumps,
    U: LightAccounts<'a>,
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
    U: LightAccounts<'a>,
{
    pub fn new(
        anchor_context: Context<'a, 'b, 'c, 'info, T>,
        accounts: &'a [PackedCompressedAccountWithMerkleContext],
    ) -> Result<Self> {
        let light_accounts = U::try_light_accounts(accounts)?;
        Ok(Self {
            anchor_context,
            light_accounts,
            // inputs,
        })
    }

    // pub fn verify(&mut self, accounts: &[LightAccount<T>]) -> Result<()>
    // where
    //     T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
    // {
    //     let bump = Pubkey::find_program_address(
    //         &[CPI_AUTHORITY_PDA_SEED],
    //         &self.anchor_context.accounts.get_invoking_program().key(),
    //     )
    //     .1;
    //     let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];

    //     // let new_address_params = self.light_accounts.new_address_params();
    //     // let new_address_params = self.inputs.new_addresses.clone().unwrap_or(Vec::new());
    //     // let input_compressed_accounts_with_merkle_context =
    //     //     self.inputs.accounts.clone().unwrap_or(Vec::new());
    //     let output_compressed_accounts = self.light_accounts.output_accounts()?;

    //     let instruction = InstructionDataInvokeCpi {
    //         proof: self.inputs.proof.as_ref().map(|proof| proof.proof.clone()),
    //         new_address_params,
    //         relay_fee: None,
    //         input_compressed_accounts_with_merkle_context,
    //         output_compressed_accounts,
    //         compress_or_decompress_lamports: None,
    //         is_compress: false,
    //         cpi_context: None,
    //     };

    //     verify(
    //         &self.anchor_context,
    //         &instruction,
    //         &[signer_seeds.as_slice()],
    //     )?;

    //     Ok(())
    // }
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct LightInstructionInputs {
    pub proof: Option<ProofRpcResult>,
    pub accounts: Option<Vec<PackedCompressedAccountWithMerkleContext>>,
    pub new_addresses: Option<Vec<PackedNewAddressParams>>,
}
