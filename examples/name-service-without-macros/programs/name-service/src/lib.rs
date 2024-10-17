use std::io::Cursor;
use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::{
    address::PackedNewAddressParams,
    compressed_account::{
        LightAccount, LightAccounts, OutputCompressedAccountWithPackedContext,
        PackedCompressedAccountWithMerkleContext,
    },
    context::{LightContext, LightInstructionInputs},
    light_system_accounts,
    proof::ProofRpcResult,
    LightDiscriminator, LightHasher, LightTraits,
};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {
    use light_sdk::verify::verify_compressed_accounts;

    use super::*;

    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let mut inputs = Cursor::new(inputs);
        let user_inputs = CreateRecordInputs::deserialize_reader(&mut inputs)?;
        let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
        let accounts = Option::<Vec<PackedCompressedAccountWithMerkleContext>>::deserialize_reader(
            &mut inputs,
        )?;
        let new_addresses = Option::<Vec<PackedNewAddressParams>>::deserialize_reader(&mut inputs)?;

        // TODO: handle the error
        let accounts = accounts.unwrap();

        let mut light_accounts = LightCreateRecord::try_light_accounts(&accounts)?;

        light_accounts.record.owner = ctx.accounts.signer.key();
        light_accounts.record.name = user_inputs.name;
        light_accounts.record.rdata = user_inputs.rdata;

        verify_compressed_accounts(&ctx, &[light_accounts.record])?;

        Ok(())
    }

    pub fn update_record<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateRecord<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let mut inputs = Cursor::new(inputs);
        let user_inputs = UpdateRecordInputs::deserialize_reader(&mut inputs)?;
        let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
        let accounts = Option::<Vec<PackedCompressedAccountWithMerkleContext>>::deserialize_reader(
            &mut inputs,
        )?;
        let new_addresses = Option::<Vec<PackedNewAddressParams>>::deserialize_reader(&mut inputs)?;

        // TODO: handle the error
        let accounts = accounts.unwrap();

        let mut light_accounts = LightUpdateRecord::try_light_accounts(&accounts)?;

        light_accounts.record.rdata = user_inputs.new_rdata;

        verify_compressed_accounts(&ctx, &[light_accounts.record])?;

        Ok(())
    }

    pub fn delete_record<'info>(
        ctx: Context<'_, '_, '_, 'info, DeleteRecord<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let mut inputs = Cursor::new(inputs);
        let user_inputs = DeleteRecordInputs::deserialize_reader(&mut inputs)?;
        let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
        let accounts = Option::<Vec<PackedCompressedAccountWithMerkleContext>>::deserialize_reader(
            &mut inputs,
        )?;
        let new_addresses = Option::<Vec<PackedNewAddressParams>>::deserialize_reader(&mut inputs)?;

        // TODO: handle the error
        let accounts = accounts.unwrap();

        let mut light_accounts = LightDeleteRecord::try_light_accounts(&accounts)?;

        verify_compressed_accounts(&ctx, &[light_accounts.record])?;

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
    pub owner: Pubkey,
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
        accounts: &'a [PackedCompressedAccountWithMerkleContext],
    ) -> Result<Self> {
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

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct CreateRecordInputs {
    pub light_inputs: LightInstructionInputs,
    pub name: String,
    pub rdata: RData,
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
        accounts: &'a [PackedCompressedAccountWithMerkleContext],
    ) -> Result<Self> {
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

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct UpdateRecordInputs {
    pub light_inputs: LightInstructionInputs,
    pub new_rdata: RData,
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
        accounts: &'a [PackedCompressedAccountWithMerkleContext],
    ) -> Result<Self> {
        let record: LightAccount<NameRecord> = LightAccount::try_from_slice_mut(&accounts[0])?;
        Ok(Self { record })
    }

    fn output_accounts(&self) -> Result<Vec<OutputCompressedAccountWithPackedContext>> {
        Ok(Vec::new())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct DeleteRecordInputs {
    pub light_inputs: LightInstructionInputs,
}
