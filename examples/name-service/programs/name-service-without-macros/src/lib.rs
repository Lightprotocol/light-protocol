use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::account_info::LightAccountInfo;
use light_sdk::{
    compressed_account::{LightAccount, LightAccounts},
    instruction_data::LightInstructionData,
    light_system_accounts,
    verify::verify_compressed_accounts,
    LightDiscriminator, LightHasher, LightTraits,
};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {
    use light_hasher::Discriminator;
    use light_sdk::{
        address::derive_address, error::LightSdkError,
        program_merkle_context::unpack_address_merkle_context,
    };

    use super::*;

    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
        inputs: Vec<u8>,
        name: String,
        rdata: RData,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;

        let address_merkle_context = accounts[0]
            .address_merkle_context
            .ok_or(LightSdkError::ExpectedAddressParams)?;
        let address_merkle_context =
            unpack_address_merkle_context(address_merkle_context, ctx.remaining_accounts);
        let (address, address_seed) = derive_address(
            &[b"name-service", name.as_bytes()],
            &address_merkle_context,
            &crate::ID,
        );

        let account_infos: &[LightAccountInfo] = &[LightAccountInfo::from_meta(
            &accounts[0],
            Some(NameRecord::discriminator()),
            Some(address),
            Some(address_seed),
            &crate::ID,
        )?];

        let mut light_accounts = LightCreateRecord::try_light_accounts(account_infos)?;

        light_accounts.record.owner = ctx.accounts.signer.key();
        light_accounts.record.name = name;
        light_accounts.record.rdata = rdata;

        verify_compressed_accounts(
            &ctx,
            inputs.proof,
            &[light_accounts.record],
            None,
            false,
            None,
            &crate::ID,
        )?;

        Ok(())
    }

    pub fn update_record<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateRecord<'info>>,
        inputs: Vec<u8>,
        new_rdata: RData,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;
        let account_infos: &[LightAccountInfo] = &[LightAccountInfo::from_meta(
            &accounts[0],
            Some(NameRecord::discriminator()),
            None,
            None,
            &crate::ID,
        )?];

        let mut light_accounts = LightCreateRecord::try_light_accounts(account_infos)?;

        if light_accounts.record.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }

        light_accounts.record.rdata = new_rdata;

        verify_compressed_accounts(
            &ctx,
            inputs.proof,
            &[light_accounts.record],
            None,
            false,
            None,
            &crate::ID,
        )?;

        Ok(())
    }

    pub fn delete_record<'info>(
        ctx: Context<'_, '_, '_, 'info, DeleteRecord<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;
        let account_infos: &[LightAccountInfo] = &[LightAccountInfo::from_meta(
            &accounts[0],
            Some(NameRecord::discriminator()),
            None,
            None,
            &crate::ID,
        )?];

        let light_accounts = LightDeleteRecord::try_light_accounts(account_infos)?;

        if light_accounts.record.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }

        verify_compressed_accounts(
            &ctx,
            inputs.proof,
            &[light_accounts.record],
            None,
            false,
            None,
            &crate::ID,
        )?;

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
    fn try_light_accounts(accounts: &'a [LightAccountInfo]) -> Result<Self> {
        let record: LightAccount<NameRecord> = LightAccount::from_light_account_info(&accounts[0])?;
        Ok(Self { record })
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
    fn try_light_accounts(accounts: &'a [LightAccountInfo]) -> Result<Self> {
        let record: LightAccount<NameRecord> = LightAccount::from_light_account_info(&accounts[0])?;
        Ok(Self { record })
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
    fn try_light_accounts(accounts: &'a [LightAccountInfo]) -> Result<Self> {
        let record: LightAccount<NameRecord> = LightAccount::from_light_account_info(&accounts[0])?;
        Ok(Self { record })
    }
}
