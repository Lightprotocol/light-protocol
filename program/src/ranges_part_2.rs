
pub const f_cubic_0_range :[usize;2]= [0,192];
pub const f_cubic_1_range :[usize;2]= [192,384];
pub const solo_cubic_0_range :[usize;2]= [0,192];
pub const f_f2_range_iter : usize = 0;
pub const f1_r_range_iter : usize = 1;
pub const i_range_iter : usize = 2;
pub const y0_range_iter : usize = 3;
pub const y1_range_iter : usize = 4;
pub const y2_range_iter : usize = 5;
pub const cubic_range_0_iter : usize = 6;
pub const cubic_range_1_iter : usize = 7;
pub const cubic_range_2_iter : usize = 8;
pub const quad_range_0_iter : usize = 9;
pub const quad_range_1_iter : usize = 10;
pub const quad_range_2_iter : usize = 11;
pub const quad_range_3_iter : usize = 12;
pub const fp384_range_iter : usize = 13;


//bls12 pub const naf_vec: [i64; 65] = [1, 0, -1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
//bn254 and already reversed
pub const naf_vec: [i64; 63] = [1, 0, 0, 0, 1, 0, 1, 0, 0, -1, 0, 1, 0, 1, 0, -1, 0, 0, 1, 0, 1, 0, -1, 0, -1, 0, -1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, -1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, -1, 0, 0, 0, 1];
