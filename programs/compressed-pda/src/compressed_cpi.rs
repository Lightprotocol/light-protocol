use account_compression::StateMerkleTreeAccount;
use anchor_lang::prelude::*;

use crate::{event::PublicTransactionEvent, InstructionDataTransfer, TransferInstruction};
use aligned_sized::aligned_sized;

#[derive(Accounts)]
pub struct InitializeCpiSignatureAccount<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(zero)]
    pub cpi_signature_account: Account<'info, CpiSignatureAccount>,
    pub system_program: Program<'info, System>,
    pub associated_merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
}
// Security:
// - checking the slot is not enough there can be multiple transactions in the same slot
// - the CpiSignatureAccount must be derived from the first Merkle tree account as the current transaction
// - to check that all data in the CpiSignature account is from the same transaction we compare the proof bytes
// - I need to guaratee that all the data in the cpi signature account is from the same transaction
//   - if we just overwrite the data in the account if the proof is different we cannot be sure because the program could be malicious
//   - wouldn't the same proofs be enough, if you overwrite something then I discard everything that is in the account -> these utxos will not be spent
//   - do I need to check ownership before or after? before we need to check who invoked the program
//   - we need a transaction hash that hashes the complete instruction data, this will be a pain to produce offchain Sha256(proof, input_account_hashes, output_account_hashes, relay_fee, compression_lamports)
//   - the last tx passes the hash and tries to recalculate the hash
/// collects invocations without proofs
/// invocations are collected and processed when an invocation with a proof is received
#[aligned_sized(anchor)]
// #[account]
#[derive(Debug, PartialEq, Default)]
#[account]
pub struct CpiSignatureAccount {
    pub associated_merkle_tree: Pubkey,
    pub execute: bool,
    pub signatures: Vec<InstructionDataTransfer>,
}

impl CpiSignatureAccount {
    pub fn init(&mut self, associated_merkle_tree: Pubkey) {
        self.associated_merkle_tree = associated_merkle_tree;
        self.execute = false;
        self.signatures = Vec::new();
    }
}

pub const CPI_SEED: &[u8] = b"cpi_signature_pda";

/// To spend multiple compressed
#[derive(Debug, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedCpiContext {
    /// index of the output state Merkle tree that will be used to store cpi signatures
    /// The transaction will fail if this index is not consistent in your transaction.
    pub cpi_signature_account_index: u8,
    /// The final cpi of your program needs to set execute to true.
    /// Execute compressed transaction will verify the proof and execute the transaction if this is true.
    /// If this is false the transaction will be stored in the cpi signature account.
    pub execute: bool,
}

