use account_compression::StateMerkleTreeAccount;
use anchor_lang::prelude::*;
use groth16_solana::{
    decompression::{decompress_g1, decompress_g2},
    groth16::{Groth16Verifier, Groth16Verifyingkey},
};

use crate::{
    compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
    instructions::{InstructionDataTransfer, TransferInstruction},
    utils::CompressedProof,
    ErrorCode,
};

#[inline(never)]
pub fn fetch_roots<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    roots: &'a mut [[u8; 32]],
) -> Result<()> {
    for (j, input_compressed_account_with_context) in inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .enumerate()
    {
        let merkle_tree = AccountLoader::<StateMerkleTreeAccount>::try_from(
            &ctx.remaining_accounts
                [input_compressed_account_with_context.index_mt_account as usize],
        )
        .unwrap();
        let merkle_tree = merkle_tree.load()?;
        let fetched_roots = merkle_tree.load_roots()?;

        roots[j] = fetched_roots[inputs.input_root_indices[j] as usize];
    }
    Ok(())
}

#[inline(never)]
pub fn hash_input_compressed_accounts<'a, 'b, 'c: 'info, 'info>(
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: &'a InstructionDataTransfer,
    leaves: &'a mut [[u8; 32]],
) -> anchor_lang::Result<()> {
    for (j, input_compressed_account_with_context) in inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .enumerate()
    {
        if input_compressed_account_with_context
            .compressed_account
            .address
            .is_some()
        {
            unimplemented!("Address is not supported yet")
        }
        leaves[j] = input_compressed_account_with_context
            .compressed_account
            .hash(
                &ctx.remaining_accounts
                    [input_compressed_account_with_context.index_mt_account as usize]
                    .key(),
                &input_compressed_account_with_context.leaf_index,
            )?;
    }
    Ok(())
}

#[inline(never)]
pub fn sum_check(
    input_compressed_accounts_with_merkle_context: &[CompressedAccountWithMerkleContext],
    output_compressed_account: &[CompressedAccount],
    relay_fee: &Option<u64>,
) -> anchor_lang::Result<()> {
    let mut sum: u64 = 0;
    for compressed_account_with_context in input_compressed_accounts_with_merkle_context.iter() {
        sum = sum
            .checked_add(compressed_account_with_context.compressed_account.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeInputSumFailed)?;
    }

    for compressed_account in output_compressed_account.iter() {
        sum = sum
            .checked_sub(compressed_account.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeOutputSumFailed)?;
    }

    if let Some(relay_fee) = relay_fee {
        sum = sum
            .checked_sub(*relay_fee)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeRpcSumFailed)?;
    }

    if sum == 0 {
        Ok(())
    } else {
        Err(ErrorCode::SumCheckFailed.into())
    }
}

