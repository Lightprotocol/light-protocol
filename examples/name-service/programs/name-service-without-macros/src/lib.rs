use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::{
    account::LightAccount, instruction_data::PackedLightInstructionData, verify::verify_light_accounts,
    LightDiscriminator, LightHasher,
};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {
    use light_hasher::Discriminator;
    use light_sdk::{
        address::derive_address, error::LightSdkError,
        program_merkle_context::unpack_address_merkle_context, system_accounts::LightCpiAccounts,
    };

    use super::*;

    pub fn create_record<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, CreateRecord<'info>>,
        inputs: Vec<u8>,
        name: String,
        rdata: RData,
    ) -> Result<()> {
        let inputs = PackedLightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;

        // msg!("accounts: {:#?}", accounts);
        let address_merkle_context = accounts[0]
            .address_merkle_context
            .ok_or(LightSdkError::ExpectedAddressMerkleContext)?;
        let address_merkle_context =
            unpack_address_merkle_context(address_merkle_context, ctx.remaining_accounts);
        let (address, address_seed) = derive_address(
            &[b"name-service", name.as_bytes()],
            &address_merkle_context,
            &crate::ID,
        );

        let mut record: LightAccount<'_, NameRecord> = LightAccount::from_meta_init(
            &accounts[0],
            NameRecord::discriminator(),
            address,
            address_seed,
            &crate::ID,
        )?;

        record.owner = ctx.accounts.signer.key();
        record.name = name;
        record.rdata = rdata;
        // msg!("remaining accounts: {:#?}", ctx.remaining_accounts);

        // adds into total accounts
        let light_cpi_accounts = LightCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.cpi_signer.as_ref(),
            ctx.remaining_accounts,
        );
        msg!("fee payer: {:?}", ctx.accounts.signer);
        msg!("authority: {:?}", ctx.accounts.cpi_signer);
        verify_light_accounts(
            &light_cpi_accounts,
            inputs.proof,
            &[record],
            None,
            false,
            None,
        )?;

        Ok(())
    }

    pub fn update_record<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateRecord<'info>>,
        inputs: Vec<u8>,
        new_rdata: RData,
    ) -> Result<()> {
        // Deserialize the Light Protocol related data.
        let inputs = PackedLightInstructionData::deserialize(&inputs)?;
        // Require accounts to be provided.
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;

        // Convert `PackedLightAccountMeta` to `LightAccount`.
        let mut record: LightAccount<'_, NameRecord> =
            LightAccount::from_meta_mut(&accounts[0], NameRecord::discriminator(), &crate::ID)?;

        // Check the ownership of the `record`.
        if record.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }

        record.rdata = new_rdata;

        let light_cpi_accounts = LightCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.cpi_signer.as_ref(),
            ctx.remaining_accounts,
        );
        verify_light_accounts(
            &light_cpi_accounts,
            inputs.proof,
            &[record],
            None,
            false,
            None,
        )?;

        Ok(())
    }

    pub fn delete_record<'info>(
        ctx: Context<'_, '_, '_, 'info, DeleteRecord<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs = PackedLightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;

        let record: LightAccount<'_, NameRecord> =
            LightAccount::from_meta_close(&accounts[0], NameRecord::discriminator(), &crate::ID)?;

        if record.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }

        let light_cpi_accounts = LightCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.cpi_signer.as_ref(),
            ctx.remaining_accounts,
        );
        verify_light_accounts(
            &light_cpi_accounts,
            inputs.proof,
            &[record],
            None,
            false,
            None,
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

#[derive(Accounts)]
pub struct CreateRecord<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK: Checked in light-system-program.
    pub cpi_signer: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UpdateRecord<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK: Checked in light-system-program.
    pub cpi_signer: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct DeleteRecord<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK: Checked in light-system-program.
    pub cpi_signer: AccountInfo<'info>,
}