// TODO: validate security of this approach
pub fn process_cpi_context<'a, 'b, 'c: 'info, 'info>(
    cpi_context: CompressedCpiContext,
    ctx: &mut Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: &mut InstructionDataTransfer,
) -> Result<Option<std::prelude::v1::Result<PublicTransactionEvent, Error>>> {
    // // cpi_signature_account_index needs to be one of the output state Merkle tree account indices
    // if inputs
    //     .output_state_merkle_tree_account_indices
    //     .iter()
    //     .any(|&x| x != cpi_context.cpi_signature_account_index)
    // {
    //     return Ok(Some(Err(
    //         crate::ErrorCode::CpiSignatureAccountIndexInOutputStateMerkleTreeAccountIndices.into(),
    //     )));
    // }

    // let cpi_signature_account = &mut cpi_signature_account_from_merkle_tree_account_mut(
    //     &mut ctx.remaining_accounts[cpi_context.cpi_signature_account_index as usize],
    // )?;
    // let account: &'info AccountInfo =
    //     &ctx.remaining_accounts[cpi_context.cpi_signature_account_index as usize];
    // let merkle_tree_account = AccountLoader::<StateMerkleTreeAccount>::try_from(&account).unwrap();
    // let merkle_tree = &mut merkle_tree_account.load_mut()?;
    // msg!("cpi signature len {}", CpiSignatureAccount::LEN);
    // let mut cpi_signature_account =
    //     CpiSignatureAccount::deserialize_reader(&mut &merkle_tree.cpi_signatures[..]).unwrap();
    // .as_mut()
    // .unwrap()
    // .load_mut()?;
    // let merkle_tree_pubkey = merkle_tree_account.key();
    // let mut merkle_tree = merkle_tree_account.load_mut()?;
    // let deserialized_account = StateMerkleTreeAccount::load_merkle_tree_mut(merkle_tree_account);
    // ConcurrentMerkleTree::struct_from_bytes_mut(account.data.as_mut_slice())
    //     .expect("failed to deserialize merkle tree account");
    // let mut cpi_signature_account =
    //     CpiSignatureAccount::try_from_slice(&cpi_signature_account.cpi_signatures).unwrap();
    if !cpi_context.execute {
        // TODO: enable for more than invocations by adding an execute tx input, we should have a macro that adds it automatically to a program that wants to activate cpi
        // TODO: remove cpi_signature_account and make the Merkle tree accounts bigger
        // we should use one of the output Merkle tree accounts as the cpi_signature_account
        match ctx.accounts.cpi_signature_account.is_some() {
            true => {
                if let Some(cpi_signature_account) = &mut ctx.accounts.cpi_signature_account {
                    msg!("cpi_signature_account detected");
                    // Check conditions and modify the signatures
                    if cpi_signature_account.signatures.is_empty() {
                        msg!("cpi signatures are empty");
                        // cpi signature account should only be used with mutiple compressed accounts owned by different programs
                        // thus the first invocation execute is assumed to be false
                        cpi_signature_account.signatures.push(inputs.clone());
                    } else if cpi_signature_account.signatures[0].proof.as_ref().unwrap()
                        == inputs.proof.as_ref().unwrap()
                    {
                        cpi_signature_account.signatures.push(inputs.clone());
                    } else {
                        cpi_signature_account.signatures = vec![inputs.clone()];
                    }
                    // serialize the cpi signature account
                    // let mut cpi_signature_account_data = vec![0u8; CpiSignatureAccount::LEN];
                    // cpi_signature_account
                    //     .serialize(&mut merkle_tree.cpi_signatures.as_mut())
                    //     .unwrap();
                };
            }
            false => {
                return Ok(Some(err!(crate::ErrorCode::CpiSignatureAccountUndefined)));
            }
        };
        return Ok(Some(Ok(PublicTransactionEvent::default())));
    } else {
        if let Some(cpi_signature_account) = &ctx.accounts.cpi_signature_account {
            inputs.combine(&cpi_signature_account.signatures);
        }
        // inputs.combine(&cpi_signature_account.signatures);
    }
    Ok(None)
}

// pub fn cpi_signature_account_from_merkle_tree_account_mut<'info>(
//     account: &mut AccountInfo<'info>,
// ) -> Result<CpiSignatureAccount> {
//     let merkle_tree_account = &mut AccountLoader::<StateMerkleTreeAccount>::try_from(account)
//         .unwrap()
//         .load_mut()?;
//     // let merkle_tree_pubkey = merkle_tree_account.key();
//     // let mut merkle_tree = merkle_tree_account.load_mut()?;
//     // let deserialized_account = StateMerkleTreeAccount::load_merkle_tree_mut(merkle_tree_account);
//     // ConcurrentMerkleTree::struct_from_bytes_mut(account.data.as_mut_slice())
//     //     .expect("failed to deserialize merkle tree account");
//     let mut cpi_signature_account =
//         CpiSignatureAccount::try_from_slice(&merkle_tree_account.cpi_signatures).unwrap();
//     Ok(cpi_signature_account)
// }

// pub unsafe fn struct_from_bytes_mut(bytes_struct: &[u8]) -> Result<&mut CpiSignatureAccount> {
//     let expected_bytes_struct_size = std::mem::size_of::<CpiSignatureAccount>();
//     if bytes_struct.len() != expected_bytes_struct_size {
//         return Err(crate::ErrorCode::StructBufferSize(
//             expected_bytes_struct_size,
//             bytes_struct.len(),
//         ));
//     }
//     let tree: *mut CpiSignatureAccount = bytes_struct.as_ptr() as _;

//     Ok(&mut *tree)
// }
