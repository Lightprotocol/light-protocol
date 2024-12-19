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
