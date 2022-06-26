use crate::groth16_verifier::miller_loop::state::*;
use crate::groth16_verifier::parsers::*;
use crate::groth16_verifier::VerifierState;
use crate::utils::prepared_verifying_key::*;
use ark_ec::bn::BnParameters;
use ark_ec::models::bn::g2::{addition_step, doubling_step, mul_by_char};
use ark_ff::{
    Field,
    Fp2ParamsWrapper,
    QuadExtField
};

use ark_std::One;
use ark_std::Zero;
use solana_program::msg;
use std::cell::RefMut;
use std::convert::TryInto;
use ark_bn254::{
    Parameters,
    Fq2Parameters
};

pub fn get_coeff(
    pair_index: u64,
    tmp_account: &mut RefMut<'_, VerifierState>,
    current_compute: &mut u64,
    tmp_account_compute: &mut MillerLoopStateCompute,
) -> Option<(
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
)> {
    match pair_index {
        0 => {
            //proof_b
            msg!("getting proof_b coeff");
            get_b_coeffs(current_compute, tmp_account, tmp_account_compute)
        }
        1 => {
            //gamma_g2_neg_pc
            msg!("getting gamma coeff");
            return Some(get_gamma_g2(tmp_account));
        }
        2 => {
            //delta_g2
            msg!("getting delta coeff");
            return Some(get_delta_g2(tmp_account));
        }
        _ => {
            panic!("Invalid index {}", pair_index);
        }
    }
}

