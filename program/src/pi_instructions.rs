use crate::pi_254_parsers::*;
use crate::utils::prepared_verifying_key::*;
use ark_ec::{AffineCurve, ProjectiveCurve};
use ark_ff::biginteger::BigInteger256;
use ark_ff::fields::BitIteratorBE;
use ark_ff::fields::{Field, PrimeField};
use ark_ff::{Fp256, Fp384};
use ark_relations::r1cs::SynthesisError; // currently commented out, should implement manual error.
use ark_std::Zero;
use solana_program::{log::sol_log_compute_units, msg, program_error::ProgramError};

// Initializes all i,x pairs. 7 pairs for 7 public inputs.
// Creates all i,x pairs once, then stores them in specified ranges.
// Other ix can then parse the i,x pair they need. This essentially replicates
// the loop behavior inside the library's implementation:
//  for (i, b) in public_inputs.iter().zip(pvk.vk.gamma_abc_g1.iter().skip(1)) {
//      g_ic.add_assign(&b.mul(i.into_repr()));
//  }
// @ark-groth/src/verifier.rs
pub fn init_pairs_instruction(
    public_inputs: &[ark_ff::Fp256<ark_ed_on_bn254::FqParameters>], // from bytes
    i_1_range: &mut Vec<u8>,
    x_1_range: &mut Vec<u8>,
    i_2_range: &mut Vec<u8>,
    x_2_range: &mut Vec<u8>,
    i_3_range: &mut Vec<u8>,
    x_3_range: &mut Vec<u8>,
    i_4_range: &mut Vec<u8>,
    x_4_range: &mut Vec<u8>,
    i_5_range: &mut Vec<u8>,
    x_5_range: &mut Vec<u8>,
    i_6_range: &mut Vec<u8>,
    x_6_range: &mut Vec<u8>,
    i_7_range: &mut Vec<u8>,
    x_7_range: &mut Vec<u8>,
    g_ic_x_range: &mut Vec<u8>,
    g_ic_y_range: &mut Vec<u8>,
    g_ic_z_range: &mut Vec<u8>,
) {
    // Parses vk_gamma_abc_g1 from hardcoded file.
    // Should have 8 items if 7 public inputs are passed in since [0] will be used to initialize g_ic.
    // Called once.
    // Inputs from bytes -- cost: 20k
    let mut pvk_vk_gamma_abc_g1 = vec![
        get_gamma_abc_g1_0(),
        get_gamma_abc_g1_1(),
        get_gamma_abc_g1_2(),
        get_gamma_abc_g1_3(),
        get_gamma_abc_g1_4(),
        get_gamma_abc_g1_5(),
        get_gamma_abc_g1_6(),
        get_gamma_abc_g1_7(),
    ];
    if (public_inputs.len() + 1) != pvk_vk_gamma_abc_g1.len() { // 693
         // TODO: add manual error throw.
         // Err(SynthesisError::MalformedVerifyingKey);
    }

    // inits g_ic into range.
    let g_ic = pvk_vk_gamma_abc_g1[0].into_projective(); // 80
                                                         // println!("initial g_ic me: {:?}", &g_ic);
    parse_group_projective_to_bytes_254(g_ic, g_ic_x_range, g_ic_y_range, g_ic_z_range); // 10k

    // Creates and parses i,x pairs into ranges.
    let mut i_vec: Vec<ark_ff::Fp256<ark_ed_on_bn254::FqParameters>> = vec![];
    let mut x_vec: Vec<ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>> =
        vec![];

    for (i, x) in public_inputs.iter().zip(pvk_vk_gamma_abc_g1.iter().skip(1)) {
        i_vec.push(*i);
        x_vec.push(*x);
    }
    // println!("i me: {:?}", i_vec[0]);
    // println!("x me: {:?}", x_vec[0]);
    // println!("i me: {:?}", i_vec[1]);
    // println!("x me: {:?}", x_vec[1]);
    // println!("i me: {:?}", i_vec[2]);
    // println!("x me: {:?}", x_vec[2]);
    // println!("i me: {:?}", i_vec[3]);
    // println!("x me: {:?}", x_vec[3]);
    // println!("i me: {:?}", i_vec[4]);
    // println!("x me: {:?}", x_vec[4]);
    // println!("i me: {:?}", i_vec[5]);
    // println!("x me: {:?}", x_vec[5]);
    // println!("i me: {:?}", i_vec[6]);
    // println!("x me: {:?}", x_vec[6]);

    parse_fp256_to_bytes_254(i_vec[0], i_1_range, [0, 32]); // 3k
    parse_fp256_to_bytes_254(i_vec[1], i_2_range, [0, 32]); // 3k
    parse_fp256_to_bytes_254(i_vec[2], i_3_range, [0, 32]); // 3k
    parse_fp256_to_bytes_254(i_vec[3], i_4_range, [0, 32]); // 3k
    parse_fp256_to_bytes_254(i_vec[4], i_5_range, [0, 32]); // 3k
    parse_fp256_to_bytes_254(i_vec[5], i_6_range, [0, 32]); // 3k
    parse_fp256_to_bytes_254(i_vec[6], i_7_range, [0, 32]); // 3k

    parse_x_group_affine_to_bytes_254(x_vec[0], x_1_range); // 96bytes 10kr, 3kwr => 6k
    parse_x_group_affine_to_bytes_254(x_vec[1], x_2_range); // 6k
    parse_x_group_affine_to_bytes_254(x_vec[2], x_3_range); // 6k
    parse_x_group_affine_to_bytes_254(x_vec[3], x_4_range); // 6k

    parse_x_group_affine_to_bytes_254(x_vec[4], x_5_range); // 6k
    parse_x_group_affine_to_bytes_254(x_vec[5], x_6_range); // 6k
    parse_x_group_affine_to_bytes_254(x_vec[6], x_7_range); // 6k

    // check proper parsing
    // let a = parse_fp256_from_bytes_254(i_1_range, [0, 32]);
    // assert_eq!(a, i_vec[0]);
    // let b = parse_x_group_affine_from_bytes_254(x_1_range);
    // assert_eq!(b, x_vec[0]);
    // assert_eq!(b, pvk_vk_gamma_abc_g1[1]); // groupAffine vs G1Affine
}

