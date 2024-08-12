use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::{prelude::*, solana_program::hash};
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::{errors::HasherError, DataHasher, Discriminator, Hasher, Poseidon};
use light_sdk::{
    light_accounts, utils::create_cpi_inputs_for_new_address, verify::verify, LightDiscriminator,
    LightTraits,
};
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        address::derive_address,
        compressed_account::{
            CompressedAccount, CompressedAccountData, PackedAddressMerkleContext,
            PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
        },
        CompressedCpiContext,
    },
    InstructionDataInvokeCpi, NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {
    use super::*;

    /// Creates a DNS record based on the given `name` and `rdata`.
    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, NameService<'info>>,
        proof: CompressedProof,
        merkle_tree_pubkey_index: u8,
        address_merkle_context: PackedAddressMerkleContext,
        name: String,
        rdata: RData,
        cpi_context: Option<CompressedCpiContext>,
    ) -> Result<()> {
        // Use the domain name as a seed for the address of the compressed
        // account.
        let seed = hash::hash(name.as_bytes()).to_bytes();
        let new_address_params = NewAddressParamsPacked {
            seed,
            address_merkle_context,
        };

        // Create a compressed account.
        let record = NameRecord {
            owner: ctx.accounts.signer.key(),
            name,
            rdata,
        };
        let compressed_account = create_compressed_account(
            &ctx,
            &record,
            merkle_tree_pubkey_index,
            &new_address_params,
        )?;

        // Find the seeds for CPI to light-system program.
        let signer_seed = b"cpi_signer".as_slice();
        let bump = Pubkey::find_program_address(&[signer_seed], &ctx.accounts.self_program.key()).1;
        let signer_seeds = [signer_seed, &[bump]];

        // Make a CPI call to light-system program, which effectively creates
        // the compressed account.
        let inputs = create_cpi_inputs_for_new_address(
            proof,
            new_address_params,
            compressed_account,
            &signer_seeds,
            cpi_context,
        );
        verify(ctx, &inputs, &[&signer_seeds])?;

        Ok(())
    }

    /// Updates the DNS record with the given `name` with `new_rdata` (which
    /// replaces `old_rdata`).
    pub fn update_record<'info>(
        ctx: Context<'_, '_, '_, 'info, NameService<'info>>,
        proof: CompressedProof,
        merkle_context: PackedMerkleContext,
        address: [u8; 32],
        name: String,
        old_rdata: RData,
        new_rdata: RData,
        cpi_context: Option<CompressedCpiContext>,
    ) -> Result<()> {
        let owner = ctx.accounts.signer.key();

        // Re-create the old compressed account. It's needed as an input for
        // validation and nullification.
        let old_record = NameRecord {
            owner,
            name: name.clone(),
            rdata: old_rdata,
        };
        let old_compressed_account =
            compressed_input_account_with_address(old_record, merkle_context, address)?;

        // Create a new compressed account with `new_rdata`.
        let new_record = NameRecord {
            owner,
            name,
            rdata: new_rdata,
        };
        let new_compressed_account = compressed_output_account_with_address(&new_record, address)?;

        // Find the seeds for CPI to light-system program.
        let signer_seed = b"cpi_signer".as_slice();
        let bump = Pubkey::find_program_address(&[signer_seed], &ctx.accounts.self_program.key()).1;
        let signer_seeds = [signer_seed, &[bump]];

        // Make a CPI call to light-system program, which effectively nullifies
        // the old compressed account and creates the new one.
        //
        // Update is done by specifying both input and output accounts.
        let inputs = InstructionDataInvokeCpi {
            proof: Some(proof),
            new_address_params: vec![],
            input_compressed_accounts_with_merkle_context: vec![old_compressed_account],
            output_compressed_accounts: vec![new_compressed_account],
            relay_fee: None,
            compress_or_decompress_lamports: None,
            is_compress: false,
            signer_seeds: signer_seeds.iter().map(|seed| seed.to_vec()).collect(),
            cpi_context,
        };
        verify(ctx, &inputs, &[&signer_seeds])?;

        Ok(())
    }

    /// Removes the DNS record with the given `name` and `rdata`.
    pub fn delete_record<'info>(
        ctx: Context<'_, '_, '_, 'info, NameService<'info>>,
        proof: CompressedProof,
        merkle_context: PackedMerkleContext,
        address: [u8; 32],
        name: String,
        rdata: RData,
        cpi_context: Option<CompressedCpiContext>,
    ) -> Result<()> {
        // Re-create the compressed account, which we are going to use as an input.
        let record = NameRecord {
            owner: ctx.accounts.signer.key(),
            name,
            rdata,
        };
        let compressed_account =
            compressed_input_account_with_address(record, merkle_context, address)?;

        // Find the seeds for CPI to light-system program.
        let signer_seed = b"cpi_signer".as_slice();
        let bump = Pubkey::find_program_address(&[signer_seed], &ctx.accounts.self_program.key()).1;
        let signer_seeds = [signer_seed, &[bump]];

        // Make a CPI call to light-system program, which effectively nullifies
        // the old compressed account and creates the new one.
        //
        // Update is done by specifying only an input account and no output
        // ones.
        let inputs = InstructionDataInvokeCpi {
            proof: Some(proof),
            new_address_params: vec![],
            input_compressed_accounts_with_merkle_context: vec![compressed_account],
            output_compressed_accounts: vec![],
            relay_fee: None,
            compress_or_decompress_lamports: None,
            is_compress: false,
            signer_seeds: signer_seeds.iter().map(|seed| seed.to_vec()).collect(),
            cpi_context,
        };
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

#[derive(Debug, BorshDeserialize, BorshSerialize, LightDiscriminator)]
pub struct NameRecord {
    pub owner: Pubkey,
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

impl light_hasher::DataHasher for NameRecord {
    fn hash<H: Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        let owner = hash_to_bn254_field_size_be(self.owner.to_bytes().as_slice())
            .unwrap()
            .0;
        H::hashv(&[&owner, self.name.as_bytes()])
    }
}

fn create_compressed_account(
    ctx: &Context<'_, '_, '_, '_, NameService<'_>>,
    record: &NameRecord,
    merkle_tree_index: u8,
    new_address_params: &NewAddressParamsPacked,
) -> Result<OutputCompressedAccountWithPackedContext> {
    let data = record.try_to_vec()?;
    let data_hash = record.hash::<Poseidon>().map_err(ProgramError::from)?;
    let compressed_account_data = CompressedAccountData {
        discriminator: NameRecord::discriminator(),
        data,
        data_hash,
    };
    let address = derive_address(
        &ctx.remaining_accounts[new_address_params
            .address_merkle_context
            .address_merkle_tree_pubkey_index as usize]
            .key(),
        &new_address_params.seed,
    )
    .map_err(|_| ProgramError::InvalidArgument)?;
    let compressed_account = CompressedAccount {
        owner: crate::ID,
        lamports: 0,
        address: Some(address),
        data: Some(compressed_account_data),
    };

    Ok(OutputCompressedAccountWithPackedContext {
        compressed_account,
        merkle_tree_index,
    })
}

fn compressed_input_account_with_address(
    record: NameRecord,
    merkle_context: PackedMerkleContext,
    address: [u8; 32],
) -> Result<PackedCompressedAccountWithMerkleContext> {
    let data = record.try_to_vec()?;
    let data_hash = record.hash::<Poseidon>().map_err(ProgramError::from)?;
    let compressed_account_data = CompressedAccountData {
        discriminator: NameRecord::discriminator(),
        data,
        data_hash,
    };
    let compressed_account = CompressedAccount {
        owner: crate::ID,
        lamports: 0,
        address: Some(address),
        data: Some(compressed_account_data),
    };

    Ok(PackedCompressedAccountWithMerkleContext {
        compressed_account,
        merkle_context,
    })
}

fn compressed_output_account_with_address(
    record: &NameRecord,
    address: [u8; 32],
) -> Result<OutputCompressedAccountWithPackedContext> {
    let data = record.try_to_vec()?;
    let data_hash = record.hash::<Poseidon>().map_err(ProgramError::from)?;
    let compressed_account_data = CompressedAccountData {
        discriminator: NameRecord::discriminator(),
        data,
        data_hash,
    };
    let compressed_account = CompressedAccount {
        owner: crate::ID,
        lamports: 0,
        address: Some(address),
        data: Some(compressed_account_data),
    };

    Ok(OutputCompressedAccountWithPackedContext {
        compressed_account,
        merkle_tree_index: 0,
    })
}
