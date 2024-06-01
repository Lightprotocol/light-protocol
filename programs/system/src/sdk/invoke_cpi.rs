use account_compression::errors::AccountCompressionErrorCode;
// #![cfg(target_os = "solana")]
use anchor_lang::{prelude::*, Bumps};

use super::{accounts::{InvokeAccounts, InvokeCpiAccounts, InvokeCpiContextAccount, LightSystemAccount, SignerAccounts}, CompressedCpiContext};

//TODO: add test and update function name
// Invokes the light system program for state transitions on compressed
// accounts.
//
// This function facilitates caller programs to interact with the light system
// program, ensuring state transitions are verified and applied correctly.
//
// # Parameters
// * `remaining_accounts`             A vector of `AccountInfo`.
// * `light_system_program`           The `AccountInfo` for the light system program.
// * `inputs`                         Serialized input data for the CPI call.
// * `cpi_accounts`                   Accounts required for the CPI, structured for the light system program.
// * `seeds`                          Array of seeds used for deriving the signing PDA.
//
// # Returns
// Result indicating the success or failure of the operation.
// pub fn invoke_system_cpi<'info>(
//     remaining_accounts: Vec<AccountInfo<'info>>,
//     light_system_program: AccountInfo<'info>,
//     inputs: Vec<u8>,
//     cpi_accounts: InvokeCpiInstruction,
//     seeds: [&[&[u8]]; 1],
// ) -> Result<()> {
//     invoke_cpi(
//         CpiContext::new_with_signer(light_system_program, cpi_accounts, &seeds)
//             .with_remaining_accounts(remaining_accounts.to_vec()),
//         inputs,
//     )
// }
// TODO: properly document compressed-cpi-context
// TODO: turn into a simple check!
// TOOD: CHECK needed bc can be different from own, if called from another program.
pub fn get_compressed_cpi_context_account<'info>(
    ctx: &Context<'_, '_, '_, 'info, impl InvokeAccounts<'info> + LightSystemAccount<'info> + InvokeCpiAccounts<'info> + SignerAccounts<'info> + Bumps>, 
    compressed_cpi_context: &CompressedCpiContext,
) -> Result<AccountInfo<'info>> {
    let cpi_context_account = ctx.remaining_accounts
        .get(compressed_cpi_context.cpi_context_account_index as usize)
        .map(|account| account.to_account_info())
        .ok_or_else(|| anchor_lang::error::Error::from(crate::errors::CompressedPdaError::CpiContextAccountUndefined))?;
    Ok(cpi_context_account)
}


// pub fn verify<'info>(
//     ctx: Context<'_, '_, '_, 'info, impl InvokeAccounts<'info> + LightSystemAccount<'info> + InvokeCpiAccounts<'info> + SignerAccounts<'info> + InvokeCpiContextAccount<'info> + Bumps>, 
//     inputs: Vec<u8>, 
//     seeds: [&[&[u8]]; 1]
// ) -> Result<()> {

//     let cpi_accounts = crate::cpi::accounts::InvokeCpiInstruction {
//         fee_payer: ctx.accounts.get_fee_payer().to_account_info(),
//         authority: ctx.accounts.get_authority().to_account_info(),
//         registered_program_pda: ctx.accounts.get_registered_program_pda().to_account_info(),
//         noop_program: ctx.accounts.get_noop_program().to_account_info(),
//         account_compression_authority: ctx.accounts.get_account_compression_authority().to_account_info(),
//         account_compression_program: ctx.accounts.get_account_compression_program().to_account_info(),
//         invoking_program: ctx.accounts.get_invoking_program().to_account_info(),
//         compressed_sol_pda: None,
//         compression_recipient: None,
//         system_program: ctx.accounts.get_system_program().to_account_info(),
//         // cpi_context_account: ctx.accounts.get_cpi_context_account().map(|acc| acc.to_account_info()), // Assuming there's a method to get this optionally
//         cpi_context_account: ctx.accounts.get_cpi_context_account().map(|acc| acc.to_account_info()),
//     };

//     crate::cpi::invoke_cpi(
//         CpiContext::new_with_signer(
//             ctx.accounts.get_light_system_program().to_account_info(),
//             cpi_accounts,
//             &seeds,
//         ).with_remaining_accounts(ctx.remaining_accounts.to_vec()),
//         inputs
//     )
// }

// #[cfg(test)]
// mod test {
//     use anchor_lang::AnchorDeserialize;

//     use super::*;

//     #[test]
//     fn test_create_execute_compressed_transaction() {
//         let payer = Pubkey::new_unique();
//         let recipient = Pubkey::new_unique();
//         let input_compressed_accounts = vec![
//             CompressedAccount {
//                 lamports: 100,
//                 owner: payer,
//                 address: None,
//                 data: None,
//             },
//             CompressedAccount {
//                 lamports: 100,
//                 owner: payer,
//                 address: None,
//                 data: None,
//             },
//         ];
//         let output_compressed_accounts = vec![
//             CompressedAccount {
//                 lamports: 50,
//                 owner: payer,
//                 address: None,
//                 data: None,
//             },
//             CompressedAccount {
//                 lamports: 150,
//                 owner: recipient,
//                 address: None,
//                 data: None,
//             },
//         ];
//         let merkle_tree_indices = vec![0, 2];
//         let merkle_tree_pubkey = Pubkey::new_unique();
//         let merkle_tree_pubkey_1 = Pubkey::new_unique();