// Initializes fresh res range. Called once for each bit at the beginning of each loop (256x).
pub fn init_res_instruction(
    res_x_range: &mut Vec<u8>,
    res_y_range: &mut Vec<u8>,
    res_z_range: &mut Vec<u8>,
) {
    let res: ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bn254::g1::Parameters> =
        ark_ec::short_weierstrass_jacobian::GroupProjective::zero(); // 88

    parse_group_projective_to_bytes_254(res, res_x_range, res_y_range, res_z_range);
    //Cost: 10k
}

// Computes new res values. Gets the current i,x pair.
// The current i,x pair is already chosen by the processor based on ix_id.
// Called 256 times for each i,x pair - so 256*7x.
// Current_index (0..256) is parsed in because we need to
// replicate the stripping of leading zeroes (which are random because based on the public inputs).
// TODO: Can we put the current_index entirely on-chain? Then we could send all ix in one block.
// TODO: Finish documentation
pub fn maths_instruction(
    res_x_range: &mut Vec<u8>,
    res_y_range: &mut Vec<u8>,
    res_z_range: &mut Vec<u8>,
    i_range: &Vec<u8>,
    x_range: &Vec<u8>,
    current_index: usize,
) {
    // Parses res,x,i from range.
    let mut res = parse_group_projective_from_bytes_254(res_x_range, res_y_range, res_z_range); //15k
    let x = parse_x_group_affine_from_bytes_254(x_range); // 10k
    let i = parse_fp256_from_bytes_254(i_range, [0, 32]); // 5k

    // create bit: (current i,x * current index).
    // First constructs all bits of current i,x pair.
    // Must skip leading zeroes. those are random based on the inputs (i).
    let a = i.into_repr(); // 1037
    let bits: ark_ff::BitIteratorBE<ark_ff::BigInteger256> = BitIteratorBE::new(a.into()); // 58
    let bits_without_leading_zeroes: Vec<bool> = bits.skip_while(|b| !b).collect();
    // println!("bits w/o 0s: {:?}", bits_without_leading_zeroes);
    let skipped = 256 - bits_without_leading_zeroes.len();

    // testing multi call: merging 2 ix into one:
    let mut index_in = current_index;
    // msg!("current index start: {}", index_in);

    for m in 0..4 {
        // println!("current index: {}", current_index);
        // If i.e. two leading zeroes exists (skipped == 2), 2 ix will be skipped (0,1).
        // msg!("current index (index in): {}, m: {}", index_in, m);
        if index_in < skipped {
            // msg!("skipping leading zero instruction...");
            // parse_group_projective_to_bytes_254(res, res_x_range, res_y_range, res_z_range);
            // Only needed for if m==0 goes into else, which doesnt store the res value, then goes into if at m==1
            if m == 3 {
                parse_group_projective_to_bytes_254(res, res_x_range, res_y_range, res_z_range);
            }

            // return;
        } else {
            // sol_log_compute_units();
            // msg!("current index: {:?}", current_index);
            // msg!("skipped: {:?}", skipped);
            // Get the current bit but account for removed zeroes.
            let current_bit = bits_without_leading_zeroes[index_in - skipped];
            // Info: when refering to the library's implementation keep in mind that here:
            // res == self
            // x == other
            res.double_in_place(); // 252 // 28145 // 28469 // 28411 // 28522 // 28306
                                   // sol_log_compute_units();

            if current_bit {
                // res.add_assign_mixed(&x) ==> same as >
                if x.is_zero() {
                    // cost: 0
                    // msg!("if if");
                } else if res.is_zero() {
                    // cost: 162
                    // msg!("if if else if");
                    // This is always the same value for the same curve.
                    // Hardcoded here since we can't pass in the P: of the library's implementation.
                    // TODO: Explain why.
                    let p_basefield_one: Fp256<ark_bn254::FqParameters> =
                        Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
                            15230403791020821917,
                            754611498739239741,
                            7381016538464732716,
                            1011752739694698287,
                        ])); // Cost: 31
                    res.x = x.x;
                    res.y = x.y;
                    res.z = p_basefield_one; // Replaces: &P:BASEFIELD::ONE();
                } else {
                    // msg!("if if else if else");
                    // Z1Z1 = Z1^2
                    let z1z1 = res.z.square();
                    // U2 = X2*Z1Z1
                    let u2 = x.x * &z1z1;
                    // S2 = Y2*Z1*Z1Z1
                    let s2 = (x.y * &res.z) * &z1z1;
                    // sol_log_compute_units(); // cost: 16709

                    if res.x == u2 && res.y == s2 {
                        // cost: 30k
                        // msg!("if if else if else if");

                        // The two points are equal, so we double.
                        res.double_in_place();
                        // sol_log_compute_units();
                    } else {
                        // cost: 29894

                        // If we're adding -a and a together, self.z becomes zero as H becomes zero.
                        // msg!("if if else if else if else");
                        // H = U2-X1
                        let h = u2 - &res.x;
                        // HH = H^2
                        let hh = h.square();
                        // I = 4*HH
                        let mut i = hh;
                        i.double_in_place().double_in_place();
                        // J = H*I
                        let mut j = h * &i;
                        // r = 2*(S2-Y1)
                        let r = (s2 - &res.y).double();
                        // V = X1*I
                        let v = res.x * &i;
                        // X3 = r^2 - J - 2*V
                        res.x = r.square();
                        res.x -= &j;
                        res.x -= &v;
                        res.x -= &v;
                        // Y3 = r*(V-X3)-2*Y1*J
                        j *= &res.y; // J = 2*Y1*J
                        j.double_in_place();
                        res.y = v - &res.x;
                        res.y *= &r;
                        res.y -= &j;
                        // Z3 = (Z1+H)^2-Z1Z1-HH
                        res.z += &h;
                        res.z.square_in_place();
                        res.z -= &z1z1;
                        res.z -= &hh;
                    }
                }
            }
            // sol_log_compute_units();
            // if m == max
            if m == 3 {
                parse_group_projective_to_bytes_254(res, res_x_range, res_y_range, res_z_range);
            }
        }
        index_in += 1;
    }
}

