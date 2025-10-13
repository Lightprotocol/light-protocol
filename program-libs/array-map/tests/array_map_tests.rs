use light_array_map::{ArrayMap, ArrayMapError};

// Test error type for testing
#[derive(Debug, PartialEq)]
enum TestError {
    ArrayMap(ArrayMapError),
    Custom,
}

impl From<ArrayMapError> for TestError {
    fn from(e: ArrayMapError) -> Self {
        TestError::ArrayMap(e)
    }
}

#[test]
fn test_new_map() {
    let map = ArrayMap::<u32, String, 10>::new();
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
    assert!(map.last_accessed_index().is_none());
}

#[test]
fn test_insert() {
    let mut map = ArrayMap::<u32, String, 10>::new();

    let idx = map.insert(1, "one".to_string(), TestError::Custom).unwrap();

    assert_eq!(idx, 0);
    assert_eq!(map.len(), 1);
    assert_eq!(map.last_accessed_index(), Some(0));
    assert_eq!(map.get(0).unwrap().1, "one");
}

#[test]
fn test_get_by_key() {
    let mut map = ArrayMap::<u32, String, 10>::new();

    map.insert(1, "one".to_string(), TestError::Custom).unwrap();
    map.insert(2, "two".to_string(), TestError::Custom).unwrap();

    assert_eq!(map.get_by_key(&1), Some(&"one".to_string()));
    assert_eq!(map.get_by_key(&2), Some(&"two".to_string()));
    assert_eq!(map.get_by_key(&3), None);
}

#[test]
fn test_get_mut_by_key() {
    let mut map = ArrayMap::<u32, String, 10>::new();

    map.insert(1, "one".to_string(), TestError::Custom).unwrap();

    if let Some(val) = map.get_mut_by_key(&1) {
        *val = "ONE".to_string();
    }

    assert_eq!(map.get_by_key(&1), Some(&"ONE".to_string()));
}

#[test]
fn test_find_index() {
    let mut map = ArrayMap::<u32, String, 10>::new();

    map.insert(10, "ten".to_string(), TestError::Custom)
        .unwrap();
    map.insert(20, "twenty".to_string(), TestError::Custom)
        .unwrap();

    assert_eq!(map.find_index(&10), Some(0));
    assert_eq!(map.find_index(&20), Some(1));
    assert_eq!(map.find_index(&30), None);
}

#[test]
fn test_set_last_accessed_index() {
    let mut map = ArrayMap::<u32, String, 10>::new();

    map.insert(1, "one".to_string(), TestError::Custom).unwrap();
    map.insert(2, "two".to_string(), TestError::Custom).unwrap();

    // Should be at index 1 after last insert
    assert_eq!(map.last_accessed_index(), Some(1));

    // Set to 0
    map.set_last_accessed_index::<TestError>(0).unwrap();
    assert_eq!(map.last_accessed_index(), Some(0));

    // Out of bounds should fail
    let result = map.set_last_accessed_index::<TestError>(10);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        TestError::ArrayMap(ArrayMapError::IndexOutOfBounds)
    );
}

#[test]
fn test_capacity_limit() {
    let mut map = ArrayMap::<u32, String, 5>::new();

    // Fill to capacity
    for i in 0..5 {
        map.insert(i, format!("val{}", i), TestError::Custom)
            .unwrap();
    }

    assert_eq!(map.len(), 5);

    // 6th entry should fail
    let result = map.insert(5, "val5".to_string(), TestError::Custom);
    assert!(result.is_err());
}

#[test]
fn test_get_mut_direct() {
    let mut map = ArrayMap::<u32, u64, 10>::new();

    map.insert(1, 100, TestError::Custom).unwrap();

    if let Some(entry) = map.get_mut(0) {
        entry.1 += 50;
    }

    assert_eq!(map.get(0).unwrap().1, 150);
}

#[test]
fn test_last_accessed_index_updates() {
    let mut map = ArrayMap::<u32, u64, 10>::new();

    // Insert first entry
    map.insert(1, 100, TestError::Custom).unwrap();
    assert_eq!(map.last_accessed_index(), Some(0));

    // Insert second entry
    map.insert(2, 200, TestError::Custom).unwrap();
    assert_eq!(map.last_accessed_index(), Some(1));
}

