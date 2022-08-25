use crate::errors::ErrorCode;
use crate::groth16_verifier::VerifierState;
use crate::groth16_verifier::_process_instruction;
use crate::groth16_verifier::init_pairs_instruction;
use crate::groth16_verifier::parse_proof_b_from_bytes;
use crate::groth16_verifier::parse_r_to_bytes;
use anchor_lang::prelude::*;
use ark_ec::bn::g2::G2HomProjective;
// use ark_ed_on_bn254::{Fq, Fr};
use ark_bn254::{Fq, Fr};

use ark_ff::Fp2;
use ark_ff::PrimeField;
use ark_std::{One, Zero};
use ark_ff::Fp256;
use solana_program::alt_bn128::{
    alt_bn128_addition,
    alt_bn128_pairing,
    alt_bn128_multiplication
};
use crate::utils::verification_key::{
    VK_IC,
    VK_ALPHA_G1,
    VK_BETA_G2,
    VK_GAMMA_G2,
    VK_DELTA_G2,
};
#[derive(Accounts)]
#[instruction(
    proof:              [u8;256],
    merkle_root:        [u8;32],
    amount:             [u8;32],
    tx_integrity_hash:  [u8;32]
)]
pub struct CreateVerifierState<'info> {
    #[account(init_if_needed, seeds = [tx_integrity_hash.as_ref(), b"storage"], bump,  payer=signing_address, space= 5 * 1024)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    /// First time therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn process_create_verifier_state(
    ctx: Context<CreateVerifierState>,
    proof: [u8; 256],
    merkle_root: [u8; 32],
    amount: [u8; 32],
    tx_integrity_hash: [u8; 32],
    nullifier0: [u8; 32],
    nullifier1: [u8; 32],
    leaf_right: [u8; 32],
    leaf_left: [u8; 32],
    recipient: [u8; 32],
    ext_amount: [u8; 8],
    _relayer: [u8; 32],
    relayer_fee: [u8; 8],
    encrypted_utxos: [u8; 256],
    merkle_tree_index: [u8; 1],
) -> Result<()> {
    // If not initialized this will run load_init.
    let verifier_state_data = &mut match ctx.accounts.verifier_state.load_mut() {
        Ok(res) => {
            // Checking that all values are the same as in create escrow.
            if res.signing_address == ctx.accounts.signing_address.key() &&
                res.merkle_tree_index == merkle_tree_index[0] &&
                res.ext_amount == ext_amount &&
                res.tx_integrity_hash == tx_integrity_hash &&
                res.relayer_fee == u64::from_le_bytes(relayer_fee.try_into().unwrap()).clone() {
                Ok(res)
            } else {
                err!(ErrorCode::WrongSigner)
            }
        }
        Err(_) => ctx.accounts.verifier_state.load_init(),
    }?;
    // Verifier state accounts should only be initialized once.
    if verifier_state_data.current_instruction_index > 0 {
        return err!(ErrorCode::VerifierStateAlreadyInitialized);
    }
    verifier_state_data.signing_address = ctx.accounts.signing_address.key();
    verifier_state_data.tx_integrity_hash = tx_integrity_hash.clone();
    verifier_state_data.merkle_root = merkle_root.clone();
    verifier_state_data.amount = amount.clone();
    verifier_state_data.merkle_tree_index = merkle_tree_index[0].clone();
    verifier_state_data.relayer_fee = u64::from_le_bytes(relayer_fee.try_into().unwrap()).clone();
    verifier_state_data.recipient = Pubkey::new(&recipient).clone();
    verifier_state_data.ext_amount = ext_amount.clone();
    verifier_state_data.fee = relayer_fee.clone();
    verifier_state_data.leaf_left = leaf_left;
    verifier_state_data.leaf_right = leaf_right;
    verifier_state_data.nullifier0 = nullifier0;
    verifier_state_data.nullifier1 = nullifier1;
    verifier_state_data.encrypted_utxos = encrypted_utxos[..222].try_into().unwrap();
    /*let public_inputs = [
        root,
        nullifier0,
        nullifier1,
        leaf_left,
        leaf_right,
        publicAmount,
        extDataHash,
        feeAmount,
        mintPubkey
    ].concat();
    // Initing pairs to prepare inputs.
    // init_pairs_instruction(verifier_state_data)?;
    // _process_instruction(
    //     41,
    //     verifier_state_data,
    //     usize::try_from(verifier_state_data.current_index).unwrap(),
    // )?;
    // verifier_state_data.current_index = 1;
    // verifier_state_data.current_instruction_index = 1;
    // verifier_state_data.computing_prepared_inputs = true;

    // miller loop
    // verifier_state_data.proof_a_bytes = proof[0..64].try_into().unwrap();
    // verifier_state_data.proof_b_bytes = proof[64..64 + 128].try_into().unwrap();
    // verifier_state_data.proof_c_bytes = proof[64 + 128..256].try_into().unwrap();

    // preparing inputs
     let mut public_inputs_res_bytes = VK_IC[0];
     for i in 0..VK_IC {
         let input_mul_bytes = [VK_IC[i], public_inputs[i]].concat();
         let mul_res = alt_bn128_multiplication(&input_mul_bytes[..]);
         let input_addition_bytes = [mul_res, public_inputs_res_bytes].concat();
         // .add_assign(&b.mul(i.into_repr()));
         public_inputs_res_bytes = alt_bn128_addition(&input_addition_bytes[..]);
     }
     msg!("public_inputs_res_bytes: {:?}", public_inputs_res_bytes);
    let pairing_input = [
        proof[0..64].to_vec(), // proof_a
        proof[64..64 + 128].to_vec(), // proof_b
        public_inputs_bytes.to_vec(),
        VK_GAMMA_G2.to_vec(),
        proof[64 + 128..256].to_vec(), // proof_c
        VK_DELTA_G2.to_vec(),
        VK_ALPHA_G1.to_vec(),
        VK_BETA_G2.to_vec(),
    ].concat();
    let pairing_res = alt_bn128_pairing(&pairing_input[..]);
    if pairing_res != 1 {
        return err!(ErrorCode::ProofVerificationFailed);
    }*/
    Ok(())
    // check_tx_integrity_hash(
    //     verifier_state_data.recipient.to_bytes().to_vec(),
    //     verifier_state_data.ext_amount.to_vec(),
    //     verifier_state_data
    //         .signing_address
    //         .key()
    //         .to_bytes()
    //         .to_vec(),
    //     verifier_state_data.fee.to_vec(),
    //     verifier_state_data.tx_integrity_hash.to_vec(),
    //     verifier_state_data.merkle_tree_index,
    //     verifier_state_data.encrypted_utxos[..222].to_vec(),
    //     merkle_tree_program::utils::config::MERKLE_TREE_ACC_BYTES_ARRAY
    //         [usize::try_from(verifier_state_data.merkle_tree_index).unwrap()]
    //         .0
    //         .to_vec(),
    // )
}

