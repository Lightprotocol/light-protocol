use std::io::Cursor;
use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::error::LightSdkError;
use light_sdk::verify::verify_compressed_accounts;
use light_sdk::{
    address::PackedNewAddressParams,
    compressed_account::{
        LightAccount, LightAccounts, OutputCompressedAccountWithPackedContext,
        PackedCompressedAccountWithMerkleContext,
    },
    light_system_accounts,
    proof::ProofRpcResult,
    LightDiscriminator, LightHasher, LightTraits,
};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {

    use light_sdk::inputs::LightInputs;

    use super::*;

    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
        inputs: Vec<u8>,
        name: String,
        rdata: RData,
    ) -> Result<()> {
        let inputs = LightInputs::serialize(&inputs)?;
        let mut light_accounts = LightCreateRecord::try_light_accounts(&inputs.accounts)?;

        light_accounts.record.owner = ctx.accounts.signer.key();
        light_accounts.record.name = name;
        light_accounts.record.rdata = rdata;

        verify_compressed_accounts(
            &ctx,
            inputs.proof,
            &[light_accounts.record],
            &inputs.new_addresses.unwrap(),
            None,
            None,
        )?;

        Ok(())
    }

    pub fn update_record<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateRecord<'info>>,
        inputs: Vec<u8>,
        new_rdata: RData,
    ) -> Result<()> {
        let mut inputs = Cursor::new(inputs);
        let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
        let accounts = Option::<Vec<PackedCompressedAccountWithMerkleContext>>::deserialize_reader(
            &mut inputs,
        )?;
        let _new_addresses =
            Option::<Vec<PackedNewAddressParams>>::deserialize_reader(&mut inputs)?;
        let mut light_accounts = LightUpdateRecord::try_light_accounts(&accounts)?;

        if light_accounts.record.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }

        light_accounts.record.rdata = new_rdata;

        verify_compressed_accounts(&ctx, proof, &[light_accounts.record], &[], None, None)?;

        Ok(())
    }

    pub fn delete_record<'info>(
        ctx: Context<'_, '_, '_, 'info, DeleteRecord<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let mut inputs = Cursor::new(inputs);
        let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
        let accounts = Option::<Vec<PackedCompressedAccountWithMerkleContext>>::deserialize_reader(
            &mut inputs,
        )?;
        let _new_addresses =
            Option::<Vec<PackedNewAddressParams>>::deserialize_reader(&mut inputs)?;
        let light_accounts = LightDeleteRecord::try_light_accounts(&accounts)?;

        if light_accounts.record.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }

        verify_compressed_accounts(&ctx, proof, &[light_accounts.record], &[], None, None)?;

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum RData {
    A(Ipv4Addr),
    AAAA(Ipv6Addr),
    CName(String),
}

impl anchor_lang::IdlBuild for RData {}

impl AsByteVec for RData {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        match self {
            Self::A(ipv4_addr) => vec![ipv4_addr.octets().to_vec()],
            Self::AAAA(ipv6_addr) => vec![ipv6_addr.octets().to_vec()],
            Self::CName(cname) => cname.as_byte_vec(),
        }
    }
}

impl Default for RData {
    fn default() -> Self {
        Self::A(Ipv4Addr::new(127, 0, 0, 1))
    }
}

#[derive(
    Clone, Debug, Default, AnchorDeserialize, AnchorSerialize, LightDiscriminator, LightHasher,
)]
pub struct NameRecord {
    #[truncate]
    pub owner: Pubkey,
    #[truncate]
    pub name: String,
    pub rdata: RData,
}

#[error_code]
pub enum CustomError {
    #[msg("No authority to perform this action")]
    Unauthorized,
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct CreateRecord<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}

pub struct LightCreateRecord<'a> {
    pub record: LightAccount<'a, NameRecord>,
}

impl<'a> LightAccounts<'a> for LightCreateRecord<'a> {
    fn try_light_accounts(
        accounts: &'a Option<Vec<PackedCompressedAccountWithMerkleContext>>,
    ) -> Result<Self> {
        let accounts = accounts.as_ref().ok_or(LightSdkError::ExpectedAccounts)?;
        let record: LightAccount<NameRecord> =
            LightAccount::new_init(&accounts[0].compressed_account);
        Ok(Self { record })
    }

    fn output_accounts(&self) -> Result<Vec<OutputCompressedAccountWithPackedContext>> {
        let mut output_accounts = Vec::with_capacity(1);
        if let Some(record_output_account) = self.record.output_compressed_account()? {
            output_accounts.push(record_output_account);
        }
        Ok(output_accounts)
    }
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct UpdateRecord<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}

pub struct LightUpdateRecord<'a> {
    pub record: LightAccount<'a, NameRecord>,
}

impl<'a> LightAccounts<'a> for LightUpdateRecord<'a> {
    fn try_light_accounts(
        accounts: &'a Option<Vec<PackedCompressedAccountWithMerkleContext>>,
    ) -> Result<Self> {
        let accounts = accounts.as_ref().ok_or(LightSdkError::ExpectedAccounts)?;
        let record: LightAccount<NameRecord> = LightAccount::try_from_slice_mut(&accounts[0])?;
        Ok(Self { record })
    }

    fn output_accounts(&self) -> Result<Vec<OutputCompressedAccountWithPackedContext>> {
        let mut output_accounts = Vec::with_capacity(1);
        if let Some(record_output_account) = self.record.output_compressed_account()? {
            output_accounts.push(record_output_account);
        }
        Ok(output_accounts)
    }
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct DeleteRecord<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}

pub struct LightDeleteRecord<'a> {
    pub record: LightAccount<'a, NameRecord>,
}

impl<'a> LightAccounts<'a> for LightDeleteRecord<'a> {
    fn try_light_accounts(
        accounts: &'a Option<Vec<PackedCompressedAccountWithMerkleContext>>,
    ) -> Result<Self> {
        let accounts = accounts.as_ref().ok_or(LightSdkError::ExpectedAccounts)?;
        let record: LightAccount<NameRecord> = LightAccount::try_from_slice_mut(&accounts[0])?;
        Ok(Self { record })
    }

    fn output_accounts(&self) -> Result<Vec<OutputCompressedAccountWithPackedContext>> {
        Ok(Vec::new())
    }
}
