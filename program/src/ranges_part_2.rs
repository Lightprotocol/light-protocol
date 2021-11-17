
pub const f_cubic_0_range :[usize;2]= [0,288];
pub const f_cubic_1_range :[usize;2]= [288,576];
pub const solo_cubic_0_range :[usize;2]= [0,288];

pub const f_f2_range :[usize;2]= [0,576];  //f is not used anymore after inverse and replaced by f2
pub const f_f2_cubic_0_range :[usize;2]= [0,288];
pub const f_f2_cubic_1_range :[usize;2]= [288,576];
pub const f_f2_range_iter : usize = 0;

pub const f1_r_range :[usize;2]= [576,1152];
pub const f1_r_cubic_0_range :[usize;2]= [576, 576 + 288];
pub const f1_r_cubic_1_range :[usize;2]= [576 + 288, 1152];
pub const f1_r_range_iter : usize = 1;

pub const i_range :[usize;2]= [1152,1728];
pub const i_cubic_0_range :[usize;2]= [1152, 1152 + 288];
pub const i_cubic_1_range :[usize;2]= [1152 + 288, 1728];
pub const i_range_iter : usize = 2;

pub const y0_range :[usize;2]= [1728,2304];
pub const y0_cubic_0_range :[usize;2]= [1728, 1728 + 288];
pub const y0_cubic_1_range :[usize;2]= [1728 + 288, 2304];
pub const y0_range_iter : usize = 3;

pub const y1_range :[usize;2]= [2304,2880];
pub const y1_cubic_0_range :[usize;2]= [2304, 2304 + 288];
pub const y1_cubic_1_range :[usize;2]= [2304 + 288, 2880];
pub const y1_range_iter : usize = 4;

pub const y2_range :[usize;2]= [2880,3456];
pub const y2_cubic_0_range :[usize;2]= [2880, 2880 + 288];
pub const y2_cubic_1_range :[usize;2]= [2880 + 288, 3456];
pub const y2_range_iter : usize = 5;

pub const cubic_range_0: [usize;2] = [3456,3744];   // 288
pub const cubic_range_0_iter : usize = 6;
pub const cubic_range_1: [usize;2] = [3744,4032];   // 288
pub const cubic_range_1_iter : usize = 7;
pub const cubic_range_2: [usize;2] = [4465,4753];
pub const cubic_range_2_iter : usize = 8;

pub const quad_range_0: [usize;2]= [4032,4128];    // 96
pub const quad_range_0_iter : usize = 9;

pub const quad_range_1: [usize;2]= [4128,4224];    // 96
pub const quad_range_1_iter : usize = 10;

pub const quad_range_2: [usize;2]= [4224,4320];    // 96
pub const quad_range_2_iter : usize = 11;

pub const quad_range_3: [usize;2]= [4320,4416];    // 96
pub const quad_range_3_iter : usize = 12;

pub const fp384_range: [usize;2]= [4416,4464];   // 48
pub const fp384_range_iter : usize = 13;

pub const found_nonzero_range: [usize;2]= [4464,4465];
/*
pub const i2_range :[usize;2]= [4464,5040];
pub const i2_cubic_0_range :[usize;2]= [4464, 4464 + 288];
pub const i2_cubic_1_range :[usize;2]= [4464 + 288, 5040];
*/
pub const naf_vec: [i64; 65] = [1, 0, -1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];