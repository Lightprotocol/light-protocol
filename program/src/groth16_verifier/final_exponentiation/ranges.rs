pub const F_CUBIC_0_RANGE: [usize; 2] = [0, 192];
pub const F_CUBIC_1_RANGE: [usize; 2] = [192, 384];
pub const SOLO_CUBIC_0_RANGE: [usize; 2] = [0, 192];
pub const F_F2_RANGE_ITER: usize = 0;
pub const F2_R_RANGE_ITER: usize = 1;
pub const I_RANGE_ITER: usize = 2;
pub const Y0_RANGE_ITER: usize = 3;
pub const Y1_RANGE_ITER: usize = 4;
pub const Y2_RANGE_ITER: usize = 5;
pub const CUBIC_RANGE_0_ITER: usize = 6;
pub const CUBIC_RANGE_1_ITER: usize = 7;
pub const CUBIC_RANGE_2_ITER: usize = 8;
pub const QUAD_RANGE_0_ITER: usize = 9;
pub const QUAD_RANGE_1_ITER: usize = 10;
pub const QUAD_RANGE_2_ITER: usize = 11;
pub const QUAD_RANGE_3_ITER: usize = 12;
pub const FP384_RANGE_ITER: usize = 13;
pub const FOUND_NULLIFIER_ITER: usize = 14;
pub const Y6_RANGE_ITER: usize = 15;

//bls12 pub const NAF_VEC: [i64; 65] = [1, 0, -1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
//bn254 and already reversed
pub const NAF_VEC: [i64; 63] = [
    1, 0, 0, 0, 1, 0, 1, 0, 0, -1, 0, 1, 0, 1, 0, -1, 0, 0, 1, 0, 1, 0, -1, 0, -1, 0, -1, 0, 1, 0,
    0, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, -1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, -1, 0, 0,
    0, 1,
];
pub const FINAL_EXPONENTIATION_START_INDEX: usize = 895;
pub const FINAL_EXPONENTIATION_END_INDEX: usize = 1266;