fn check_tx_integrity_hash(
    recipient: Vec<u8>,
    ext_amount: Vec<u8>,
    relayer: Vec<u8>,
    fee: Vec<u8>,
    tx_integrity_hash: Vec<u8>,
    merkle_tree_index: u8,
    encrypted_utxos: Vec<u8>,
    merkle_tree_pda_pubkey: Vec<u8>,
) -> Result<()> {
    let input = [
        recipient,
        ext_amount,
        relayer,
        fee,
        merkle_tree_pda_pubkey,
        vec![merkle_tree_index],
        encrypted_utxos,
    ]
    .concat();
    msg!("integrity_hash inputs: {:?}", input);
    let hash = solana_program::keccak::hash(&input[..]).try_to_vec()?;
    msg!("hash computed {:?}", hash);

    if Fq::from_be_bytes_mod_order(&hash[..]) != Fq::from_le_bytes_mod_order(&tx_integrity_hash) {
        msg!(
            "tx_integrity_hash verification failed.{:?} != {:?}",
            &hash[..],
            &tx_integrity_hash
        );
        return err!(ErrorCode::WrongTxIntegrityHash);
    }
    Ok(())
}
#[cfg(test)]
mod test {
    use super::*;
    use ark_ff::{BigInteger, bytes::{FromBytes, ToBytes}};
    use ark_ec::AffineCurve;
    use ark_ec::ProjectiveCurve;
    use std::ops::AddAssign;
    use ark_ff::FpParameters;
    use std::ops::MulAssign;
    use ark_ff::BigInteger256;

