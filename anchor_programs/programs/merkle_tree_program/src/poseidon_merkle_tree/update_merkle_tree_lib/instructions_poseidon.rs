use crate::poseidon_merkle_tree::poseidon_round_constants_split;
use anchor_lang::prelude::*;
use ark_ed_on_bn254::Fq;
use ark_ff::{
    bytes::{FromBytes, ToBytes},
    BigInteger,
};
use ark_std::Zero;
use arkworks_gadgets::poseidon::{sbox::PoseidonSbox, PoseidonParameters, Rounds};
use arkworks_gadgets::utils;

use crate::poseidon_merkle_tree::update_merkle_tree_lib::merkle_tree_update_state::MerkleTreeUpdateState;
use std::ops::{Add, AddAssign, Mul};

//configuration for the poseidon hash to be compatible with circom bn254 with 2 inputs
#[derive(Default, Clone)]
pub struct PoseidonCircomRounds3;

impl Rounds for PoseidonCircomRounds3 {
    const FULL_ROUNDS: usize = 8;
    const PARTIAL_ROUNDS: usize = 57;
    const SBOX: PoseidonSbox = PoseidonSbox::Exponentiation(5);
    const WIDTH: usize = 3;
}

pub fn poseidon_0(verifier_state_data: &mut MerkleTreeUpdateState) -> Result<()> {
    let mut current_round_index = 0;
    let mut current_round = 0;
    let mds = poseidon_round_constants_split::get_mds_poseidon_circom_bn254_x5_3();

    let rounds = poseidon_round_constants_split::get_rounds_poseidon_circom_bn254_x5_3_split(
        current_round_index,
    );
    let params = PoseidonParameters::<Fq>::new(rounds, mds.clone());
    let mut state_new1 = prepare_inputs(
        &params,
        &verifier_state_data.node_left,
        &verifier_state_data.node_right,
    )
    .unwrap();
    state_new1 = permute_custom_split(&params, state_new1, current_round, 4).unwrap();
    current_round += 4;
    current_round_index += 1;

    for _i in 0..3 {
        let rounds = poseidon_round_constants_split::get_rounds_poseidon_circom_bn254_x5_3_split(
            current_round_index,
        );
        let params = PoseidonParameters::<Fq>::new(rounds, mds.clone());
        state_new1 = permute_custom_split(&params, state_new1, current_round, 6).unwrap();
        current_round += 6;
        current_round_index += 1;
    }
    let mut state_final = vec![vec![0u8; 32]; 3];
    for (i, input_state) in state_final.iter_mut().enumerate() {
        <Fq as ToBytes>::write(&state_new1[i], &mut input_state[..])?;
    }

    verifier_state_data.current_round_index = current_round_index.try_into().unwrap();
    verifier_state_data.current_round = current_round.try_into().unwrap();
    // verifier_state_data.current_instruction_index  +=1;
    let mut tmp_state = vec![0u8; 96];
    for (i, elem) in state_final.iter().enumerate() {
        for (j, inner_elem) in elem.iter().enumerate() {
            tmp_state[i * 32 + j] = inner_elem.clone();
        }
    }
    verifier_state_data.state = tmp_state.try_into().unwrap();

    Ok(())
}

