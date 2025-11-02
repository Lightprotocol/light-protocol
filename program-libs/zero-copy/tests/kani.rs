#![cfg(kani)]
// Kani formal verification tests for ZeroCopyCyclicVec and ZeroCopyVec
// cargo kani --tests --no-default-features -Z stubbing --features kani

use light_zero_copy::cyclic_vec::ZeroCopyCyclicVecU32;
use light_zero_copy::vec::ZeroCopyVecU32;

// without kani feature Verification Time: 214.86237s
// with kani feature Verification Time: 1.6097491s
/// Verify that push operations work correctly and maintain cyclic behavior
#[kani::proof]
#[kani::unwind(12)]
fn verify_cyclic_vec_push() {
    let mut buffer = [0u8; 512];
    let capacity: u32 = kani::any();

    // Bound capacity for faster verification
    kani::assume(capacity > 0 && capacity <= 5);

    let required_size = ZeroCopyCyclicVecU32::<u32>::required_size_for_capacity(capacity);
    kani::assume(buffer.len() >= required_size);

    let mut vec = ZeroCopyCyclicVecU32::<u32>::new(capacity, &mut buffer).unwrap();

    // Verify initial state
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());

    // Push elements up to twice the capacity to test cyclic behavior
    let push_count = capacity * 2;
    for i in 0..push_count {
        vec.push(i);

        // Length should grow until capacity, then stay at capacity
        let expected_len = ((i + 1) as usize).min(capacity as usize);
        assert_eq!(vec.len(), expected_len);

        // Length should never exceed capacity (cyclic property)
        assert!(vec.len() <= vec.capacity());
    }

    // After pushing 2*capacity elements, length should equal capacity
    assert_eq!(vec.len(), capacity as usize);
}

/// Verify that ZeroCopyVec push operations work correctly
#[kani::proof]
#[kani::unwind(12)]
fn verify_vec_push() {
    let mut buffer = [0u8; 512];
    let capacity: u32 = kani::any();

    // Bound capacity for faster verification
    kani::assume(capacity > 0 && capacity <= 5);

    let required_size = ZeroCopyVecU32::<u32>::required_size_for_capacity(capacity);
    kani::assume(buffer.len() >= required_size);

    let mut vec = ZeroCopyVecU32::<u32>::new(capacity, &mut buffer).unwrap();

    // Verify initial state
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
    assert_eq!(vec.capacity(), capacity as usize);

    // Push elements up to capacity
    for i in 0..capacity {
        assert!(vec.push(i).is_ok());
        assert_eq!(vec.len(), (i + 1) as usize);
        assert!(!vec.is_empty());

        // Verify the element was added correctly
        assert_eq!(vec.get(i as usize), Some(&i));
    }

    // Verify vector is full at capacity
    assert_eq!(vec.len(), capacity as usize);

    // Verify pushing beyond capacity fails
    assert!(vec.push(capacity).is_err());

    // Length should still be at capacity after failed push
    assert_eq!(vec.len(), capacity as usize);
}