    // use ark_groth16::prepare_inputs;
    #[test]
    fn test_multiplication() {

        let public_inputs = [231,174,226,37,211,160,187,178,149,82,17,60,110,116,28,61,58,145,58,71,25,42,67,46,189,214,248,234,182,251,238,34,0,202,154,59,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,225,157,11,252,221,230,8,141,243,173,43,5,181,92,233,158,1,49,222,73,181,162,6,187,38,215,115,133,129,28,41,33,64,66,15,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,31,11,137,87,252,84,250,28,95,106,202,3,89,36,51,65,87,13,68,84,74,168,117,74,173,9,245,77,76,208,8,43,9,104,56,69,0,210,158,191,124,224,87,221,60,245,64,77,144,7,188,85,172,210,50,118,177,19,152,107,59,12,91,18,91,254,46,62,123,95,171,253,40,21,64,207,111,160,248,60,12,79,137,212,36,211,220,186,107,150,211,98,38,138,17,11,6,157,54,154,53,7,47,129,189,27,245,196,6,142,80,113,42,122,200,199,126,246,182,237,223,200,251,91,92,40,239,9];
        let input_mul_bytes = [to_be_64(&VK_IC[1]).to_vec(), vec![0u8;32]].concat();

        let mul_res_syscall = alt_bn128_multiplication(&input_mul_bytes[..]).unwrap();
        let input_addition_bytes= [to_be_64(&VK_IC[0]).to_vec(), mul_res_syscall.clone().to_vec()].concat();

        let addition_res_syscall = alt_bn128_addition(&input_addition_bytes[..]).unwrap();

        let mut g_ic = <G1 as FromBytes>::read(&*[&VK_IC[0][..], &[0u8][..]].concat()).unwrap();

        let mut g_ic_1 = <G1 as FromBytes>::read(&*[&VK_IC[2][..], &[0u8][..]].concat()).unwrap().into_projective();
        // BigInteger256::new([0,0,0,0]).into()
        g_ic_1.mul_assign(Fp256::<ark_ed_on_bn254::FqParameters>::zero());
        let mut mul_res_ark_bytes = [0u8;64];
        <G1 as ToBytes>::write(&g_ic_1.into(),&mut mul_res_ark_bytes[..]);
        // BigInteger256::zero();
        // g_ic.add_assign(&g_ic_1);
        println!("p ark {:?}", g_ic);
        println!("q ark {:?}", g_ic_1.into_affine());
        let res = g_ic + g_ic_1.into_affine();
        let mut addition_res_ark_bytes = [0u8;64];
        <G1 as ToBytes>::write(&res.into(),&mut addition_res_ark_bytes[..]);
        println!("mul_res_syscall{:?}", mul_res_syscall);
        println!("to_be_64(&mul_res_ark_bytes[..]) {:?}",to_be_64(&mul_res_ark_bytes[..]) );
        assert_eq!(mul_res_syscall, to_be_64(&mul_res_ark_bytes[..]));
        assert_eq!(addition_res_syscall, to_be_64(&addition_res_ark_bytes[..]));
        println!("g1 zero{:?}",G1::zero() );

        // g_ic.add_assign(&b.mul(scalar.into_repr()));


    }
    type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>;
    type G1p = ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bn254::g1::Parameters>;
    type G2 = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g2::Parameters>;

