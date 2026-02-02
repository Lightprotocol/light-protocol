//! Generic prepare_account_for_decompression.

use crate::{constants::RENT_SPONSOR_SEED, instruction::PackedStateTreeInfo};
use light_account_checks::AccountInfoTrait;
use light_compressed_account::{
    address::derive_address,
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_compressible::DECOMPRESSED_PDA_DISCRIMINATOR;
use light_hasher::{sha256::Sha256BE, Hasher};

use crate::interface::{
    account::light_account::LightAccount,
    program::{
        decompression::processor::DecompressCtx,
        variant::{LightAccountVariantTrait, PackedLightAccountVariantTrait},
    },
};
use crate::{
    error::LightSdkTypesError,
    light_account_checks::discriminator::Discriminator as LightDiscriminator, AnchorSerialize,
};

/// Generic prepare_account_for_decompression.
///
/// Takes a packed variant and metadata, handles:
/// 1. Validating PDA derivation (security check - MUST be first)
/// 2. Checking idempotency (skip if already initialized)
/// 3. Getting seeds from packed variant
/// 4. Unpacking data
/// 5. Creating PDA and writing data
/// 6. Deriving compressed address from PDA key
/// 7. Building CompressedAccountInfo for CPI
///
/// # Security
/// PDA validation MUST run before idempotency check to prevent accepting
/// wrong PDAs that happen to be already initialized.
///
/// # Type Parameters
/// * `SEED_COUNT` - Number of seeds including bump
/// * `P` - Packed variant type implementing PackedLightAccountVariantTrait
/// * `AI` - Account info type (solana or pinocchio)
#[inline(never)]
pub fn prepare_account_for_decompression<const SEED_COUNT: usize, P, AI>(
    packed: &P,
    tree_info: &PackedStateTreeInfo,
    output_queue_index: u8,
    pda_account: &AI,
    ctx: &mut DecompressCtx<'_, AI>,
) -> Result<(), LightSdkTypesError>
where
    AI: AccountInfoTrait + Clone,
    P: PackedLightAccountVariantTrait<SEED_COUNT>,
    <P::Unpacked as LightAccountVariantTrait<SEED_COUNT>>::Data: LightAccount,
{
    // Type alias for the account data type
    type Data<const N: usize, P> =
        <<P as PackedLightAccountVariantTrait<N>>::Unpacked as LightAccountVariantTrait<N>>::Data;

    // 1. Unpack to get seeds (must happen first for PDA validation)
    let packed_accounts = ctx
        .cpi_accounts
        .packed_accounts()
        .map_err(LightSdkTypesError::from)?;

    let unpacked = packed
        .unpack(packed_accounts)
        .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
    let account_data = unpacked.data().clone();

    // 2. Get seeds from unpacked variant using seed_vec() (owned data, no lifetime issues)
    let bump = packed.bump();
    let bump_bytes = [bump];
    let mut seed_vecs = unpacked.seed_vec();
    seed_vecs.push(bump_bytes.to_vec());
    let seed_slices: Vec<&[u8]> = seed_vecs.iter().map(|v| v.as_slice()).collect();

    // 3. SECURITY: Validate PDA derivation FIRST (defense-in-depth)
    // This MUST run before idempotency check to prevent accepting wrong PDAs
    let expected_pda = AI::create_program_address(&seed_slices, ctx.program_id)
        .map_err(|_| LightSdkTypesError::InvalidSeeds)?;

    if pda_account.key() != expected_pda {
        return Err(LightSdkTypesError::InvalidSeeds);
    }

    // 4. Idempotency check - if PDA already has data (non-zero discriminator), skip
    // IMPORTANT: This runs AFTER PDA validation so wrong PDAs cannot bypass validation
    if crate::interface::program::validation::is_pda_initialized(pda_account)? {
        return Ok(());
    }

    // 5. Hash with canonical CompressionInfo::compressed() for input verification
    let data_bytes = account_data
        .try_to_vec()
        .map_err(|_| LightSdkTypesError::Borsh)?;
    let data_len = data_bytes.len();
    let mut input_data_hash = Sha256BE::hash(&data_bytes)?;
    input_data_hash[0] = 0; // Zero first byte per protocol convention

    // 6. Calculate space and create PDA
    let discriminator_len = 8;
    let space = discriminator_len + data_len.max(<Data<SEED_COUNT, P> as LightAccount>::INIT_SPACE);
    let rent_minimum = AI::get_min_rent_balance(space)?;

    let system_program = ctx
        .cpi_accounts
        .system_program()
        .map_err(LightSdkTypesError::from)?;

    // Construct rent sponsor seeds for PDA signing
    let rent_sponsor_bump_bytes = [ctx.rent_sponsor_bump];
    let rent_sponsor_seeds: &[&[u8]] = &[RENT_SPONSOR_SEED, &rent_sponsor_bump_bytes];

    pda_account
        .create_pda_account(
            rent_minimum,
            space as u64,
            ctx.program_id,
            &seed_slices,
            ctx.rent_sponsor,
            rent_sponsor_seeds,
            system_program,
        )
        .map_err(|_| LightSdkTypesError::CpiFailed)?;

    // 7. Write discriminator + data to PDA
    let mut pda_data = pda_account
        .try_borrow_mut_data()
        .map_err(|_| LightSdkTypesError::ConstraintViolation)?;
    pda_data[..8]
        .copy_from_slice(&<Data<SEED_COUNT, P> as LightDiscriminator>::LIGHT_DISCRIMINATOR);

    // 8. Set decompressed state and serialize
    let mut decompressed = account_data;
    decompressed.set_decompressed(ctx.light_config, ctx.current_slot);
    let writer = &mut &mut pda_data[8..];
    decompressed
        .serialize(writer)
        .map_err(|_| LightSdkTypesError::Borsh)?;

    // 9. Derive compressed address from PDA key
    let pda_key = pda_account.key();
    let address = derive_address(&pda_key, &ctx.light_config.address_space[0], ctx.program_id);

    // 10. Build CompressedAccountInfo for CPI
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

    // Output is a DECOMPRESSED_PDA placeholder (same as init creates).
    // This allows CompressAccountsIdempotent to re-compress the account
    // in a future cycle by finding and nullifying this placeholder.
    let pda_pubkey_bytes = pda_account.key();
    let output_data_hash = Sha256BE::hash(&pda_pubkey_bytes)?;
    let output = OutAccountInfo {
        lamports: 0,
        output_merkle_tree_index: output_queue_index,
        discriminator: DECOMPRESSED_PDA_DISCRIMINATOR,
        data: pda_pubkey_bytes.to_vec(),
        data_hash: output_data_hash,
    };

    // 11. Push to ctx's internal vec
    ctx.compressed_account_infos.push(CompressedAccountInfo {
        address: Some(address),
        input: Some(input),
        output: Some(output),
    });

    Ok(())
}
