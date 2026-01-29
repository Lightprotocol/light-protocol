use anchor_lang::prelude::*;
use light_compressed_account::{
    address::derive_address,
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_hasher::{Hasher, Sha256};
use light_sdk_types::{constants::RENT_SPONSOR_SEED, instruction::PackedStateTreeInfo};
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;

use super::traits::{LightAccount, LightAccountVariantTrait, PackedLightAccountVariantTrait};
use crate::{
    interface::{create_pda_account, DecompressCtx},
    light_account_checks::checks::check_data_is_zeroed,
    LightDiscriminator,
};

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
/// * `P` - Packed variant type implementing PackedLightAccountVariantTrait
pub fn prepare_account_for_decompression<'info, const SEED_COUNT: usize, P>(
    packed: &P,
    tree_info: &PackedStateTreeInfo,
    output_queue_index: u8,
    pda_account: &AccountInfo<'info>,
    ctx: &mut DecompressCtx<'_, 'info>,
) -> std::result::Result<(), ProgramError>
where
    P: PackedLightAccountVariantTrait<SEED_COUNT>,
    <P::Unpacked as LightAccountVariantTrait<SEED_COUNT>>::Data:
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

    // 2. Unpack to get the data (must happen before seed derivation so seed_vec() works
    //    with function-call seeds that produce temporaries)
    let packed_accounts = ctx.cpi_accounts.packed_accounts();

    let unpacked = packed
        .unpack(packed_accounts)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let account_data = unpacked.data().clone();

    // 3. Get seeds from unpacked variant using seed_vec() (owned data, no lifetime issues)
    let bump = packed.bump();
    let bump_bytes = [bump];
    let mut seed_vecs = unpacked.seed_vec();
    seed_vecs.push(bump_bytes.to_vec());
    let seed_slices: Vec<&[u8]> = seed_vecs.iter().map(|v| v.as_slice()).collect();

    // 4. Hash with canonical CompressionInfo::compressed() for input verification
    let data_bytes = account_data
        .try_to_vec()
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let data_len = data_bytes.len();
    let mut input_data_hash = Sha256::hash(&data_bytes).map_err(|_| ProgramError::Custom(100))?;
    input_data_hash[0] = 0; // Zero first byte per protocol convention

    // 5. Calculate space and create PDA
    type Data<const N: usize, P> =
        <<P as PackedLightAccountVariantTrait<N>>::Unpacked as LightAccountVariantTrait<N>>::Data;
    let discriminator_len = 8;
    let space = discriminator_len + data_len.max(<Data<SEED_COUNT, P> as LightAccount>::INIT_SPACE);
    let rent_minimum = ctx.rent.minimum_balance(space);

    let system_program = ctx
        .cpi_accounts
        .system_program()
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Construct rent sponsor seeds for PDA signing
    let rent_sponsor_bump_bytes = [ctx.rent_sponsor_bump];
    let rent_sponsor_seeds: &[&[u8]] = &[RENT_SPONSOR_SEED, &rent_sponsor_bump_bytes];

    create_pda_account(
        ctx.rent_sponsor,
        rent_sponsor_seeds,
        pda_account,
        rent_minimum,
        space as u64,
        ctx.program_id,
        &seed_slices,
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
        output_merkle_tree_index: output_queue_index,
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