    #[test]
    fn test_groth16_verification() {

        // original public inputs the all 0 element throws a group error
        // let public_inputs = [34,238,251,182,234,248,214,189,46,67,42,25,71,58,145,58,61,28,116,110,60,17,82,149,178,187,160,211,37,226,174,231,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,59,154,202,0,43,223,170,106,86,191,3,134,169,166,97,179,10,139,71,201,124,116,122,168,7,166,16,82,87,87,55,138,100,65,144,63,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,15,66,64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,42,193,192,156,15,46,99,214,68,44,64,245,153,95,88,47,59,97,174,9,81,73,224,59,175,90,81,176,130,35,75,65,29,25,86,66,122,132,239,36,216,86,2,150,23,205,25,62,124,65,157,152,212,7,0,36,58,27,199,147,203,0,75,247,17,165,151,106,130,197,203,27,237,151,250,137,37,238,192,5,166,225,6,33,133,86,177,4,157,118,125,201,22,195,106,9,41,29,214,42,35,223,191,115,24,160,192,52,55,2,154,201,186,194,34,3,155,134,210,36,91,144,30,243,80,76,197,199];
        println!("{:?}",Fq::zero().into_repr().to_bytes_be() );
        let public_inputs = [231,174,226,37,211,160,187,178,149,82,17,60,110,116,28,61,58,145,58,71,25,42,67,46,189,214,248,234,182,251,238,34,0,202,154,59,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,147,133,167,234,114,214,160,62,253,149,162,29,94,213,10,163,230,52,212,43,160,89,66,87,200,156,67,245,97,229,199,41,64,66,15,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,236,115,74,30,48,235,24,244,215,152,178,245,113,207,157,65,217,3,33,32,79,12,67,40,61,184,108,88,134,172,216,6,125,164,181,19,123,95,136,53,123,117,255,137,200,164,3,157,71,120,195,252,19,175,35,169,175,38,78,220,245,223,193,33,88,98,12,176,149,34,52,121,191,150,0,126,161,63,17,81,206,125,197,93,76,129,246,73,59,88,94,30,46,203,40,13,145,189,53,69,231,100,68,245,250,86,57,217,161,216,32,47,23,118,254,137,159,204,121,196,152,192,186,195,72,80,36,3];
        let proof = [103,10,220,223,94,148,102,79,96,125,1,194,227,55,125,114,214,160,29,210,5,69,113,72,37,165,142,127,216,11,4,34,132,151,253,204,34,180,23,111,37,16,189,219,212,35,246,106,154,75,5,233,252,92,74,127,144,37,153,138,90,49,237,22,91,71,254,230,62,149,227,57,117,183,245,47,252,158,107,97,188,141,210,28,161,30,123,142,150,12,102,202,214,138,241,36,137,122,26,68,165,124,59,92,150,118,53,74,160,30,222,167,253,243,194,5,153,156,23,24,114,250,138,164,31,143,132,7,225,219,219,111,110,69,42,227,5,6,250,194,123,158,227,168,117,176,158,228,82,202,217,41,180,186,181,226,147,219,180,40,113,21,24,141,236,132,163,63,73,209,125,152,146,100,200,84,151,241,33,49,176,42,41,11,4,225,197,203,0,142,195,39,8,35,95,133,40,240,204,82,56,113,121,170,83,203,2,116,127,31,223,135,198,83,146,16,170,91,70,4,15,12,141,10,169,126,159,174,46,127,58,251,221,234,157,113,155,84,207,243,146,157,169,34,76,241,70,224,57,50,42,104,1,181,197,42];
        // let mut g_ic = <G1 as FromBytes>::read(&*[&*to_be_64(&VK_IC[0][..]), &[0u8][..]].concat()).unwrap().into_projective();
        // for (i, input) in public_inputs.chunks(32).enumerate() {
        //     if i != 0{
        //         let scalar = <Fq as FromBytes>::read(&*to_be_64(&input[..])).unwrap();
        //         let b = <G1 as FromBytes>::read(&*[&*to_be_64(&VK_IC[i][..]), &[0u8][..]].concat()).unwrap().into_projective();
        //         g_ic.add_assign(&b.mul(scalar.into_repr()));
        //     }
        // }
        let mut public_inputs_ark = Vec::new();
        for (i, input) in public_inputs.chunks(32).enumerate() {
            let scalar = <Fp256<ark_bn254::FqParameters> as FromBytes> ::read(&input[..]).unwrap();
            public_inputs_ark.push(scalar);
        }
        // let prepared_inputs = prepare_inputs(&pvk, &public_inputs_ark[..]).unwrap();
        let mut g_ic = <G1 as FromBytes>::read(&*[&VK_IC[0][..], &[0u8][..]].concat()).unwrap().into_projective();
        // for (i, input) in public_inputs.chunks(32).enumerate() {
        //     // let scalar = <Fr as FromBytes>::read(&input[..]).unwrap();
        //     println!("g_ic{}", g_ic);
        //
        //     let scalar = <Fp256<ark_bn254::FqParameters> as FromBytes> ::read(&input[..]).unwrap();
        //     // let scalar = <Fq as FromBytes>::read(&*to_be_64(&input[..])).unwrap();
        //     let b = <G1 as FromBytes>::read(&*[&VK_IC[i+1][..], &[0u8][..]].concat()).unwrap().into_projective();
        //     println!("b {}", b.into_affine());
        //     println!("scalar{}", scalar);
        //     g_ic.add_assign(&b.mul(scalar.into_repr()));
        //
        //
        // }
        let mut g_ic_affine_bytes = [0u8;64];
        <G1 as ToBytes>::write(&g_ic.into_affine(),&mut g_ic_affine_bytes[..]);

        // let mut g_ic_bytes :[u8;96] = [0u8;96];
        // <G1p as ToBytes>::write(&g_ic,&mut g_ic_bytes[..]);
        // assert_eq!(snarkjs_public_inputs, g_ic_bytes);

        // let mut g_ic = pvk.vk.gamma_abc_g1[0].into_projective();
        // for (i, b) in public_inputs.iter().zip(pvk.vk.gamma_abc_g1.iter().skip(1)) {
        //     g_ic.add_assign(&b.mul(i.into_repr()));
        // }
        // let snarkjs_public_inputs_be = to_be_64(&snarkjs_public_inputs[..]);
        let mut public_inputs_res_bytes = to_be_64(&VK_IC[0]);
        for (i, input) in public_inputs.chunks(32).enumerate() {
            let scalar = <Fp256<ark_bn254::FqParameters> as FromBytes> ::read(&input[..]).unwrap();
            let b = <G1 as FromBytes>::read(&*[&VK_IC[i+1][..], &[0u8][..]].concat()).unwrap().into_projective();
            println!("b {:?}", b.into_affine());
            println!("scalar {:?}", scalar);
            println!("p ark {:?}", g_ic);
            let mul_res_ark = b.mul(scalar.into_repr());
            println!("mul_res_ark {:?}", mul_res_ark.into_affine());
            g_ic.add_assign(&mul_res_ark);


            let input_mul_bytes = [to_be_64(&VK_IC[i+1]).to_vec(), (to_be_64(&input)).to_vec()].concat();
            let mul_res = alt_bn128_multiplication(&input_mul_bytes[..]).unwrap();
            println!("mul_res {:?}",<G1 as FromBytes>::read(&*to_be_64(&mul_res[..])) );
            let input_addition_bytes= [mul_res, public_inputs_res_bytes.to_vec()].concat();
            // .add_assign(&b.mul(i.into_repr()));
            public_inputs_res_bytes = alt_bn128_addition(&input_addition_bytes[..]).unwrap().try_into().unwrap();
            println!("public_inputs_res_bytes {:?}",<G1 as FromBytes>::read(&*[&*to_be_64(&public_inputs_res_bytes[..]).to_vec(), &vec![0]].concat()).unwrap() );
            println!("iteration {}",i);
            assert_eq!(<G1 as FromBytes>::read(&*[&*to_be_64(&public_inputs_res_bytes[..]).to_vec(), &vec![0]].concat()).unwrap(), g_ic);
        }
        // assert_eq!(public_inputs_res_bytes, to_be_64(&g_ic_affine_bytes));
        // assert_eq!(snarkjs_public_inputs_be, public_inputs_res_bytes);
        println!("public_inputs_res_bytes: {:?}", public_inputs_res_bytes);
        // let mut affine_public_inputs_snarkjs_rs_bytes = [0u8;64];
        // let affine_public_inputs_snarkjs_rs = <G1p as FromBytes>::read(&*[snarkjs_public_inputs.to_vec(), vec![0u8]].concat()).unwrap().into_affine();
        // println!("affine_public_inputs_snarkjs_rs {:?}", affine_public_inputs_snarkjs_rs);
        // assert!(affine_public_inputs_snarkjs_rs.is_on_curve(), "not on curve");
        // <G1 as ToBytes>::write(&affine_public_inputs_snarkjs_rs, &mut affine_public_inputs_snarkjs_rs_bytes[..]);
        println!("public_inputs_res_bytes {:?}", public_inputs_res_bytes);
        // println!("affine_public_inputs_snarkjs_rs_bytes {:?}", affine_public_inputs_snarkjs_rs_bytes);


       // // assert!(pairing_res == 1);
       // println!("{:?}",pairing_res);
       let proof_a: G1 =  <G1 as FromBytes>::read(&*[&proof[0..64][..], &[0u8][..]].concat()).unwrap();
       let proof_b: G2 =  <G2 as FromBytes>::read(&*[&proof[64..192][..], &[0u8][..]].concat()).unwrap();

       let g_ic: G1 = g_ic.into();
       let gamma_g2_neg_pc: G2 =  <G2 as FromBytes>::read(&*[&VK_GAMMA_G2[..], &[0u8][..]].concat()).unwrap();

       let delta_g2_neg_pc: G2 =  <G2 as FromBytes>::read(&*[&VK_DELTA_G2[..], &[0u8][..]].concat()).unwrap();
       let proof_c: G1 =  <G1 as FromBytes>::read(&*[&proof[192..256][..], &[0u8][..]].concat()).unwrap();

       let alpha_g1: G1 =  <G1 as FromBytes>::read(&*[&VK_ALPHA_G1[..], &[0u8][..]].concat()).unwrap();
       let beta_g2 : G2 =  <G2 as FromBytes>::read(&*[&VK_BETA_G2[..], &[0u8][..]].concat()).unwrap();


       let miller_output_ref =
       <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::miller_loop(
           [
               (proof_a.neg().into(), proof_b.into()),
               (
                   g_ic.into(),
                   gamma_g2_neg_pc.clone().into(),
               ),
               (proof_c.into(), delta_g2_neg_pc.clone().into()),
               (alpha_g1.into(), beta_g2.into())
           ]
           .iter(),
       );
       let fe_output_ref = <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::final_exponentiation(&miller_output_ref);
       println!("fe_output_ref {:?}", fe_output_ref);
       type GT = <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk;

       assert_eq!(fe_output_ref.unwrap(),GT::one());

       let mut proof_a_neg = [0u8;64];
       <G1 as ToBytes>::write(&proof_a.neg(), &mut proof_a_neg[..]);

       let pairing_input = [
           to_be_64(&proof_a_neg).to_vec(), // proof_a
           to_be_128(&proof[64..64 + 128]).to_vec(), // proof_b
           (public_inputs_res_bytes).to_vec(),
           // to_be_64(&affine_public_inputs_snarkjs_rs_bytes).to_vec(),
           to_be_128(&VK_GAMMA_G2).to_vec(),
           to_be_64(&proof[64 + 128..256]).to_vec(), // proof_c
           to_be_128(&VK_DELTA_G2).to_vec(),
           to_be_64(&VK_ALPHA_G1).to_vec(),
           to_be_128(&VK_BETA_G2).to_vec(),
       ].concat();
       let pairing_res = alt_bn128_pairing(&pairing_input[..]).unwrap();
       println!("paring res {:?}",pairing_res );
       assert_eq!(pairing_res[31], 1);
    }
    use std::ops::Neg;

