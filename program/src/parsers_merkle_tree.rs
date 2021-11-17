use ark_sponge::poseidon::*;
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ed_on_bls12_381::EdwardsProjective as Edwards;
use ark_ff::{fields, models::Fp256};
use ark_ed_on_bls12_381;
use ark_sponge::CryptographicSponge;

use crate::poseidon_params::{get_params};


// parsers:
pub fn parse_state_to_bytes(state: &Vec<Fp256::<ark_ed_on_bls12_381::FqParameters>>, range_1: &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>){
    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&state[0], &mut range_1[..]);
    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&state[1], &mut range_2[..]);
    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&state[2], &mut range_3[..]);
}

pub fn parse_sponge_from_bytes(range_1: &Vec<u8>, range_2: &Vec<u8>, range_3: &Vec<u8>) -> ark_sponge::poseidon::PoseidonSponge::<ark_ed_on_bls12_381::Fq>{
    // build base sponge:
    let mut parameters = get_params();
    let mut sponge = ark_sponge::poseidon::PoseidonSponge::<ark_ed_on_bls12_381::Fq>::new(&parameters);

    let one = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&range_1[..]).unwrap();
    let two = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&range_2[..]).unwrap();
    let three = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&range_3[..]).unwrap();

    // assign current state to sponge:
    sponge.state = vec![one,two,three];
    sponge
}