pub fn poseidon_1(verifier_state_data: &mut MerkleTreeUpdateState) -> Result<()> {
    let mut current_round_index = verifier_state_data.current_round_index;
    let mut current_round = verifier_state_data.current_round;
    let mds = poseidon_round_constants_split::get_mds_poseidon_circom_bn254_x5_3();

    let mut state_new1 = Vec::new();
    for i in verifier_state_data.state.chunks(32) {
        state_new1.push(<Fq as FromBytes>::read(&i[..]).unwrap());
    }

    for _i in 0..4 {
        let rounds = poseidon_round_constants_split::get_rounds_poseidon_circom_bn254_x5_3_split(
            current_round_index.try_into().unwrap(),
        );
        let params = PoseidonParameters::<Fq>::new(rounds, mds.clone());
        state_new1 =
            permute_custom_split(&params, state_new1, current_round.try_into().unwrap(), 6)
                .unwrap();
        current_round += 6;
        current_round_index += 1;
    }
    let mut state_final = vec![vec![0u8; 32]; 3];
    for (i, input_state) in state_final.iter_mut().enumerate() {
        <Fq as ToBytes>::write(&state_new1[i], &mut input_state[..])?;
    }
    verifier_state_data.current_round_index = current_round_index;
    verifier_state_data.current_round = current_round;
    // verifier_state_data.current_instruction_index +=1;

    let mut tmp_state = vec![0u8; 96];
    for (i, elem) in state_final.iter().enumerate() {
        for (j, inner_elem) in elem.iter().enumerate() {
            tmp_state[i * 32 + j] = inner_elem.clone();
        }
    }
    verifier_state_data.state = tmp_state.try_into().unwrap();

    Ok(())
}

pub fn poseidon_2(verifier_state_data: &mut MerkleTreeUpdateState) -> Result<()> {
    let mut current_round_index: usize =
        verifier_state_data.current_round_index.try_into().unwrap();
    let mut current_round: usize = verifier_state_data.current_round.try_into().unwrap();

    let mds = poseidon_round_constants_split::get_mds_poseidon_circom_bn254_x5_3();
    let rounds = poseidon_round_constants_split::get_rounds_poseidon_circom_bn254_x5_3_split(
        current_round_index,
    );
    let params = PoseidonParameters::<Fq>::new(rounds, mds.clone());

    let mut state_new1 = Vec::new();
    for i in verifier_state_data.state.chunks(32) {
        state_new1.push(<Fq as FromBytes>::read(&i[..]).unwrap());
    }
    state_new1 = permute_custom_split(&params, state_new1, current_round, 6).unwrap();
    current_round += 6;
    current_round_index += 1;

    let rounds = poseidon_round_constants_split::get_rounds_poseidon_circom_bn254_x5_3_split(
        current_round_index,
    );
    let params = PoseidonParameters::<Fq>::new(rounds, mds.clone());
    state_new1 = permute_custom_split(&params, state_new1, current_round, 6).unwrap();
    current_round += 6;
    current_round_index += 1;

    let rounds = poseidon_round_constants_split::get_rounds_poseidon_circom_bn254_x5_3_split(
        current_round_index,
    );
    let params = PoseidonParameters::<Fq>::new(rounds, mds.clone());
    state_new1 = permute_custom_split(&params, state_new1, current_round, 3).unwrap();
    current_round += 3;
    current_round_index += 1;

    let rounds = poseidon_round_constants_split::get_rounds_poseidon_circom_bn254_x5_3_split(
        current_round_index,
    );
    let params = PoseidonParameters::<Fq>::new(rounds, mds.clone());
    state_new1 = permute_custom_split(&params, state_new1, current_round, 4).unwrap();

    let mut state_final = vec![vec![0u8; 32]; 3];
    for (i, input_state) in state_final.iter_mut().enumerate() {
        <Fq as ToBytes>::write(&state_new1[i], &mut input_state[..])?;
    }

    // verifier_state_data.current_instruction_index +=1;
    let mut tmp_state = vec![0u8; 96];
    for (i, elem) in state_final.iter().enumerate() {
        for (j, inner_elem) in elem.iter().enumerate() {
            tmp_state[i * 32 + j] = inner_elem.clone();
        }
    }
    verifier_state_data.state = tmp_state.try_into().unwrap();

    Ok(())
}