pub fn get_b_coeffs(
    current_compute: &mut u64,
    tmp_account: &mut RefMut<'_, VerifierState>,
    tmp_account_compute: &mut MillerLoopStateCompute,
) -> Option<(
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
)> {
    msg!("getting b coeff");
    // if q.is_zero() {
    //     return Self {
    //         ell_coeffs: vec![],
    //         infinity: true,
    //     };
    // }
    let two_inv = <Parameters as BnParameters>::Fp::one()
        .double()
        .inverse()
        .unwrap();

    for i in (1..Parameters::ATE_LOOP_COUNT.len()
        - (tmp_account.outer_first_loop_coeff as usize))
        .rev()
    {
        if tmp_account.inner_first_coeff == 0 {
            *current_compute += 140_000;
            msg!("doubling_step");
            if *current_compute >= tmp_account.ml_max_compute {
                return None;
            } else {
                tmp_account.inner_first_coeff = 1;
                tmp_account.coeff_index[0] += 1;
                return Some(doubling_step::<Parameters>(
                    &mut tmp_account_compute.r,
                    &two_inv,
                ));
            }
        }

        let bit = Parameters::ATE_LOOP_COUNT[i - 1];

        match bit {
            1 => {
                *current_compute += 200_000;
                msg!("addition_step1");
                if *current_compute >= tmp_account.ml_max_compute {
                    return None;
                } else {
                    tmp_account.inner_first_coeff = 0;
                    tmp_account.outer_first_loop_coeff += 1;
                    tmp_account.coeff_index[0] += 1;
                    return Some(addition_step::<Parameters>(
                        &mut tmp_account_compute.r,
                        &parse_proof_b_from_bytes(&tmp_account.proof_b_bytes.to_vec()),
                    ));
                }
            }
            -1 => {
                *current_compute += 200_000;
                msg!("addition_step-1");
                if *current_compute >= tmp_account.ml_max_compute {
                    return None;
                } else {
                    tmp_account.inner_first_coeff = 0;
                    tmp_account.outer_first_loop_coeff += 1;
                    tmp_account.coeff_index[0] += 1;
                    return Some(addition_step::<Parameters>(
                        &mut tmp_account_compute.r,
                        &-parse_proof_b_from_bytes(&tmp_account.proof_b_bytes.to_vec()),
                    ));
                }
            }
            _ => {
                tmp_account.inner_first_coeff = 0;
                tmp_account.outer_first_loop_coeff += 1;
                continue;
            }
        }
    }
    // It is not negative.
    // if Parameters::X_IS_NEGATIVE {
    //     r.y = -r.y;
    // }

    if tmp_account.outer_second_coeff == 0 {
        *current_compute += 200_000;
        msg!("mul_by_char + addition_step");
        if *current_compute >= tmp_account.ml_max_compute {
            return None;
        }
        let q1 = mul_by_char::<Parameters>(parse_proof_b_from_bytes(
            &tmp_account.proof_b_bytes.to_vec(),
        ));

        let mut tmp = vec![0u8; 128];
        parse_proof_b_to_bytes(q1, &mut tmp);
        tmp_account.q1_bytes = tmp.try_into().unwrap();
        tmp_account.outer_second_coeff = 1;

        tmp_account.coeff_index[0] += 1;
        return Some(addition_step::<Parameters>(
            &mut tmp_account_compute.r,
            &q1,
        ));
    }
    *current_compute += 200_000;
    msg!("mul_by_char + addition_step");
    if *current_compute >= tmp_account.ml_max_compute {
        return None;
    }
    let mut q2 = mul_by_char::<Parameters>(parse_proof_b_from_bytes(
        &tmp_account.q1_bytes.to_vec(),
    ));
    q2.y = -q2.y;
    tmp_account.coeff_index[0] += 1;

    return Some(addition_step::<Parameters>(
        &mut tmp_account_compute.r,
        &q2,
    ));
}
#[allow(unused_assignments)]
pub fn get_gamma_g2(
    tmp_account: &mut VerifierState,
) -> (
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
) {
    let mut coeff: (
        QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
        QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
        QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
    ) = (
        QuadExtField::<Fp2ParamsWrapper<Fq2Parameters>>::zero(),
        QuadExtField::<Fp2ParamsWrapper<Fq2Parameters>>::zero(),
        QuadExtField::<Fp2ParamsWrapper<Fq2Parameters>>::zero(),
    );
    let id = tmp_account.coeff_index[1];
    msg!("Getting gamma coeff id: {}", id);
    // Reads from hardcoded verifying key.
    if id == 0 {
        coeff = get_gamma_g2_neg_pc_0();
    } else if id == 1 {
        coeff = get_gamma_g2_neg_pc_1();
    } else if id == 2 {
        coeff = get_gamma_g2_neg_pc_2();
    } else if id == 3 {
        coeff = get_gamma_g2_neg_pc_3();
    } else if id == 4 {
        coeff = get_gamma_g2_neg_pc_4();
    } else if id == 5 {
        coeff = get_gamma_g2_neg_pc_5();
    } else if id == 6 {
        coeff = get_gamma_g2_neg_pc_6();
    } else if id == 7 {
        coeff = get_gamma_g2_neg_pc_7();
    } else if id == 8 {
        coeff = get_gamma_g2_neg_pc_8();
    } else if id == 9 {
        coeff = get_gamma_g2_neg_pc_9();
    } else if id == 10 {
        coeff = get_gamma_g2_neg_pc_10();
    } else if id == 11 {
        coeff = get_gamma_g2_neg_pc_11();
    } else if id == 12 {
        coeff = get_gamma_g2_neg_pc_12();
    } else if id == 13 {
        coeff = get_gamma_g2_neg_pc_13();
    } else if id == 14 {
        coeff = get_gamma_g2_neg_pc_14();
    } else if id == 15 {
        coeff = get_gamma_g2_neg_pc_15();
    } else if id == 16 {
        coeff = get_gamma_g2_neg_pc_16();
    } else if id == 17 {
        coeff = get_gamma_g2_neg_pc_17();
    } else if id == 18 {
        coeff = get_gamma_g2_neg_pc_18();
    } else if id == 19 {
        coeff = get_gamma_g2_neg_pc_19();
    } else if id == 20 {
        coeff = get_gamma_g2_neg_pc_20();
    } else if id == 21 {
        coeff = get_gamma_g2_neg_pc_21();
    } else if id == 22 {
        coeff = get_gamma_g2_neg_pc_22();
    } else if id == 23 {
        coeff = get_gamma_g2_neg_pc_23();
    } else if id == 24 {
        coeff = get_gamma_g2_neg_pc_24();
    } else if id == 25 {
        coeff = get_gamma_g2_neg_pc_25();
    } else if id == 26 {
        coeff = get_gamma_g2_neg_pc_26();
    } else if id == 27 {
        coeff = get_gamma_g2_neg_pc_27();
    } else if id == 28 {
        coeff = get_gamma_g2_neg_pc_28();
    } else if id == 29 {
        coeff = get_gamma_g2_neg_pc_29();
    } else if id == 30 {
        coeff = get_gamma_g2_neg_pc_30();
    } else if id == 31 {
        coeff = get_gamma_g2_neg_pc_31();
    } else if id == 32 {
        coeff = get_gamma_g2_neg_pc_32();
    } else if id == 33 {
        coeff = get_gamma_g2_neg_pc_33();
    } else if id == 34 {
        coeff = get_gamma_g2_neg_pc_34();
    } else if id == 35 {
        coeff = get_gamma_g2_neg_pc_35();
    } else if id == 36 {
        coeff = get_gamma_g2_neg_pc_36();
    } else if id == 37 {
        coeff = get_gamma_g2_neg_pc_37();
    } else if id == 38 {
        coeff = get_gamma_g2_neg_pc_38();
    } else if id == 39 {
        coeff = get_gamma_g2_neg_pc_39();
    } else if id == 40 {
        coeff = get_gamma_g2_neg_pc_40();
    } else if id == 41 {
        coeff = get_gamma_g2_neg_pc_41();
    } else if id == 42 {
        coeff = get_gamma_g2_neg_pc_42();
    } else if id == 43 {
        coeff = get_gamma_g2_neg_pc_43();
    } else if id == 44 {
        coeff = get_gamma_g2_neg_pc_44();
    } else if id == 45 {
        coeff = get_gamma_g2_neg_pc_45();
    } else if id == 46 {
        coeff = get_gamma_g2_neg_pc_46();
    } else if id == 47 {
        coeff = get_gamma_g2_neg_pc_47();
    } else if id == 48 {
        coeff = get_gamma_g2_neg_pc_48();
    } else if id == 49 {
        coeff = get_gamma_g2_neg_pc_49();
    } else if id == 50 {
        coeff = get_gamma_g2_neg_pc_50();
    } else if id == 51 {
        coeff = get_gamma_g2_neg_pc_51();
    } else if id == 52 {
        coeff = get_gamma_g2_neg_pc_52();
    } else if id == 53 {
        coeff = get_gamma_g2_neg_pc_53();
    } else if id == 54 {
        coeff = get_gamma_g2_neg_pc_54();
    } else if id == 55 {
        coeff = get_gamma_g2_neg_pc_55();
    } else if id == 56 {
        coeff = get_gamma_g2_neg_pc_56();
    } else if id == 57 {
        coeff = get_gamma_g2_neg_pc_57();
    } else if id == 58 {
        coeff = get_gamma_g2_neg_pc_58();
    } else if id == 59 {
        coeff = get_gamma_g2_neg_pc_59();
    } else if id == 60 {
        coeff = get_gamma_g2_neg_pc_60();
    } else if id == 61 {
        coeff = get_gamma_g2_neg_pc_61();
    } else if id == 62 {
        coeff = get_gamma_g2_neg_pc_62();
    } else if id == 63 {
        coeff = get_gamma_g2_neg_pc_63();
    } else if id == 64 {
        coeff = get_gamma_g2_neg_pc_64();
    } else if id == 65 {
        coeff = get_gamma_g2_neg_pc_65();
    } else if id == 66 {
        coeff = get_gamma_g2_neg_pc_66();
    } else if id == 67 {
        coeff = get_gamma_g2_neg_pc_67();
    } else if id == 68 {
        coeff = get_gamma_g2_neg_pc_68();
    } else if id == 69 {
        coeff = get_gamma_g2_neg_pc_69();
    } else if id == 70 {
        coeff = get_gamma_g2_neg_pc_70();
    } else if id == 71 {
        coeff = get_gamma_g2_neg_pc_71();
    } else if id == 72 {
        coeff = get_gamma_g2_neg_pc_72();
    } else if id == 73 {
        coeff = get_gamma_g2_neg_pc_73();
    } else if id == 74 {
        coeff = get_gamma_g2_neg_pc_74();
    } else if id == 75 {
        coeff = get_gamma_g2_neg_pc_75();
    } else if id == 76 {
        coeff = get_gamma_g2_neg_pc_76();
    } else if id == 77 {
        coeff = get_gamma_g2_neg_pc_77();
    } else if id == 78 {
        coeff = get_gamma_g2_neg_pc_78();
    } else if id == 79 {
        coeff = get_gamma_g2_neg_pc_79();
    } else if id == 80 {
        coeff = get_gamma_g2_neg_pc_80();
    } else if id == 81 {
        coeff = get_gamma_g2_neg_pc_81();
    } else if id == 82 {
        coeff = get_gamma_g2_neg_pc_82();
    } else if id == 83 {
        coeff = get_gamma_g2_neg_pc_83();
    } else if id == 84 {
        coeff = get_gamma_g2_neg_pc_84();
    } else if id == 85 {
        coeff = get_gamma_g2_neg_pc_85();
    } else if id == 86 {
        coeff = get_gamma_g2_neg_pc_86();
    } else if id == 87 {
        coeff = get_gamma_g2_neg_pc_87();
    } else if id == 88 {
        coeff = get_gamma_g2_neg_pc_88();
    } else if id == 89 {
        coeff = get_gamma_g2_neg_pc_89();
    } else if id == 90 {
        coeff = get_gamma_g2_neg_pc_90();
    } else {
        msg!("ERR: coeff uninitialized value");
        panic!();
    }
    tmp_account.coeff_index[1] += 1;
    coeff
}
#[allow(unused_assignments)]
pub fn get_delta_g2(
    tmp_account: &mut VerifierState,
) -> (
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
    QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
) {
    let mut coeff: (
        QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
        QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
        QuadExtField<Fp2ParamsWrapper<Fq2Parameters>>,
    ) = (
        QuadExtField::<Fp2ParamsWrapper<Fq2Parameters>>::zero(),
        QuadExtField::<Fp2ParamsWrapper<Fq2Parameters>>::zero(),
        QuadExtField::<Fp2ParamsWrapper<Fq2Parameters>>::zero(),
    );
    let id = tmp_account.coeff_index[2];
    msg!("Getting delta coeff id: {}", id);

    // Reads from hardcoded verifying key.
    if id == 0 {
        coeff = get_delta_g2_neg_pc_0();
    } else if id == 1 {
        coeff = get_delta_g2_neg_pc_1();
    } else if id == 2 {
        coeff = get_delta_g2_neg_pc_2();
    } else if id == 3 {
        coeff = get_delta_g2_neg_pc_3();
    } else if id == 4 {
        coeff = get_delta_g2_neg_pc_4();
    } else if id == 5 {
        coeff = get_delta_g2_neg_pc_5();
    } else if id == 6 {
        coeff = get_delta_g2_neg_pc_6();
    } else if id == 7 {
        coeff = get_delta_g2_neg_pc_7();
    } else if id == 8 {
        coeff = get_delta_g2_neg_pc_8();
    } else if id == 9 {
        coeff = get_delta_g2_neg_pc_9();
    } else if id == 10 {
        coeff = get_delta_g2_neg_pc_10();
    } else if id == 11 {
        coeff = get_delta_g2_neg_pc_11();
    } else if id == 12 {
        coeff = get_delta_g2_neg_pc_12();
    } else if id == 13 {
        coeff = get_delta_g2_neg_pc_13();
    } else if id == 14 {
        coeff = get_delta_g2_neg_pc_14();
    } else if id == 15 {
        coeff = get_delta_g2_neg_pc_15();
    } else if id == 16 {
        coeff = get_delta_g2_neg_pc_16();
    } else if id == 17 {
        coeff = get_delta_g2_neg_pc_17();
    } else if id == 18 {
        coeff = get_delta_g2_neg_pc_18();
    } else if id == 19 {
        coeff = get_delta_g2_neg_pc_19();
    } else if id == 20 {
        coeff = get_delta_g2_neg_pc_20();
    } else if id == 21 {
        coeff = get_delta_g2_neg_pc_21();
    } else if id == 22 {
        coeff = get_delta_g2_neg_pc_22();
    } else if id == 23 {
        coeff = get_delta_g2_neg_pc_23();
    } else if id == 24 {
        coeff = get_delta_g2_neg_pc_24();
    } else if id == 25 {
        coeff = get_delta_g2_neg_pc_25();
    } else if id == 26 {
        coeff = get_delta_g2_neg_pc_26();
    } else if id == 27 {
        coeff = get_delta_g2_neg_pc_27();
    } else if id == 28 {
        coeff = get_delta_g2_neg_pc_28();
    } else if id == 29 {
        coeff = get_delta_g2_neg_pc_29();
    } else if id == 30 {
        coeff = get_delta_g2_neg_pc_30();
    } else if id == 31 {
        coeff = get_delta_g2_neg_pc_31();
    } else if id == 32 {
        coeff = get_delta_g2_neg_pc_32();
    } else if id == 33 {
        coeff = get_delta_g2_neg_pc_33();
    } else if id == 34 {
        coeff = get_delta_g2_neg_pc_34();
    } else if id == 35 {
        coeff = get_delta_g2_neg_pc_35();
    } else if id == 36 {
        coeff = get_delta_g2_neg_pc_36();
    } else if id == 37 {
        coeff = get_delta_g2_neg_pc_37();
    } else if id == 38 {
        coeff = get_delta_g2_neg_pc_38();
    } else if id == 39 {
        coeff = get_delta_g2_neg_pc_39();
    } else if id == 40 {
        coeff = get_delta_g2_neg_pc_40();
    } else if id == 41 {
        coeff = get_delta_g2_neg_pc_41();
    } else if id == 42 {
        coeff = get_delta_g2_neg_pc_42();
    } else if id == 43 {
        coeff = get_delta_g2_neg_pc_43();
    } else if id == 44 {
        coeff = get_delta_g2_neg_pc_44();
    } else if id == 45 {
        coeff = get_delta_g2_neg_pc_45();
    } else if id == 46 {
        coeff = get_delta_g2_neg_pc_46();
    } else if id == 47 {
        coeff = get_delta_g2_neg_pc_47();
    } else if id == 48 {
        coeff = get_delta_g2_neg_pc_48();
    } else if id == 49 {
        coeff = get_delta_g2_neg_pc_49();
    } else if id == 50 {
        coeff = get_delta_g2_neg_pc_50();
    } else if id == 51 {
        coeff = get_delta_g2_neg_pc_51();
    } else if id == 52 {
        coeff = get_delta_g2_neg_pc_52();
    } else if id == 53 {
        coeff = get_delta_g2_neg_pc_53();
    } else if id == 54 {
        coeff = get_delta_g2_neg_pc_54();
    } else if id == 55 {
        coeff = get_delta_g2_neg_pc_55();
    } else if id == 56 {
        coeff = get_delta_g2_neg_pc_56();
    } else if id == 57 {
        coeff = get_delta_g2_neg_pc_57();
    } else if id == 58 {
        coeff = get_delta_g2_neg_pc_58();
    } else if id == 59 {
        coeff = get_delta_g2_neg_pc_59();
    } else if id == 60 {
        coeff = get_delta_g2_neg_pc_60();
    } else if id == 61 {
        coeff = get_delta_g2_neg_pc_61();
    } else if id == 62 {
        coeff = get_delta_g2_neg_pc_62();
    } else if id == 63 {
        coeff = get_delta_g2_neg_pc_63();
    } else if id == 64 {
        coeff = get_delta_g2_neg_pc_64();
    } else if id == 65 {
        coeff = get_delta_g2_neg_pc_65();
    } else if id == 66 {
        coeff = get_delta_g2_neg_pc_66();
    } else if id == 67 {
        coeff = get_delta_g2_neg_pc_67();
    } else if id == 68 {
        coeff = get_delta_g2_neg_pc_68();
    } else if id == 69 {
        coeff = get_delta_g2_neg_pc_69();
    } else if id == 70 {
        coeff = get_delta_g2_neg_pc_70();
    } else if id == 71 {
        coeff = get_delta_g2_neg_pc_71();
    } else if id == 72 {
        coeff = get_delta_g2_neg_pc_72();
    } else if id == 73 {
        coeff = get_delta_g2_neg_pc_73();
    } else if id == 74 {
        coeff = get_delta_g2_neg_pc_74();
    } else if id == 75 {
        coeff = get_delta_g2_neg_pc_75();
    } else if id == 76 {
        coeff = get_delta_g2_neg_pc_76();
    } else if id == 77 {
        coeff = get_delta_g2_neg_pc_77();
    } else if id == 78 {
        coeff = get_delta_g2_neg_pc_78();
    } else if id == 79 {
        coeff = get_delta_g2_neg_pc_79();
    } else if id == 80 {
        coeff = get_delta_g2_neg_pc_80();
    } else if id == 81 {
        coeff = get_delta_g2_neg_pc_81();
    } else if id == 82 {
        coeff = get_delta_g2_neg_pc_82();
    } else if id == 83 {
        coeff = get_delta_g2_neg_pc_83();
    } else if id == 84 {
        coeff = get_delta_g2_neg_pc_84();
    } else if id == 85 {
        coeff = get_delta_g2_neg_pc_85();
    } else if id == 86 {
        coeff = get_delta_g2_neg_pc_86();
    } else if id == 87 {
        coeff = get_delta_g2_neg_pc_87();
    } else if id == 88 {
        coeff = get_delta_g2_neg_pc_88();
    } else if id == 89 {
        coeff = get_delta_g2_neg_pc_89();
    } else if id == 90 {
        coeff = get_delta_g2_neg_pc_90();
    } else {
        msg!("ERR: coeff uninitialized value");
        panic!();
    }
    tmp_account.coeff_index[2] += 1;
    coeff
}