//         let nullifier_array_pubkey = Pubkey::new_unique();
//         let input_merkle_context = vec![
//             MerkleContext {
//                 merkle_tree_pubkey,
//                 nullifier_queue_pubkey: nullifier_array_pubkey,
//                 leaf_index: 0,
//             },
//             MerkleContext {
//                 merkle_tree_pubkey,
//                 nullifier_queue_pubkey: nullifier_array_pubkey,
//                 leaf_index: 1,
//             },
//         ];

//         let output_compressed_account_merkle_tree_pubkeys =
//             vec![merkle_tree_pubkey, merkle_tree_pubkey_1];
//         let input_root_indices = vec![0, 1];
//         let proof = CompressedProof {
//             a: [0u8; 32],
//             b: [1u8; 64],
//             c: [0u8; 32],
//         };
//         let instruction = create_invoke_instruction(
//             &payer,
//             &payer,
//             &input_compressed_accounts.clone(),
//             &output_compressed_accounts.clone(),
//             &input_merkle_context,
//             &output_compressed_account_merkle_tree_pubkeys,
//             &input_root_indices.clone(),
//             Vec::<NewAddressParams>::new().as_slice(),
//             Some(proof.clone()),
//             Some(100),
//             true,
//             None,
//         );
//         assert_eq!(instruction.program_id, crate::ID);

//         let deserialized_instruction_data: InstructionDataInvoke =
//             InstructionDataInvoke::deserialize(&mut instruction.data[12..].as_ref()).unwrap();
//         deserialized_instruction_data
//             .input_compressed_accounts_with_merkle_context
//             .iter()
//             .enumerate()
//             .for_each(|(i, compressed_account_with_context)| {
//                 assert_eq!(
//                     input_compressed_accounts[i],
//                     compressed_account_with_context.compressed_account
//                 );
//             });
//         deserialized_instruction_data
//             .output_compressed_accounts
//             .iter()
//             .enumerate()
//             .for_each(|(i, compressed_account)| {
//                 assert_eq!(
//                     OutputCompressedAccountWithPackedContext {
//                         compressed_account: output_compressed_accounts[i].clone(),
//                         merkle_tree_index: merkle_tree_indices[i] as u8
//                     },
//                     *compressed_account
//                 );
//             });
//         assert_eq!(
//             deserialized_instruction_data
//                 .input_compressed_accounts_with_merkle_context
//                 .len(),
//             2
//         );
//         assert_eq!(
//             deserialized_instruction_data
//                 .output_compressed_accounts
//                 .len(),
//             2
//         );
//         assert_eq!(
//             deserialized_instruction_data.proof.clone().unwrap().a,
//             proof.a
//         );
//         assert_eq!(
//             deserialized_instruction_data.proof.clone().unwrap().b,
//             proof.b
//         );
//         assert_eq!(
//             deserialized_instruction_data.proof.clone().unwrap().c,
//             proof.c
//         );
//         assert_eq!(
//             deserialized_instruction_data.compression_lamports.unwrap(),
//             100
//         );
//         assert_eq!(deserialized_instruction_data.is_compress, true);
//         let ref_account_meta = AccountMeta::new(payer, true);
//         assert_eq!(instruction.accounts[0], ref_account_meta);
//         assert_eq!(
//             deserialized_instruction_data.input_compressed_accounts_with_merkle_context[0]
//                 .merkle_context
//                 .nullifier_queue_pubkey_index,
//             1
//         );
//         assert_eq!(
//             deserialized_instruction_data.input_compressed_accounts_with_merkle_context[1]
//                 .merkle_context
//                 .nullifier_queue_pubkey_index,
//             1
//         );
//         assert_eq!(
//             instruction.accounts[9 + deserialized_instruction_data
//                 .input_compressed_accounts_with_merkle_context[0]
//                 .merkle_context
//                 .merkle_tree_pubkey_index as usize],
//             AccountMeta::new(merkle_tree_pubkey, false)
//         );
//         assert_eq!(
//             instruction.accounts[9 + deserialized_instruction_data
//                 .input_compressed_accounts_with_merkle_context[1]
//                 .merkle_context
//                 .merkle_tree_pubkey_index as usize],
//             AccountMeta::new(merkle_tree_pubkey, false)
//         );
//         assert_eq!(
//             instruction.accounts[9 + deserialized_instruction_data
//                 .input_compressed_accounts_with_merkle_context[0]
//                 .merkle_context
//                 .nullifier_queue_pubkey_index as usize],
//             AccountMeta::new(nullifier_array_pubkey, false)
//         );
//         assert_eq!(
//             instruction.accounts[9 + deserialized_instruction_data
//                 .input_compressed_accounts_with_merkle_context[1]
//                 .merkle_context
//                 .nullifier_queue_pubkey_index as usize],
//             AccountMeta::new(nullifier_array_pubkey, false)
//         );
//         assert_eq!(
//             instruction.accounts[9 + deserialized_instruction_data.output_compressed_accounts[0]
//                 .merkle_tree_index as usize],
//             AccountMeta::new(merkle_tree_pubkey, false)
//         );
//         assert_eq!(
//             instruction.accounts[9 + deserialized_instruction_data.output_compressed_accounts[1]
//                 .merkle_tree_index as usize],
//             AccountMeta::new(merkle_tree_pubkey_1, false)
//         );
//     }
// }