//3
pub fn maths_g_ic_instruction(
    g_ic_x_range: &mut Vec<u8>,
    g_ic_y_range: &mut Vec<u8>,
    g_ic_z_range: &mut Vec<u8>,
    res_x_range: &Vec<u8>,
    res_y_range: &Vec<u8>,
    res_z_range: &Vec<u8>,
) {
    // parse g_ic
    let mut g_ic = parse_group_projective_from_bytes_254(g_ic_x_range, g_ic_y_range, g_ic_z_range); // 15k
    let res = parse_group_projective_from_bytes_254(res_x_range, res_y_range, res_z_range); // 15k

    if g_ic.is_zero() {
        g_ic = res;
    } else if res.is_zero() {
    } else {
        // http://www.hyperelliptic.org/EFD/g1p/auto-shortw-jacobian-0.html#addition-add-2007-bl
        // Works for all curves.

        // Z1Z1 = Z1^2
        let z1z1 = g_ic.z.square();

        // Z2Z2 = Z2^2
        let z2z2 = res.z.square();

        // U1 = X1*Z2Z2
        let u1 = g_ic.x * &z2z2;

        // U2 = X2*Z1Z1
        let u2 = res.x * &z1z1;

        // S1 = Y1*Z2*Z2Z2
        let s1 = g_ic.y * &res.z * &z2z2;

        // S2 = Y2*Z1*Z1Z1
        let s2 = res.y * &g_ic.z * &z1z1;

        if u1 == u2 && s1 == s2 {
            // The two points are equal, so we double.
            g_ic.double_in_place();
        } else {
            // If we're adding -a and a together, self.z becomes zero as H becomes zero.

            // H = U2-U1
            let h = u2 - &u1;

            // I = (2*H)^2
            let i = (h.double()).square();

            // J = H*I
            let j = h * &i;

            // r = 2*(S2-S1)
            let r = (s2 - &s1).double();

            // V = U1*I
            let v = u1 * &i;

            // X3 = r^2 - J - 2*V
            g_ic.x = r.square() - &j - &(v.double());

            // Y3 = r*(V - X3) - 2*S1*J
            g_ic.y = r * &(v - &g_ic.x) - &*(s1 * &j).double_in_place();

            // Z3 = ((Z1+Z2)^2 - Z1Z1 - Z2Z2)*H
            g_ic.z = ((g_ic.z + &res.z).square() - &z1z1 - &z2z2) * &h;
        }
    }
    // res will be created anew with new loop, + new i,x will be used with index
    // cost: 15k
    parse_group_projective_to_bytes_254(g_ic, g_ic_x_range, g_ic_y_range, g_ic_z_range)
}

