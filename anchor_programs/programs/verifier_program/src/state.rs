use anchor_lang::prelude::*;



#[account(zero_copy)]
pub struct VerifierState {
    pub current_instruction_index: u64,
    pub signing_address: Pubkey, // is relayer address
    pub merkle_tree_tmp_account: Pubkey,
    pub relayer_fee: u64,
    pub recipient: Pubkey,
    pub amount: [u8;32],
    pub nullifier_hash: [u8;32],
    pub root_hash: [u8;32],
    pub tx_integrity_hash: [u8;32], // is calculated on-chain from recipient, amount, signing_address,
    pub proof_a_bytes:     [u8;64], //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
    pub proof_b_bytes:     [u8;128],//ark_ec::models::bn::g2::G2Affine<ark_bn254::Parameters>,
    pub proof_c_bytes:     [u8;64], //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
    pub ext_amount: [u8;8],
    pub fee: [u8;8],
    pub leaf_left: [u8;32],
    pub leaf_right: [u8;32],
    pub nullifier0: [u8; 32],
    pub nullifier1: [u8;32],

    pub i_1_range: [u8;32],
    pub x_1_range: [u8;64],
    pub i_2_range: [u8;32],
    pub x_2_range: [u8;64],
    pub i_3_range: [u8;32],
    pub x_3_range: [u8;64],
    pub i_4_range: [u8;32],
    pub x_4_range: [u8;64],
    pub i_5_range: [u8;32],
    pub x_5_range: [u8;64],
    pub i_6_range: [u8;32],
    pub x_6_range: [u8;64],
    pub i_7_range: [u8;32],
    pub x_7_range: [u8;64],

    pub res_x_range: [u8;32],
    pub res_y_range: [u8;32],
    pub res_z_range: [u8;32],

    pub g_ic_x_range:       [u8;32],
    pub g_ic_y_range:       [u8;32],
    pub g_ic_z_range:       [u8;32],
    pub current_index: u64,

    // miller loop
    pub r_bytes:            [u8;192],//ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters>,//[u8;192],
    pub q1_bytes:           [u8;128],
    pub current_coeff_bytes:[u8;192],



    pub outer_first_loop_coeff:    u64,
    pub outer_second_coeff:        u64,
    pub inner_first_coeff:         u64,


    pub f_bytes:  [u8;384], // results miller_loop

    pub compute_max_miller_loop:           u64,
    pub outer_first_loop:          u64,
    pub outer_second_loop:         u64,
    pub outer_third_loop:          u64,
    pub first_inner_loop_index:    u64,
    pub second_inner_loop_index:   u64,
    pub square_in_place_executed:  u64,

    // final_exponentiation
    pub fe_instruction_index: u64,
    pub f_bytes1: [u8;384],
    pub f_bytes2: [u8;384],
    pub f_bytes3: [u8;384],
    pub f_bytes4: [u8;384],
    pub f_bytes5: [u8;384],
    pub i_bytes: [u8;384],
    pub max_compute: u64,
    pub current_compute: u64,
    pub first_exp_by_neg_x: u64,
    pub second_exp_by_neg_x:u64,
    pub third_exp_by_neg_x: u64,
    pub initialized: u64,
    pub outer_loop: u64,
    pub cyclotomic_square_in_place:u64,
    pub merkle_tree_instruction_index: u64,
    pub current_instruction_index_prepare_inputs: u64,


    pub encrypted_utxos: [u8;222],
    pub coeff_index:               [u8;3],

    pub last_transaction: bool,
    pub computing_prepared_inputs: bool, // 0 prepare inputs // 1 miller loop //
    pub computing_miller_loop: bool,
    pub computing_final_exponentiation: bool,
    pub updating_merkle_tree: bool,
    pub merkle_tree_index: u8,
    pub found_root: u8,
}

impl VerifierState {

    pub fn check_compute_units(&self)-> bool {
        if self.current_compute < self.max_compute {
            msg!("check_compute_units: {}", true);
            true
        } else {
            msg!("check_compute_units: {}", false);
            false
        }

    }
}
