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

#[test]
fn test_if_equals_zero_u64() {
    assert_eq!(if_equals_zero_u64(0), None);
    assert_eq!(if_equals_zero_u64(1), Some(1));
}

#[test]
fn test_if_equals_none() {
    assert_eq!(if_equals_none(0, 0), None);
    assert_eq!(if_equals_none(1, 0), Some(1));
}
