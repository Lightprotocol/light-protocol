use std::{
    io::Cursor,
    ops::{Deref, DerefMut},
};

use anchor_lang::{context::Context, AnchorDeserialize, Bumps, Result};

use crate::{
    address::PackedNewAddressParams,
    compressed_account::{LightAccounts, PackedCompressedAccountWithMerkleContext},
    proof::ProofRpcResult,
    traits::{
        InvokeAccounts, InvokeCpiAccounts, InvokeCpiContextAccount, LightSystemAccount,
        SignerAccounts,
    },
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
        accounts: &'a Option<Vec<PackedCompressedAccountWithMerkleContext>>,
    ) -> Result<Self> {
        let light_accounts = U::try_light_accounts(accounts)?;
        Ok(Self {
            anchor_context,
            light_accounts,
        })
    }
}

pub fn parse_light_inputs(
    inputs: Vec<u8>,
) -> Result<(
    Option<ProofRpcResult>,
    Option<Vec<PackedCompressedAccountWithMerkleContext>>,
    Option<Vec<PackedNewAddressParams>>,
)> {
    let mut inputs = Cursor::new(inputs);
    let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
    let accounts =
        Option::<Vec<PackedCompressedAccountWithMerkleContext>>::deserialize_reader(&mut inputs)?;
    let new_addresses = Option::<Vec<PackedNewAddressParams>>::deserialize_reader(&mut inputs)?;

    Ok((proof, accounts, new_addresses))
}