//foundational functions for instructions
pub fn prepare_inputs(
    _parameters: &PoseidonParameters<Fq>,
    left_input: &[u8],
    right_input: &[u8],
) -> Result<Vec<Fq>> {
    //modified from arkworks_gadgets

    const INPUT_SIZE_BITS: usize =
        ark_ff::biginteger::BigInteger256::NUM_LIMBS * 8 * PoseidonCircomRounds3::WIDTH * 8;
    const LEFT_INPUT_SIZE_BITS: usize = INPUT_SIZE_BITS / 2;
    assert_eq!(left_input.len(), right_input.len());
    assert!(left_input.len() * 8 <= LEFT_INPUT_SIZE_BITS);
    let chained: Vec<_> = left_input
        .iter()
        .chain(right_input.iter())
        .copied()
        .collect();
    msg!("chained: {:?}", chained);
    let f_inputs = utils::to_field_elements(&chained).unwrap();
    if f_inputs.len() >= PoseidonCircomRounds3::WIDTH {
        panic!(
            "incorrect input length {:?} for width {:?} -- input bits {:?}",
            f_inputs.len(),
            PoseidonCircomRounds3::WIDTH,
            chained.len()
        );
    }

    let mut buffer = vec![Fq::zero()];
    for f in f_inputs {
        buffer.push(f);
    }
    Ok(buffer)
}