    fn to_be_64(bytes: &[u8]) -> Vec<u8> {
        let mut vec = Vec::new();
        for b in bytes.chunks(32) {
            for byte in b.iter().rev() {
                vec.push(*byte);
            }
        }
        vec
    }

    fn to_be_128(bytes: &[u8]) -> Vec<u8> {
        let mut vec = Vec::new();
        for b in bytes.chunks(64) {
            for byte in b.iter().rev() {
                vec.push(*byte);
            }
        }
        vec
    }


    const TEST_DATA: [[([u8; 64], [u8; 128]); 3]; 3] = [
        [
            (
                [
                    169, 188, 126, 23, 234, 181, 49, 44, 76, 155, 186, 163, 180, 151, 19, 153, 6, 220,
                    171, 29, 119, 54, 44, 34, 82, 130, 81, 172, 144, 32, 252, 41, 51, 218, 77, 129,
                    230, 75, 37, 139, 138, 25, 61, 229, 38, 121, 209, 134, 47, 83, 24, 40, 105, 229,
                    156, 143, 191, 172, 172, 88, 204, 23, 187, 29,
                ],
                [
                    133, 52, 151, 123, 19, 114, 157, 14, 21, 62, 189, 188, 4, 178, 35, 99, 225, 132,
                    32, 193, 205, 86, 200, 15, 25, 57, 244, 156, 6, 174, 131, 16, 112, 192, 162, 11,
                    208, 105, 38, 25, 207, 152, 137, 184, 141, 148, 183, 25, 137, 165, 117, 9, 241,
                    106, 140, 254, 1, 125, 113, 17, 96, 189, 169, 2, 253, 248, 3, 180, 29, 86, 110, 90,
                    49, 229, 224, 58, 22, 188, 76, 132, 220, 16, 176, 51, 132, 26, 126, 45, 224, 132,
                    17, 56, 248, 37, 12, 7, 23, 2, 42, 116, 42, 173, 235, 102, 244, 191, 177, 1, 93,
                    177, 63, 151, 44, 150, 232, 54, 181, 66, 207, 138, 144, 211, 104, 119, 163, 198, 6,
                    17,
                ],
            ),
            (
                [
                    220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89,
                    0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187,
                    136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133,
                    250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15,
                ],
                [
                    237, 246, 146, 217, 92, 189, 222, 70, 221, 218, 94, 247, 212, 34, 67, 103, 121, 68,
                    92, 94, 102, 0, 106, 66, 118, 30, 31, 18, 239, 222, 0, 24, 194, 18, 243, 174, 183,
                    133, 228, 151, 18, 231, 169, 53, 51, 73, 170, 241, 37, 93, 251, 49, 183, 191, 96,
                    114, 58, 72, 13, 146, 147, 147, 142, 25, 157, 127, 130, 113, 21, 192, 57, 239, 17,
                    247, 45, 92, 40, 131, 175, 179, 205, 23, 182, 243, 53, 212, 164, 109, 62, 50, 165,
                    5, 205, 239, 155, 29, 236, 101, 90, 7, 58, 177, 115, 230, 153, 59, 190, 247, 93,
                    57, 54, 219, 199, 36, 117, 24, 9, 172, 177, 203, 179, 175, 209, 136, 162, 196, 93,
                    39,
                ],
            ),
            (
                [
                    181, 129, 186, 7, 53, 61, 26, 93, 210, 29, 170, 46, 100, 150, 94, 3, 69, 237, 166,
                    21, 152, 146, 211, 52, 142, 103, 21, 166, 133, 176, 141, 24, 57, 122, 149, 35, 146,
                    161, 222, 19, 116, 168, 229, 88, 0, 246, 241, 65, 134, 237, 213, 24, 65, 254, 219,
                    138, 55, 223, 50, 68, 107, 147, 187, 32,
                ],
                [
                    83, 221, 254, 184, 55, 148, 227, 43, 133, 7, 18, 158, 114, 71, 125, 201, 138, 190,
                    192, 0, 56, 234, 29, 190, 13, 53, 55, 124, 65, 213, 82, 16, 190, 225, 85, 93, 216,
                    143, 253, 91, 162, 249, 28, 124, 77, 137, 187, 191, 41, 63, 204, 124, 190, 22, 134,
                    112, 142, 91, 162, 209, 153, 210, 182, 31, 36, 167, 184, 235, 213, 41, 254, 96, 37,
                    227, 187, 127, 87, 12, 115, 172, 212, 196, 214, 182, 240, 132, 194, 165, 181, 15,
                    200, 254, 250, 69, 45, 32, 97, 149, 114, 77, 166, 31, 30, 137, 84, 29, 211, 14,
                    204, 3, 70, 171, 70, 14, 213, 156, 243, 16, 201, 200, 211, 247, 42, 95, 196, 13,
                    58, 48,
                ],
            ),
        ],
        [
            (
                [
                    143, 15, 147, 99, 79, 60, 78, 50, 8, 203, 226, 62, 60, 109, 217, 225, 121, 35, 63,
                    247, 36, 118, 48, 28, 46, 227, 216, 210, 143, 152, 178, 32, 196, 95, 169, 192, 62,
                    112, 118, 209, 62, 38, 48, 221, 92, 177, 39, 6, 209, 164, 125, 146, 25, 41, 79, 58,
                    75, 8, 43, 65, 211, 110, 225, 30,
                ],
                [
                    133, 52, 151, 123, 19, 114, 157, 14, 21, 62, 189, 188, 4, 178, 35, 99, 225, 132,
                    32, 193, 205, 86, 200, 15, 25, 57, 244, 156, 6, 174, 131, 16, 112, 192, 162, 11,
                    208, 105, 38, 25, 207, 152, 137, 184, 141, 148, 183, 25, 137, 165, 117, 9, 241,
                    106, 140, 254, 1, 125, 113, 17, 96, 189, 169, 2, 253, 248, 3, 180, 29, 86, 110, 90,
                    49, 229, 224, 58, 22, 188, 76, 132, 220, 16, 176, 51, 132, 26, 126, 45, 224, 132,
                    17, 56, 248, 37, 12, 7, 23, 2, 42, 116, 42, 173, 235, 102, 244, 191, 177, 1, 93,
                    177, 63, 151, 44, 150, 232, 54, 181, 66, 207, 138, 144, 211, 104, 119, 163, 198, 6,
                    17,
                ],
            ),
            (
                [
                    220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89,
                    0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187,
                    136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133,
                    250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15,
                ],
                [
                    173, 107, 171, 22, 221, 71, 45, 8, 196, 71, 21, 41, 91, 194, 234, 150, 169, 187,
                    191, 168, 232, 15, 151, 135, 154, 78, 26, 82, 238, 227, 241, 40, 226, 243, 148, 20,
                    235, 209, 68, 253, 43, 11, 170, 29, 250, 120, 231, 225, 205, 97, 222, 24, 170, 83,
                    144, 237, 88, 237, 120, 135, 51, 94, 186, 31, 225, 243, 95, 76, 78, 195, 89, 183,
                    200, 17, 179, 211, 10, 171, 25, 250, 102, 190, 107, 2, 80, 178, 187, 180, 75, 67,
                    5, 167, 39, 0, 171, 13, 198, 43, 144, 117, 20, 112, 3, 248, 251, 68, 197, 76, 168,
                    116, 200, 43, 119, 58, 222, 243, 112, 199, 3, 134, 49, 71, 184, 111, 92, 200, 89,
                    4,
                ],
            ),
            (
                [
                    43, 199, 220, 200, 152, 163, 210, 104, 247, 237, 3, 10, 42, 146, 151, 211, 32, 128,
                    69, 115, 173, 153, 226, 245, 198, 70, 127, 50, 105, 103, 69, 5, 225, 143, 168, 217,
                    93, 12, 51, 233, 218, 140, 240, 72, 95, 27, 69, 243, 32, 194, 245, 194, 132, 60,
                    63, 203, 107, 244, 113, 109, 83, 157, 100, 21,
                ],
                [
                    83, 221, 254, 184, 55, 148, 227, 43, 133, 7, 18, 158, 114, 71, 125, 201, 138, 190,
                    192, 0, 56, 234, 29, 190, 13, 53, 55, 124, 65, 213, 82, 16, 190, 225, 85, 93, 216,
                    143, 253, 91, 162, 249, 28, 124, 77, 137, 187, 191, 41, 63, 204, 124, 190, 22, 134,
                    112, 142, 91, 162, 209, 153, 210, 182, 31, 36, 167, 184, 235, 213, 41, 254, 96, 37,
                    227, 187, 127, 87, 12, 115, 172, 212, 196, 214, 182, 240, 132, 194, 165, 181, 15,
                    200, 254, 250, 69, 45, 32, 97, 149, 114, 77, 166, 31, 30, 137, 84, 29, 211, 14,
                    204, 3, 70, 171, 70, 14, 213, 156, 243, 16, 201, 200, 211, 247, 42, 95, 196, 13,
                    58, 48,
                ],
            ),
        ],
        [
            (
                [
                    34, 122, 253, 204, 243, 16, 201, 133, 161, 151, 13, 130, 78, 126, 94, 163, 224, 32,
                    110, 105, 60, 173, 80, 225, 5, 251, 211, 85, 42, 227, 225, 17, 66, 75, 107, 118,
                    161, 223, 82, 148, 65, 172, 88, 173, 9, 109, 108, 229, 250, 87, 112, 159, 113, 219,
                    102, 31, 149, 48, 83, 81, 141, 139, 169, 17,
                ],
                [
                    133, 52, 151, 123, 19, 114, 157, 14, 21, 62, 189, 188, 4, 178, 35, 99, 225, 132,
                    32, 193, 205, 86, 200, 15, 25, 57, 244, 156, 6, 174, 131, 16, 112, 192, 162, 11,
                    208, 105, 38, 25, 207, 152, 137, 184, 141, 148, 183, 25, 137, 165, 117, 9, 241,
                    106, 140, 254, 1, 125, 113, 17, 96, 189, 169, 2, 253, 248, 3, 180, 29, 86, 110, 90,
                    49, 229, 224, 58, 22, 188, 76, 132, 220, 16, 176, 51, 132, 26, 126, 45, 224, 132,
                    17, 56, 248, 37, 12, 7, 23, 2, 42, 116, 42, 173, 235, 102, 244, 191, 177, 1, 93,
                    177, 63, 151, 44, 150, 232, 54, 181, 66, 207, 138, 144, 211, 104, 119, 163, 198, 6,
                    17,
                ],
            ),
            (
                [
                    220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89,
                    0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187,
                    136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133,
                    250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15,
                ],
                [
                    27, 204, 124, 11, 165, 70, 231, 141, 30, 176, 235, 127, 5, 147, 187, 136, 179, 176,
                    39, 54, 240, 245, 69, 79, 225, 2, 29, 28, 30, 92, 220, 14, 154, 121, 195, 133, 58,
                    138, 48, 178, 244, 161, 30, 12, 144, 147, 201, 94, 26, 26, 180, 238, 105, 53, 232,
                    123, 16, 26, 111, 42, 131, 150, 17, 32, 184, 189, 171, 1, 21, 45, 85, 39, 172, 64,
                    214, 75, 179, 42, 172, 248, 41, 111, 116, 204, 218, 37, 202, 100, 74, 134, 56, 35,
                    193, 179, 194, 47, 24, 25, 165, 85, 203, 222, 32, 43, 140, 89, 155, 150, 92, 130,
                    129, 161, 37, 230, 36, 249, 77, 180, 149, 50, 16, 212, 248, 81, 4, 241, 71, 46,
                ],
            ),
            (
                [
                    208, 81, 69, 193, 208, 184, 9, 149, 1, 84, 164, 160, 88, 157, 70, 224, 244, 253,
                    90, 181, 20, 25, 183, 146, 153, 228, 241, 189, 117, 142, 186, 30, 161, 103, 48, 84,
                    73, 70, 218, 115, 168, 176, 143, 92, 214, 13, 203, 2, 34, 146, 69, 99, 20, 32, 206,
                    167, 153, 85, 92, 14, 242, 134, 25, 5,
                ],
                [
                    83, 221, 254, 184, 55, 148, 227, 43, 133, 7, 18, 158, 114, 71, 125, 201, 138, 190,
                    192, 0, 56, 234, 29, 190, 13, 53, 55, 124, 65, 213, 82, 16, 190, 225, 85, 93, 216,
                    143, 253, 91, 162, 249, 28, 124, 77, 137, 187, 191, 41, 63, 204, 124, 190, 22, 134,
                    112, 142, 91, 162, 209, 153, 210, 182, 31, 36, 167, 184, 235, 213, 41, 254, 96, 37,
                    227, 187, 127, 87, 12, 115, 172, 212, 196, 214, 182, 240, 132, 194, 165, 181, 15,
                    200, 254, 250, 69, 45, 32, 97, 149, 114, 77, 166, 31, 30, 137, 84, 29, 211, 14,
                    204, 3, 70, 171, 70, 14, 213, 156, 243, 16, 201, 200, 211, 247, 42, 95, 196, 13,
                    58, 48,
                ],
            ),
        ],
    ];
}
