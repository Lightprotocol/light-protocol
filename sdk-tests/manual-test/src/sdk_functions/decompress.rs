//! SDK generic decompression functions.
//!
//! These functions are generic over account types and can be reused by the macro.
//! The decompress flow creates PDAs from compressed state (needs validity proof, packed data, seeds).

use anchor_lang::prelude::*;
use light_compressed_account::{
    address::derive_address,
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_hasher::{Hasher, Sha256};
use light_sdk::{
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    instruction::ValidityProof,
    interface::{create_pda_account, LightConfig},
    light_account_checks::{account_iterator::AccountIterator, checks::check_data_is_zeroed},
    LightDiscriminator,
};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use light_sdk_types::CpiSigner;
use solana_program::clock::Clock;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program_error::ProgramError;

use crate::traits::{LightAccount, LightAccountVariant, PackedLightAccountVariant};

// ============================================================================
// DecompressVariant Trait (implemented by program's PackedProgramAccountVariant)
// ============================================================================

/// Trait for packed program account variants that support decompression.
///
/// This trait is implemented by the program's `PackedProgramAccountVariant` enum
/// to handle type-specific dispatch during decompression.
///
/// MACRO-GENERATED: The implementation contains a match statement routing each
/// enum variant to the appropriate `prepare_account_for_decompression` call.
pub trait DecompressVariant<'info>: AnchorSerialize + AnchorDeserialize + Clone {
    /// Decompress this variant into a PDA account.
    ///
    /// The implementation should match on the enum variant and call
    /// `prepare_account_for_decompression::<SEED_COUNT, PackedVariantType>(packed, pda_account, ctx)`.
    fn decompress(
        &self,
        pda_account: &AccountInfo<'info>,
        ctx: &mut DecompressCtx<'_, 'info>,
    ) -> std::result::Result<(), ProgramError>;
}

// ============================================================================
// Parameters and Context
// ============================================================================

/// Parameters for decompress_idempotent instruction.
/// Generic over the variant type - each program defines its own `PackedProgramAccountVariant`.
///
/// Field order matches `LoadAccountsData` from light-client for compatibility.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DecompressIdempotentParams<V>
where
    V: AnchorSerialize + AnchorDeserialize + Clone,
{
    /// Validity proof for compressed account verification
    pub proof: ValidityProof,
    /// Accounts to decompress - each variant contains packed data and metadata
    pub accounts: Vec<V>,
    /// Offset into remaining_accounts where Light system accounts begin
    pub system_accounts_offset: u8,
}

/// Context struct holding all data needed for decompression.
/// Contains internal vec for collecting CompressedAccountInfo results.
pub struct DecompressCtx<'a, 'info> {
    pub program_id: &'a Pubkey,
    pub cpi_accounts: &'a CpiAccounts<'a, 'info>,
    pub remaining_accounts: &'a [AccountInfo<'info>],
    pub rent_sponsor: &'a AccountInfo<'info>,
    pub light_config: &'a LightConfig,
    pub rent: &'a Rent,
    pub current_slot: u64,
    /// Internal vec - dispatch functions push results here
    pub compressed_account_infos: Vec<CompressedAccountInfo>,
}

// ============================================================================
// Processor Function
// ============================================================================

/// Remaining accounts layout:
/// [0]: fee_payer (Signer, mut)
/// [1]: config (LightConfig PDA)
/// [2]: rent_sponsor (mut)
/// [system_accounts_offset..]: Light system accounts for CPI
/// [remaining_accounts.len() - num_pda_accounts..]: PDA accounts to decompress

/// Runtime processor - handles all the plumbing, dispatches via DecompressVariant trait.
///
/// **Takes raw instruction data** and deserializes internally - minimizes macro code.
/// **Uses only remaining_accounts** - no Context struct needed.
/// **Generic over V** - the program's `PackedProgramAccountVariant` enum.
pub fn process_decompress_pda_accounts_idempotent<'info, V>(
    remaining_accounts: &[AccountInfo<'info>],
    instruction_data: &[u8],
    cpi_signer: CpiSigner,
    program_id: &Pubkey,
) -> std::result::Result<(), ProgramError>
where
    V: DecompressVariant<'info>,
{
    // Deserialize params internally
    let params = DecompressIdempotentParams::<V>::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Extract and validate accounts using AccountIterator
    let mut account_iter = AccountIterator::new(remaining_accounts);
    let fee_payer = account_iter
        .next_signer_mut("fee_payer")
        .map_err(ProgramError::from)?;
    let config = account_iter
        .next_non_mut("config")
        .map_err(ProgramError::from)?;
    let rent_sponsor = account_iter
        .next_mut("rent_sponsor")
        .map_err(ProgramError::from)?;

    // Load and validate config
    let light_config = LightConfig::load_checked(config, program_id)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let rent = Rent::get()?;
    let current_slot = Clock::get()?.slot;

    let system_accounts_offset_usize = params.system_accounts_offset as usize;
    if system_accounts_offset_usize > remaining_accounts.len() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let cpi_accounts = CpiAccounts::new(
        fee_payer,
        &remaining_accounts[system_accounts_offset_usize..],
        cpi_signer,
    );

    // Build context struct with all needed data (includes internal vec)
    let mut decompress_ctx = DecompressCtx {
        program_id,
        cpi_accounts: &cpi_accounts,
        remaining_accounts,
        rent_sponsor,
        light_config: &light_config,
        rent: &rent,
        current_slot,
        compressed_account_infos: Vec::new(),
    };

    // PDA accounts at end of remaining_accounts
    let pda_accounts_start = remaining_accounts
        .len()
        .checked_sub(params.accounts.len())
        .ok_or(ProgramError::InvalidInstructionData)?;
    let pda_accounts = &remaining_accounts[pda_accounts_start..];

    // Process each account using trait dispatch
    for (i, packed_variant) in params.accounts.iter().enumerate() {
        let pda_account = &pda_accounts[i];

        // Dispatch via trait - implementation is in program's PackedProgramAccountVariant
        packed_variant.decompress(pda_account, &mut decompress_ctx)?;
    }

    // CPI to Light System Program with proof
    if !decompress_ctx.compressed_account_infos.is_empty() {
        LightSystemProgramCpi::new_cpi(cpi_signer, params.proof.into())
            .with_account_infos(&decompress_ctx.compressed_account_infos)
            .invoke(cpi_accounts.clone())
            .map_err(|_| ProgramError::Custom(200))?;
    }

    Ok(())
}

