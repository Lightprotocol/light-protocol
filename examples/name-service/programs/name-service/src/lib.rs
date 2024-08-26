use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::{
    address::derive_address_seed,
    compressed_account::{
        input_compressed_account, new_compressed_account, output_compressed_account, LightAccount,
    },
    context::LightContext,
    light_account, light_accounts, light_program,
    merkle_context::{PackedAddressMerkleContext, PackedMerkleContext, PackedMerkleOutputContext},
    program_merkle_context::unpack_address_merkle_context,
    utils::{
        create_cpi_inputs_for_account_deletion, create_cpi_inputs_for_account_update,
        create_cpi_inputs_for_new_account,
    },
    verify::verify,
    LightTraits,
};
use light_system_program::{invoke::processor::CompressedProof, sdk::CompressedCpiContext};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[light_program]
pub mod name_service {
    use super::*;

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
    pub fn update_record<'info>(
        ctx: LightContext<'_, '_, '_, 'info, UpdateRecord<'info>>,
        new_rdata: RData,
    ) -> Result<()> {
        ctx.light_accounts.record.rdata = new_rdata;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn delete_record<'info>(
        ctx: LightContext<'_, '_, '_, 'info, DeleteRecord<'info>>,
    ) -> Result<()> {
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

#[light_account]
#[derive(Clone, Debug)]
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

// #[derive(Accounts, LightTraits)]

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

    #[light_account(init, seeds = [b"name-service"])]
    pub record: LightAccount<NameRecord>,
}

// #[derive(Accounts, LightTraits)]

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

    #[light_account(mut, seeds = [b"name-service"])]
    pub record: LightAccount<NameRecord>,
}

// #[derive(Accounts, LightTraits)]

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

    #[light_account(close, seeds = [b"name-service"])]
    pub record: LightAccount<NameRecord>,
}
