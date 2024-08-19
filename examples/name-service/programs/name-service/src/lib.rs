use std::net::{Ipv4Addr, Ipv6Addr};

use anchor_lang::{prelude::*, solana_program::hash};
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::{bytes::AsByteVec, errors::HasherError, DataHasher, Discriminator, Poseidon};
use light_sdk::{
    light_accounts,
    merkle_context::{PackedAddressMerkleContext, PackedMerkleContext, PackedMerkleOutputContext},
    utils::create_cpi_inputs_for_new_address,
    verify::verify,
    LightDiscriminator, LightHasher, LightTraits,
};
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        address::derive_address,
        compressed_account::{
            CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        },
        CompressedCpiContext,
    },
    InstructionDataInvokeCpi, NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {
    use super::*;

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
        let address_seed = hash::hash(name.as_bytes()).to_bytes();

        let record = NameRecord {
            owner: ctx.accounts.signer.key(),
            name,
            rdata,
        };
        let (compressed_account, new_address_params) = create_compressed_account(
            &ctx,
            &record,
            address_seed,
            &merkle_output_context,
            &address_merkle_context,
            address_merkle_tree_root_index,
        )?;

        let signer_seed = b"cpi_signer".as_slice();
        let bump = Pubkey::find_program_address(&[signer_seed], &ctx.accounts.self_program.key()).1;
        let signer_seeds = [signer_seed, &[bump]];

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

    pub fn update_record<'info>(
        ctx: Context<'_, '_, '_, 'info, NameService<'info>>,
        proof: CompressedProof,
        merkle_context: PackedMerkleContext,
        merkle_tree_root_index: u16,
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
        let old_compressed_account = compressed_input_account_with_address(
            old_record,
            address,
            &merkle_context,
            merkle_tree_root_index,
        )?;

        let new_record = NameRecord {
            owner,
            name,
            rdata: new_rdata,
        };
        let new_compressed_account =
            compressed_output_account_with_address(&new_record, address, &merkle_context)?;

        let signer_seed = b"cpi_signer".as_slice();
        let bump = Pubkey::find_program_address(&[signer_seed], &ctx.accounts.self_program.key()).1;
        let signer_seeds = [signer_seed, &[bump]];

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

    pub fn delete_record<'info>(
        ctx: Context<'_, '_, '_, 'info, NameService<'info>>,
        proof: CompressedProof,
        merkle_context: PackedMerkleContext,
        merkle_tree_root_index: u16,
        address: [u8; 32],
        name: String,
        rdata: RData,
        cpi_context: Option<CompressedCpiContext>,
    ) -> Result<()> {
        let record = NameRecord {
            owner: ctx.accounts.signer.key(),
            name,
            rdata,
        };
        let compressed_account = compressed_input_account_with_address(
            record,
            address,
            &merkle_context,
            merkle_tree_root_index,
        )?;

        let signer_seed = b"cpi_signer".as_slice();
        let bump = Pubkey::find_program_address(&[signer_seed], &ctx.accounts.self_program.key()).1;
        let signer_seeds = [signer_seed, &[bump]];

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

impl AsByteVec for RData {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        match self {
            Self::A(ipv4_addr) => vec![ipv4_addr.octets().to_vec()],
            Self::AAAA(ipv6_addr) => vec![ipv6_addr.octets().to_vec()],
            Self::CName(cname) => cname.as_byte_vec(),
        }
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize, LightDiscriminator, LightHasher)]
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

fn create_compressed_account(
    ctx: &Context<'_, '_, '_, '_, NameService<'_>>,
    record: &NameRecord,
    address_seed: [u8; 32],
    merkle_output_context: &PackedMerkleOutputContext,
    address_merkle_context: &PackedAddressMerkleContext,
    address_merkle_tree_root_index: u16,
) -> Result<(
    OutputCompressedAccountWithPackedContext,
    NewAddressParamsPacked,
)> {
    let data = record.try_to_vec()?;
    let data_hash = record.hash::<Poseidon>().map_err(ProgramError::from)?;
    let compressed_account_data = CompressedAccountData {
        discriminator: NameRecord::discriminator(),
        data,
        data_hash,
    };
    let address = derive_address(
        &ctx.remaining_accounts[address_merkle_context.address_merkle_tree_pubkey_index as usize]
            .key(),
        &address_seed,
    )
    .map_err(|_| ProgramError::InvalidArgument)?;
    let compressed_account = CompressedAccount {
        owner: crate::ID,
        lamports: 0,
        address: Some(address),
        data: Some(compressed_account_data),
    };
    let compressed_account = OutputCompressedAccountWithPackedContext {
        compressed_account,
        merkle_tree_index: merkle_output_context.merkle_tree_pubkey_index,
    };

    let new_address_params = NewAddressParamsPacked {
        seed: address_seed,
        address_merkle_tree_account_index: address_merkle_context.address_merkle_tree_pubkey_index,
        address_queue_account_index: address_merkle_context.address_queue_pubkey_index,
        address_merkle_tree_root_index,
    };

    Ok((compressed_account, new_address_params))
}

fn compressed_input_account_with_address(
    record: NameRecord,
    address: [u8; 32],
    merkle_context: &PackedMerkleContext,
    merkle_tree_root_index: u16,
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
        merkle_context: *merkle_context,
        root_index: merkle_tree_root_index,
        read_only: false,
    })
}

fn compressed_output_account_with_address(
    record: &NameRecord,
    address: [u8; 32],
    merkle_context: &PackedMerkleContext,
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
        merkle_tree_index: merkle_context.merkle_tree_pubkey_index,
    })
}