// ============================================================================
// Helper Function for Trait Implementations
// ============================================================================

/// Generic prepare_account_for_decompression.
///
/// Takes a packed variant and metadata, handles:
/// 1. Getting seeds from packed variant
/// 2. Unpacking data
/// 3. Creating PDA and writing data
/// 4. Deriving compressed address from PDA key
/// 5. Building CompressedAccountInfo for CPI
///
/// # Type Parameters
/// * `SEED_COUNT` - Number of seeds including bump
/// * `P` - Packed variant type implementing PackedLightAccountVariant
pub fn prepare_account_for_decompression<'info, const SEED_COUNT: usize, P>(
    packed: &P,
    meta: &CompressedAccountMetaNoLamportsNoAddress,
    pda_account: &AccountInfo<'info>,
    ctx: &mut DecompressCtx<'_, 'info>,
) -> std::result::Result<(), ProgramError>
where
    P: PackedLightAccountVariant<SEED_COUNT>,
    <P::Unpacked as LightAccountVariant<SEED_COUNT>>::Data:
        LightAccount + LightDiscriminator + Clone + AnchorSerialize + AnchorDeserialize,
{
    // 1. Idempotency check - if PDA already has data (non-zero discriminator), skip
    if !pda_account.data_is_empty() {
        let data = pda_account.try_borrow_data()?;
        if check_data_is_zeroed::<8>(&data).is_err() {
            // Already initialized - skip
            return Ok(());
        }
    }

    // 2. Get bump and seeds from packed variant
    // Packed indices are relative to packed_accounts (after system accounts offset)
    let packed_accounts = ctx.cpi_accounts.packed_accounts();

    let bump = packed.bump();
    let bump_storage = [bump];
    let seeds = packed.seed_refs_with_bump(packed_accounts, &bump_storage)?;

    // 3. Unpack to get the data
    let unpacked = packed
        .unpack(packed_accounts)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let account_data = unpacked.data().clone();

    // 4. Hash with canonical CompressionInfo::compressed() for input verification
    let data_bytes = account_data
        .try_to_vec()
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let data_len = data_bytes.len();
    let mut input_data_hash = Sha256::hash(&data_bytes).map_err(|_| ProgramError::Custom(100))?;
    input_data_hash[0] = 0; // Zero first byte per protocol convention

    // 5. Calculate space and create PDA
    type Data<const N: usize, P> =
        <<P as PackedLightAccountVariant<N>>::Unpacked as LightAccountVariant<N>>::Data;
    let discriminator_len = 8;
    let space = discriminator_len + data_len.max(<Data<SEED_COUNT, P> as LightAccount>::INIT_SPACE);
    let rent_minimum = ctx.rent.minimum_balance(space);

    let system_program = ctx
        .cpi_accounts
        .system_program()
        .map_err(|_| ProgramError::InvalidAccountData)?;

    create_pda_account(
        ctx.rent_sponsor,
        pda_account,
        rent_minimum,
        space as u64,
        ctx.program_id,
        &seeds,
        system_program,
    )?;

    // 6. Write discriminator + data to PDA
    let mut pda_data = pda_account.try_borrow_mut_data()?;
    pda_data[..8]
        .copy_from_slice(&<Data<SEED_COUNT, P> as LightDiscriminator>::LIGHT_DISCRIMINATOR);

    // 7. Set decompressed state and serialize
    let mut decompressed = account_data;
    decompressed.set_decompressed(ctx.light_config, ctx.current_slot);
    let writer = &mut &mut pda_data[8..];
    decompressed
        .serialize(writer)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // 8. Derive compressed address from PDA key (saves instruction data size)
    let address = derive_address(
        &pda_account.key.to_bytes(),
        &ctx.light_config.address_space[0].to_bytes(),
        &ctx.program_id.to_bytes(),
    );

    // 9. Build CompressedAccountInfo for CPI
    let tree_info = meta.tree_info;
    let input = InAccountInfo {
        data_hash: input_data_hash,
        lamports: 0,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            prove_by_index: tree_info.prove_by_index,
        },
        root_index: tree_info.root_index,
        discriminator: <Data<SEED_COUNT, P> as LightDiscriminator>::LIGHT_DISCRIMINATOR,
    };

    // Output is empty (nullifying the compressed account)
    let output = OutAccountInfo {
        lamports: 0,
        output_merkle_tree_index: meta.output_state_tree_index,
        discriminator: [0u8; 8],
        data: Vec::new(),
        data_hash: [0u8; 32],
    };

    // 10. Push to ctx's internal vec
    ctx.compressed_account_infos.push(CompressedAccountInfo {
        address: Some(address),
        input: Some(input),
        output: Some(output),
    });

    Ok(())
}
