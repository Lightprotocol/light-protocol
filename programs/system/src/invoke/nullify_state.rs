// use account_compression::append_nullify_create_address::AppendNullifyCreateAddressInputs;
// use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, Bumps};
// use light_macros::heap_neutral;

// use crate::{
//     constants::CPI_AUTHORITY_PDA_BUMP,
//     invoke::cpi_acp::{create_cpi_data, get_index_or_insert, CpiData},
//     invoke_cpi::verify_signer::check_program_owner_state_merkle_tree,
//     sdk::{
//         accounts::{InvokeAccounts, SignerAccounts},
//         compressed_account::PackedCompressedAccountWithMerkleContext,
//     },
// };

// /// 1. Checks that if nullifier queue has program_owner it invoking_program is
// ///    program_owner.
// /// 2. Inserts nullifiers into the queue.
// #[heap_neutral]
// pub fn insert_nullifiers<
//     'a,
//     'b,
//     'c: 'info,
//     'info,
//     A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
// >(
//     input_compressed_accounts_with_merkle_context: &'a [PackedCompressedAccountWithMerkleContext],
//     ctx: &'a Context<'a, 'b, 'c, 'info, A>,
//     nullifiers: &'a [[u8; 32]],
//     invoking_program: &Option<Pubkey>,
//     // tx_hash: [u8; 32],
// ) -> Result<Option<(u8, u64)>> {
//     light_heap::bench_sbf_start!("cpda_insert_nullifiers_prep_accs");
//     msg!(
//         "get_account_compression_authority {:?}",
//         ctx.accounts.get_account_compression_authority().key()
//     );
//     let num_leaves = 0;
//     let num_nullifiers = nullifiers.len() as u8;
//     let num_new_addresses = 0;
//     let CpiData {
//         mut bytes,
//         mut account_indices,
//         mut accounts,
//         mut account_infos,
//         ..
//     } = create_cpi_data(ctx, num_nullifiers, num_nullifiers, num_new_addresses)?;

//     let mut append_nullify_create_address_inputs = AppendNullifyCreateAddressInputs::new(
//         &mut bytes,
//         num_leaves,
//         num_nullifiers,
//         num_new_addresses,
//     )
//     .map_err(ProgramError::from)?;
//     append_nullify_create_address_inputs.set_invoked_by_program(true);
//     // append_nullify_create_address_inputs.tx_hash = tx_hash;
//     append_nullify_create_address_inputs.bump = CPI_AUTHORITY_PDA_BUMP;
//     // let mut leaf_indices = Vec::with_capacity(input_compressed_accounts_with_merkle_context.len());
//     // let mut prove_by_index =
//     //     Vec::with_capacity(input_compressed_accounts_with_merkle_context.len());
//     // If the transaction contains at least one input compressed account a
//     // network fee is paid. This network fee is paid in addition to the address
//     // network fee. The network fee is paid once per transaction, defined in the
//     // state Merkle tree and transferred to the nullifier queue because the
//     // nullifier queue is mutable. The network fee field in the queue is not
//     // used.
//     let mut network_fee_bundle = None;
//     for (i, account) in input_compressed_accounts_with_merkle_context
//         .iter()
//         .enumerate()
//     {
//         append_nullify_create_address_inputs.nullifiers[i].account_hash = nullifiers[i];
//         append_nullify_create_address_inputs.nullifiers[i].leaf_index =
//             account.merkle_context.leaf_index.into();
//         append_nullify_create_address_inputs.nullifiers[i].prove_by_index =
//             account.merkle_context.prove_by_index as u8;
//         // let queue_index = get_index_or_insert(
//         //     account.merkle_context.nullifier_queue_pubkey_index,
//         //     &mut account_indices,
//         //     &mut account_infos,
//         //     &mut accounts,
//         //     ctx.remaining_accounts,
//         // );
//         // append_nullify_create_address_inputs.nullifiers[i].queue_index = queue_index;
//         // let tree_index = get_index_or_insert(
//         //     account.merkle_context.merkle_tree_pubkey_index,
//         //     &mut account_indices,
//         //     &mut account_infos,
//         //     &mut accounts,
//         //     ctx.remaining_accounts,
//         // );
//         // append_nullify_create_address_inputs.nullifiers[i].tree_index = tree_index;

//         // 1. Check invoking signer is eligible to write to the nullifier queue.
//         let (_, network_fee, _, _) = check_program_owner_state_merkle_tree::<true>(
//             &ctx.remaining_accounts[account.merkle_context.merkle_tree_pubkey_index as usize],
//             invoking_program,
//         )?;
//         if network_fee_bundle.is_none() && network_fee.is_some() {
//             network_fee_bundle = Some((
//                 account.merkle_context.nullifier_queue_pubkey_index,
//                 network_fee.unwrap(),
//             ));
//         }
//     }
//     append_nullify_create_address_inputs.num_queues = account_indices.len() as u8 / 2;
//     light_heap::bench_sbf_end!("cpda_insert_nullifiers_prep_accs");
//     light_heap::bench_sbf_start!("cpda_instruction_data");
//     Ok(network_fee_bundle)
// }