pub fn permute_custom_split(
    params: &PoseidonParameters<Fq>,
    mut state: Vec<Fq>,
    nr_start: usize,
    nr_iterations: usize,
) -> Result<Vec<Fq>> {
    //modified from arkworks_gadgets
    let nr_end = nr_start + nr_iterations;

    for r in nr_start..nr_end {
        state.iter_mut().enumerate().for_each(|(i, a)| {
            let c = params.round_keys[((r - nr_start) * PoseidonCircomRounds3::WIDTH + i)];
            a.add_assign(c);
        });

        let half_rounds = PoseidonCircomRounds3::FULL_ROUNDS / 2;
        if r < half_rounds || r >= (half_rounds + PoseidonCircomRounds3::PARTIAL_ROUNDS) {
            state
                .iter_mut()
                .try_for_each(|a| PoseidonCircomRounds3::SBOX.apply_sbox(*a).map(|f| *a = f))
                .unwrap();
        } else {
            state[0] = PoseidonCircomRounds3::SBOX.apply_sbox(state[0]).unwrap();
        }

        state = state
            .iter()
            .enumerate()
            .map(|(i, _)| {
                state.iter().enumerate().fold(Fq::zero(), |acc, (j, a)| {
                    let m = params.mds_matrix[i][j];
                    acc.add(m.mul(*a))
                })
            })
            .collect();
    }
    Ok(state)
}
/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::poseidon_merkle_tree::state::MerkleTreeUpdateState;
    use ark_ff::{BigInteger, Fp256, PrimeField};
    use ark_std::{test_rng, UniformRand};
    use arkworks_gadgets::poseidon::circom::CircomCRH;

    use arkworks_gadgets::utils::{
        get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3,
    };

    use ark_crypto_primitives::crh::TwoToOneCRH;

    pub type PoseidonCircomCRH3 = CircomCRH<Fq, PoseidonCircomRounds3>;

    const INSTRUCTION_ORDER_POSEIDON_2_INPUTS: [u8; 12] = [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3];

    //defining processor function for testing
    pub fn processor_poseidon(id: u8, account_struct: &mut MerkleTreeUpdateState) {
        // if id == 0 {
        //     permute_instruction_first(
        //         &mut account_struct.state,
        //         &mut account_struct.current_round,
        //         &mut account_struct.current_round_index,
        //         &account_struct.left,
        //         &account_struct.right,
        //     )
        //     .unwrap();
        // } else if id == 1 {
        //     permute_instruction_6(
        //         &mut account_struct.state,
        //         &mut account_struct.current_round,
        //         &mut account_struct.current_round_index,
        //     )
        //     .unwrap();
        // } else if id == 2 {
        //     permute_instruction_3(
        //         &mut account_struct.state,
        //         &mut account_struct.current_round,
        //         &mut account_struct.current_round_index,
        //     )
        //     .unwrap();
        // } else if id == 3 {
        //     permute_instruction_last(
        //         &mut account_struct.state,
        //         &mut account_struct.current_round,
        //         &mut account_struct.current_round_index,
        //     )
        //     .unwrap();
        // }
                if id == 0 {
            poseidon_0(verifier_state_data)?;
        } else if id == 1 {
            poseidon_1(verifier_state_data)?;
        } else if id == 2 {
            poseidon_2(verifier_state_data)?;
        }
        Ok(())
    }


    #[test]
    fn offchain_test_poseidon_hash_instructions() {
        let rounds = get_rounds_poseidon_circom_bn254_x5_3::<Fq>();
        let mds = get_mds_poseidon_circom_bn254_x5_3::<Fq>();
        let params = PoseidonParameters::<Fq>::new(rounds, mds);
        //perform the test 1000x
        for _j in 0..1000 {
            //generating random test input
            let mut rng = test_rng();
            let left_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng)
                .into_repr()
                .to_bytes_le();
            let right_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng)
                .into_repr()
                .to_bytes_le();

            //generating reference poseidon hash with library to test against
            let poseidon_res =
                <PoseidonCircomCRH3 as TwoToOneCRH>::evaluate(&params, &left_input, &right_input)
                    .unwrap();

            //parsing reference hash to bytes
            let mut out_bytes = [0u8; 32];
            <Fq as ToBytes>::write(&poseidon_res, &mut out_bytes[..]).unwrap();

            //initing struct which similates onchain account for instructions
            let mut account_struct = MerkleTreeUpdateState {
                is_initialized: true,
                merkle_tree_index: 0,
                state: vec![vec![0u8; 32]; 3],
                current_round: 0,
                current_round_index: 0,
                node_left: left_input.to_vec(),
                node_right: right_input.to_vec(),
                left: left_input.to_vec(),
                right: right_input.to_vec(),
                current_level_hash: vec![0u8; 32],
                current_index: 0usize,
                current_level: 0usize,
                current_instruction_index: 0usize,
                encrypted_utxos: vec![0],
            };

            //executing poseidon instructions
            for i in INSTRUCTION_ORDER_POSEIDON_2_INPUTS {
                processor_poseidon(i, &mut account_struct);
            }

            assert_eq!(out_bytes.to_vec(), account_struct.state[0]);
        }
    }

    #[test]
    fn offchain_test_poseidon_hash_fails() {
        let rounds = get_rounds_poseidon_circom_bn254_x5_3::<Fq>();
        let mds = get_mds_poseidon_circom_bn254_x5_3::<Fq>();
        let params = PoseidonParameters::<Fq>::new(rounds, mds);
        //perform the test 1000x
        for _j in 0..1000 {
            //generating random test input
            let mut rng = test_rng();
            let left_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng)
                .into_repr()
                .to_bytes_le();
            let right_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng)
                .into_repr()
                .to_bytes_le();

            //generating reference poseidon hash with library to test against
            let poseidon_res =
                <PoseidonCircomCRH3 as TwoToOneCRH>::evaluate(&params, &left_input, &right_input)
                    .unwrap();

            //parsing reference hash to bytes
            let mut out_bytes = [0u8; 32];
            <Fq as ToBytes>::write(&poseidon_res, &mut out_bytes[..]).unwrap();

            //generating different random test input for second hash
            let right_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng)
                .into_repr()
                .to_bytes_le();

            //initing struct which similates onchain account for instructions
            let mut account_struct = MerkleTreeUpdateState {
                is_initialized: true,
                merkle_tree_index: 0u8,
                state: vec![vec![0u8; 32]; 3],
                current_round: 0,
                current_round_index: 0,
                node_left: left_input.to_vec(),
                node_right: right_input.to_vec(),
                left: left_input.to_vec(),
                right: right_input.to_vec(),
                current_level_hash: vec![0u8; 32],
                current_index: 0usize,
                current_level: 0usize,
                current_instruction_index: 0usize,
                encrypted_utxos: vec![0],
            };

            //executing poseidon instructions
            for i in INSTRUCTION_ORDER_POSEIDON_2_INPUTS {
                processor_poseidon(i, &mut account_struct);
            }

            assert!(out_bytes.to_vec() != account_struct.state[0]);
        }
    }
}
*/