#[inline(never)]
pub fn verify_merkle_proof_zkp(
    roots: &[[u8; 32]],
    leaves: &[[u8; 32]],
    compressed_proof: &CompressedProof,
) -> Result<()> {
    let public_inputs = [roots, leaves].concat();

    // The public inputs are expected to be a multiple of 2
    // 2 inputs means 1 inclusion proof (1 root and 1 leaf)
    // 4 inputs means 2 inclusion proofs (2 roots and 2 leaves)
    // 6 inputs means 3 inclusion proofs (3 roots and 3 leaves)
    // 8 inputs means 4 inclusion proofs (4 roots and 4 leaves)
    // 16 inputs means 8 inclusion proofs (8 roots and 8 leaves)
    match public_inputs.len() {
        2 => verify::<2>(
            &public_inputs
                .try_into()
                .map_err(|_| ErrorCode::PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::inclusion_26_1::VERIFYINGKEY,
        ),
        4 => verify::<4>(
            &public_inputs
                .try_into()
                .map_err(|_| ErrorCode::PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::inclusion_26_2::VERIFYINGKEY,
        ),
        6 => verify::<6>(
            &public_inputs
                .try_into()
                .map_err(|_| ErrorCode::PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::inclusion_26_3::VERIFYINGKEY,
        ),
        8 => verify::<8>(
            &public_inputs
                .try_into()
                .map_err(|_| ErrorCode::PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::inclusion_26_4::VERIFYINGKEY,
        ),
        16 => verify::<16>(
            &public_inputs
                .try_into()
                .map_err(|_| ErrorCode::PublicInputsTryIntoFailed)?,
            compressed_proof,
            &crate::verifying_keys::inclusion_26_8::VERIFYINGKEY,
        ),
        _ => Err(crate::ErrorCode::InvalidPublicInputsLength.into()),
    }
}

// TODO: remove const generics from groth16 solana
#[inline(never)]
fn verify<const N: usize>(
    public_inputs: &[[u8; 32]; N],
    proof: &CompressedProof,
    vk: &Groth16Verifyingkey,
) -> Result<()> {
    let proof_a = decompress_g1(&proof.a).map_err(|_| crate::ErrorCode::DecompressG1Failed)?;
    let proof_b = decompress_g2(&proof.b).map_err(|_| crate::ErrorCode::DecompressG2Failed)?;
    let proof_c = decompress_g1(&proof.c).map_err(|_| crate::ErrorCode::DecompressG1Failed)?;

    let mut verifier = Groth16Verifier::new(&proof_a, &proof_b, &proof_c, public_inputs, vk)
        .map_err(|_| crate::ErrorCode::CreateGroth16VerifierFailed)?;
    verifier.verify().map_err(|_| {
        #[cfg(target_os = "solana")]
        {
            msg!("Proof verification failed");
            msg!("Public inputs: {:?}", public_inputs);
            msg!("Proof A: {:?}", proof_a);
            msg!("Proof B: {:?}", proof_b);
            msg!("Proof C: {:?}", proof_c);
        }
        crate::ErrorCode::ProofVerificationFailed
    })?;
    Ok(())
}

#[cfg(test)]
mod test {
    use circuitlib_rs::{
        gnark::{
            constants::{INCLUSION_PATH, SERVER_ADDRESS},
            helpers::{health_check, kill_gnark_server, spawn_gnark_server},
            inclusion_json_formatter::inclusion_inputs_string,
            proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
        },
        helpers::init_logger,
    };
    use reqwest::Client;

    use super::*;
    use crate::compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext};

    #[tokio::test]
    async fn prove_inclusion() {
        init_logger();
        let mut gnark = spawn_gnark_server("../../circuit-lib/circuitlib-rs/scripts/prover.sh", 5);
        health_check().await;
        let client = Client::new();
        for number_of_compressed_accounts in &[1usize, 2, 3, 4, 8] {
            let (inputs, big_int_inputs) =
                inclusion_inputs_string(*number_of_compressed_accounts as usize);
            let response_result = client
                .post(&format!("{}{}", SERVER_ADDRESS, INCLUSION_PATH))
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(inputs)
                .send()
                .await
                .expect("Failed to execute request.");
            assert!(response_result.status().is_success());
            let body = response_result.text().await.unwrap();
            let proof_json = deserialize_gnark_proof_json(&body).unwrap();
            let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
            let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
            let mut roots = Vec::<[u8; 32]>::new();
            let mut leaves = Vec::<[u8; 32]>::new();

            for _ in 0..*number_of_compressed_accounts {
                roots.push(big_int_inputs.root.to_bytes_be().1.try_into().unwrap());
                leaves.push(big_int_inputs.leaf.to_bytes_be().1.try_into().unwrap());
            }

            verify_merkle_proof_zkp(
                &roots,
                &leaves,
                &CompressedProof {
                    a: proof_a,
                    b: proof_b,
                    c: proof_c,
                },
            )
            .unwrap();
        }
        kill_gnark_server(&mut gnark);
    }

    #[test]
    fn test_sum_check_passes() {
        let input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext> = vec![
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 100,
                    address: None,
                    data: None,
                },
                index_mt_account: 0,
                index_nullifier_array_account: 0,
                leaf_index: 0,
            },
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                index_mt_account: 0,
                index_nullifier_array_account: 0,
                leaf_index: 1,
            },
        ];

        let output_compressed_account: Vec<CompressedAccount> = vec![CompressedAccount {
            owner: Pubkey::new_unique(),
            lamports: 150,
            address: None,
            data: None,
        }];

        let relay_fee = None; // No RPC fee

        let result = sum_check(
            &input_compressed_accounts_with_merkle_context,
            &output_compressed_account,
            &relay_fee,
        );
        assert!(result.is_ok());
    }

    // TODO: add test for relay fee
    #[test]
    fn test_sum_check_fails() {
        let input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext> = vec![
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 100,
                    address: None,
                    data: None,
                },
                index_mt_account: 0,
                index_nullifier_array_account: 0,
                leaf_index: 0,
            },
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                index_mt_account: 0,
                index_nullifier_array_account: 0,
                leaf_index: 1,
            },
        ];

        let output_compressed_account: Vec<CompressedAccount> = vec![CompressedAccount {
            owner: Pubkey::new_unique(),
            lamports: 25,
            address: None,
            data: None,
        }];

        let relay_fee = Some(50); // Adding an RPC fee to ensure the sums don't match

        let result = sum_check(
            &input_compressed_accounts_with_merkle_context,
            &output_compressed_account,
            &relay_fee,
        );
        assert!(result.is_err());
    }
}