// There are two ix in total to turn the g_ic from projective into affine.
// In the end it's stored in the x_1_range (overwrite).
// The verifier then reads the x_1_range to use the g_ic value as P2 for the millerloop.
// Split up into two ix because of compute budget limits.
pub fn g_ic_into_affine_1(
    g_ic_x_range: &mut Vec<u8>,
    g_ic_y_range: &mut Vec<u8>,
    g_ic_z_range: &mut Vec<u8>,
) {
    let g_ic: ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bn254::g1::Parameters> =
        parse_group_projective_from_bytes_254(&g_ic_x_range, &g_ic_y_range, &g_ic_z_range); // 15k
    println!("g_ic_projective1: {:?}", &g_ic);

    let zinv = ark_ff::Field::inverse(&g_ic.z).unwrap();
    let g_ic_with_zinv: ark_ec::short_weierstrass_jacobian::GroupProjective<
        ark_bn254::g1::Parameters,
    > = ark_ec::short_weierstrass_jacobian::GroupProjective::new(g_ic.x, g_ic.y, zinv);
    parse_group_projective_to_bytes_254(g_ic_with_zinv, g_ic_x_range, g_ic_y_range, g_ic_z_range);
}

pub fn g_ic_into_affine_2(
    g_ic_x_range: &Vec<u8>,
    g_ic_y_range: &Vec<u8>,
    g_ic_z_range: &Vec<u8>,
    x_1_range: &mut Vec<u8>,
) {
    let g_ic: ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bn254::g1::Parameters> =
        parse_group_projective_from_bytes_254(&g_ic_x_range, &g_ic_y_range, &g_ic_z_range); // 15k
    println!("g_ic_projective with zinv: {:?}", &g_ic);

    let zinv_squared = ark_ff::Field::square(&g_ic.z);
    let x = g_ic.x * &zinv_squared;
    let y = g_ic.y * &(zinv_squared * &g_ic.z);

    let g_ic_affine: ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters> =
        ark_ec::short_weierstrass_jacobian::GroupAffine::new(x, y, false);

    println!("g_ic_affine: {:?}", g_ic_affine);
    parse_x_group_affine_to_bytes_254(g_ic_affine, x_1_range); // overwrite x1range w: 5066
}
