use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::{
    compressed_account::LightAccount, light_account, light_accounts, light_program,
    merkle_context::PackedAddressMerkleContext,
};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[light_program]
pub mod name_service {
    use super::*;

    pub fn create_record<'info>(
        ctx: LightContext<'_, '_, '_, 'info, CreateRecord<'info>>,
        name: String,
        rdata: RData,
    ) -> Result<()> {
        ctx.light_accounts.record.owner = ctx.accounts.signer.key();
        ctx.light_accounts.record.name = name;
        ctx.light_accounts.record.rdata = rdata;

        Ok(())
    }

    pub fn update_record<'info>(
        ctx: LightContext<'_, '_, '_, 'info, UpdateRecord<'info>>,
        new_rdata: RData,
    ) -> Result<()> {
        if ctx.light_accounts.record.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }
        ctx.light_accounts.record.rdata = new_rdata;

        Ok(())
    }

    pub fn delete_record<'info>(
        ctx: LightContext<'_, '_, '_, 'info, DeleteRecord<'info>>,
    ) -> Result<()> {
        if ctx.light_accounts.record.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }
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

#[light_account]
#[derive(Clone, Debug, Default)]
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
    #[msg("Record account has no data")]
    NoData,
    #[msg("Provided data hash does not match the computed hash")]
    InvalidDataHash,
}

#[light_accounts]
pub struct CreateRecord<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,

    #[light_account(init, seeds = [b"name-service", record.name.as_bytes()])]
    pub record: LightAccount<NameRecord>,
}

#[light_accounts]
pub struct UpdateRecord<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,

    #[light_account(mut, seeds = [b"name-service", record.name.as_bytes()])]
    pub record: LightAccount<NameRecord>,
}

#[light_accounts]
pub struct DeleteRecord<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,

    #[light_account(close, seeds = [b"name-service", record.name.as_bytes()])]
    pub record: LightAccount<NameRecord>,
}
