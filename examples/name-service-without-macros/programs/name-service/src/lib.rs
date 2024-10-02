use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::{
    compressed_account::{LightAccount, LightAccounts, OutputCompressedAccountWithPackedContext},
    context::LightInstructionInputs,
    light_system_accounts, LightDiscriminator, LightHasher, LightTraits,
};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {
    use light_sdk::context::LightContext;

    use super::*;

    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs = CreateRecordInputs::deserialize(&mut inputs.as_slice())?;
        let mut ctx =
            LightContext::<CreateRecord, LightCreateRecord>::new(ctx, inputs.light_inputs)?;

        ctx.light_accounts.record.owner = ctx.accounts.signer.key();
        ctx.light_accounts.record.name = inputs.name;
        ctx.light_accounts.record.rdata = inputs.rdata;

        ctx.verify()?;

        Ok(())
    }

    pub fn update_record<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateRecord<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs = UpdateRecordInputs::deserialize(&mut inputs.as_slice())?;
        let mut ctx =
            LightContext::<UpdateRecord, LightUpdateRecord>::new(ctx, inputs.light_inputs)?;

        ctx.light_accounts.record.rdata = inputs.new_rdata;

        ctx.verify()?;

        Ok(())
    }

    pub fn delete_record<'info>(
        ctx: Context<'_, '_, '_, 'info, DeleteRecord<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs = DeleteRecordInputs::deserialize(&mut inputs.as_slice())?;
        let mut ctx =
            LightContext::<DeleteRecord, LightDeleteRecord>::new(ctx, inputs.light_inputs)?;

        ctx.verify()?;

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

pub struct LightCreateRecord {
    pub record: LightAccount<NameRecord>,
}

impl LightAccounts for LightCreateRecord {
    fn try_light_accounts(inputs: &LightInstructionInputs) -> Result<Self> {
        let record: LightAccount<NameRecord> =
            LightAccount::new_init(&inputs.accounts.as_ref().unwrap()[0].compressed_account);
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

pub struct LightUpdateRecord {
    pub record: LightAccount<NameRecord>,
}

impl LightAccounts for LightUpdateRecord {
    fn try_light_accounts(inputs: &LightInstructionInputs) -> Result<Self> {
        let record: LightAccount<NameRecord> = LightAccount::try_from_slice_mut(
            &inputs.accounts.as_ref().unwrap()[0].compressed_account,
        )?;
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

pub struct LightDeleteRecord {
    pub record: LightAccount<NameRecord>,
}

impl LightAccounts for LightDeleteRecord {
    fn try_light_accounts(inputs: &LightInstructionInputs) -> Result<Self> {
        let record: LightAccount<NameRecord> = LightAccount::try_from_slice_mut(
            &inputs.accounts.as_ref().unwrap()[0].compressed_account,
        )?;
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
