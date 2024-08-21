use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::{
    light_account, light_accounts,
    merkle_context::{PackedAddressMerkleContext, PackedMerkleContext, PackedMerkleOutputContext},
    utils::{create_cpi_inputs_for_account_deletion, create_cpi_inputs_for_new_account},
    verify::verify,
    LightTraits,
};
use light_system_program::{invoke::processor::CompressedProof, sdk::CompressedCpiContext};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {
    use light_sdk::{
        address::derive_address_seed,
        compressed_account::{
            input_compressed_account, new_compressed_account, output_compressed_account,
        },
        merkle_context::unpack_address_merkle_context,
        utils::create_cpi_inputs_for_account_update,
    };

    use super::*;

    #[allow(clippy::too_many_arguments)]
    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, NameService<'info>>,
        proof: CompressedProof,
        merkle_output_context: PackedMerkleOutputContext,
        address_merkle_context: PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
        name: String,
        rdata: RData,
        cpi_context: Option<CompressedCpiContext>,
    ) -> Result<()> {
        let unpacked_address_merkle_context =
            unpack_address_merkle_context(address_merkle_context, ctx.remaining_accounts);
        let address_seed = derive_address_seed(
            &[
                ctx.accounts.signer.key.to_bytes().as_slice(),
                name.as_bytes(),
            ],
            &crate::ID,
            &unpacked_address_merkle_context,
        );

        let record = NameRecord {
            owner: ctx.accounts.signer.key(),
            name,
            rdata,
        };
        let (compressed_account, new_address_params) = new_compressed_account(
            &record,
            &address_seed,
            &crate::ID,
            &merkle_output_context,
            &address_merkle_context,
            address_merkle_tree_root_index,
            ctx.remaining_accounts,
        )?;

        let signer_seed = b"cpi_signer".as_slice();
        let bump = Pubkey::find_program_address(&[signer_seed], &ctx.accounts.self_program.key()).1;
        let signer_seeds = [signer_seed, &[bump]];

        let inputs = create_cpi_inputs_for_new_account(
            proof,
            new_address_params,
            compressed_account,
            &signer_seeds,
            cpi_context,
        );

        verify(ctx, &inputs, &[&signer_seeds])?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_record<'info>(
        ctx: Context<'_, '_, '_, 'info, NameService<'info>>,
        proof: CompressedProof,
        merkle_context: PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: PackedAddressMerkleContext,
        name: String,
        old_rdata: RData,
        new_rdata: RData,
        cpi_context: Option<CompressedCpiContext>,
    ) -> Result<()> {
        let unpacked_address_merkle_context =
            unpack_address_merkle_context(address_merkle_context, ctx.remaining_accounts);
        let address_seed = derive_address_seed(
            &[
                ctx.accounts.signer.key.to_bytes().as_slice(),
                name.as_bytes(),
            ],
            &crate::ID,
            &unpacked_address_merkle_context,
        );

        let owner = ctx.accounts.signer.key();

        // Re-create the old compressed account. It's needed as an input for
        // validation and nullification.
        let old_record = NameRecord {
            owner,
            name: name.clone(),
            rdata: old_rdata,
        };
        let old_compressed_account = input_compressed_account(
            &old_record,
            &address_seed,
            &crate::ID,
            &merkle_context,
            merkle_tree_root_index,
            &address_merkle_context,
            ctx.remaining_accounts,
        )?;

        let new_record = NameRecord {
            owner,
            name,
            rdata: new_rdata,
        };
        let new_compressed_account = output_compressed_account(
            &new_record,
            &address_seed,
            &crate::ID,
            &merkle_context,
            &address_merkle_context,
            ctx.remaining_accounts,
        )?;

        let signer_seed = b"cpi_signer".as_slice();
        let bump = Pubkey::find_program_address(&[signer_seed], &ctx.accounts.self_program.key()).1;
        let signer_seeds = [signer_seed, &[bump]];

        let inputs = create_cpi_inputs_for_account_update(
            proof,
            old_compressed_account,
            new_compressed_account,
            &signer_seeds,
            cpi_context,
        );

        verify(ctx, &inputs, &[&signer_seeds])?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn delete_record<'info>(
        ctx: Context<'_, '_, '_, 'info, NameService<'info>>,
        proof: CompressedProof,
        merkle_context: PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: PackedAddressMerkleContext,
        name: String,
        rdata: RData,
        cpi_context: Option<CompressedCpiContext>,
    ) -> Result<()> {
        let unpacked_address_merkle_context =
            unpack_address_merkle_context(address_merkle_context, ctx.remaining_accounts);
        let address_seed = derive_address_seed(
            &[
                ctx.accounts.signer.key.to_bytes().as_slice(),
                name.as_bytes(),
            ],
            &crate::ID,
            &unpacked_address_merkle_context,
        );

        let record = NameRecord {
            owner: ctx.accounts.signer.key(),
            name,
            rdata,
        };
        let compressed_account = input_compressed_account(
            &record,
            &address_seed,
            &crate::ID,
            &merkle_context,
            merkle_tree_root_index,
            &address_merkle_context,
            ctx.remaining_accounts,
        )?;

        let signer_seed = b"cpi_signer".as_slice();
        let bump = Pubkey::find_program_address(&[signer_seed], &ctx.accounts.self_program.key()).1;
        let signer_seeds = [signer_seed, &[bump]];

        let inputs = create_cpi_inputs_for_account_deletion(
            proof,
            compressed_account,
            &signer_seeds,
            cpi_context,
        );

        verify(ctx, &inputs, &[&signer_seeds])?;

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
#[derive(Debug)]
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
#[derive(Accounts, LightTraits)]
pub struct NameService<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}
