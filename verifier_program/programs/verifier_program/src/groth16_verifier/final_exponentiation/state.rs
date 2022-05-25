
use anchor_lang::solana_program::system_program;
use anchor_lang::prelude::*;


#[account(zero_copy)]
pub struct FinalExponentiationState {
    pub signing_address: Pubkey,
    pub f:  [u8;384],
    pub f1: [u8;384],
    pub f2: [u8;384],
    pub f3: [u8;384],
    pub f4: [u8;384],
    pub f5: [u8;384],
    pub i: [u8;384],
    pub current_instruction_index: u64,
    pub max_compute: u64,
    pub current_compute: u64,
    pub first_exp_by_neg_x: u64,
    pub second_exp_by_neg_x:u64,
    pub third_exp_by_neg_x: u64,
    pub initialized: u64,
    pub outer_loop: u64,
    pub cyclotomic_square_in_place:u64
}
impl FinalExponentiationState {
    pub fn new(f: [u8;384]) ->  FinalExponentiationState {
        FinalExponentiationState {
            signing_address: Pubkey::new(&[0;32]),
            f:  f,
            f1: [0;384],
            f2: [0;384],
            f3: [0;384],
            f4: [0;384],
            f5: [0;384],
            i: [0;384],
            current_instruction_index: 0,
            max_compute: 250_000,
            current_compute:0,
            first_exp_by_neg_x: 0,
            second_exp_by_neg_x:0,
            third_exp_by_neg_x: 0,
            initialized: 0,
            outer_loop: 1,
            cyclotomic_square_in_place:0,
        }
    }

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
