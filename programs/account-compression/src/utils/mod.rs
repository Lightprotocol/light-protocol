pub mod check_account;
pub mod check_discrimininator;
pub mod check_signer_is_registered_or_authority;
pub mod constants;
pub mod queue;
pub mod transfer_lamports;

pub fn if_equals_zero_u64(value: u64) -> Option<u64> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

pub fn if_equals_none<T>(value: T, default: T) -> Option<T>
where
    T: PartialEq,
{
    if value == default {
        None
    } else {
        Some(value)
    }
}