#[cfg(feature = "alloc")]
#[test]
fn test_with_alloc_feature() {
    extern crate alloc;
    use alloc::{string::String, vec::Vec};

    // NOTE: ArrayVec is ALWAYS fixed-capacity (stack-only), even with alloc feature.
    // The alloc feature just enables using heap-allocated VALUE types like String/Vec.
    // ArrayVec itself will still error when capacity is exceeded.

    let mut map = ArrayMap::<u32, String, 5>::new();

    // Fill to capacity with heap-allocated strings
    for i in 0..5 {
        map.insert(i, format!("string_{}", i), TestError::Custom)
            .unwrap();
    }

    assert_eq!(map.len(), 5);

    // ArrayVec still has fixed capacity - 6th insert should fail
    let result = map.insert(5, String::from("overflow"), TestError::Custom);
    assert!(
        result.is_err(),
        "ArrayVec should fail when capacity exceeded, even with alloc feature"
    );

    // Test with Vec values (heap-allocated)
    let mut vec_map = ArrayMap::<u32, Vec<u32>, 3>::new();
    vec_map.insert(1, vec![1, 2, 3], TestError::Custom).unwrap();
    vec_map
        .insert(2, vec![4, 5, 6, 7, 8], TestError::Custom)
        .unwrap();
    vec_map.insert(3, vec![9, 10], TestError::Custom).unwrap();

    // The Vec VALUES can be any size (heap-allocated)
    assert_eq!(vec_map.get_by_key(&1).map(|v| v.len()), Some(3));
    assert_eq!(vec_map.get_by_key(&2).map(|v| v.len()), Some(5));

    // But the ArrayVec container itself still has fixed capacity
    let result = vec_map.insert(4, vec![99], TestError::Custom);
    assert!(
        result.is_err(),
        "ArrayVec container is still fixed capacity"
    );
}

#[test]
fn test_capacity_overflow_without_alloc() {
    // Demonstrate that ArrayVec has fixed capacity regardless of alloc feature
    let mut map = ArrayMap::<u32, u64, 3>::new();

    // Fill to capacity
    map.insert(1, 100, TestError::Custom).unwrap();
    map.insert(2, 200, TestError::Custom).unwrap();
    map.insert(3, 300, TestError::Custom).unwrap();

    assert_eq!(map.len(), 3);

    // 4th insert should fail - fixed capacity
    let result = map.insert(4, 400, TestError::Custom);
    assert!(result.is_err(), "ArrayVec has fixed capacity");
}

#[test]
fn test_get_u8() {
    let mut map = ArrayMap::<u32, String, 10>::new();

    map.insert(1, "one".to_string(), TestError::Custom).unwrap();
    map.insert(2, "two".to_string(), TestError::Custom).unwrap();
    map.insert(3, "three".to_string(), TestError::Custom)
        .unwrap();

    // Test valid indices
    assert_eq!(map.get_u8(0).unwrap().1, "one");
    assert_eq!(map.get_u8(1).unwrap().1, "two");
    assert_eq!(map.get_u8(2).unwrap().1, "three");

    // Test out of bounds
    assert!(map.get_u8(3).is_none());
    assert!(map.get_u8(255).is_none());
}

#[test]
fn test_get_mut_u8() {
    let mut map = ArrayMap::<u32, u64, 10>::new();

    map.insert(1, 100, TestError::Custom).unwrap();
    map.insert(2, 200, TestError::Custom).unwrap();
    map.insert(3, 300, TestError::Custom).unwrap();

    // Modify via get_mut_u8
    if let Some(entry) = map.get_mut_u8(1) {
        entry.1 += 50;
    }

    // Verify modification
    assert_eq!(map.get_u8(1).unwrap().1, 250);
    assert_eq!(map.get_u8(0).unwrap().1, 100);
    assert_eq!(map.get_u8(2).unwrap().1, 300);

    // Test out of bounds
    assert!(map.get_mut_u8(3).is_none());
    assert!(map.get_mut_u8(255).is_none());
}
