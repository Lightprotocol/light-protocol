use light_hash_set::{zero_copy::HashSetZeroCopy, HashSet, HashSetError};
use num_bigint::BigUint;

const BUCKET_OFFSET: usize = 24;
const BUCKET_SIZE: usize = 48;

fn expected_bucket(tag: usize, sequence_number: usize, value: [u8; 32]) -> [u8; BUCKET_SIZE] {
    let mut bytes = [0; BUCKET_SIZE];
    bytes[..8].copy_from_slice(&tag.to_ne_bytes());
    bytes[8..16].copy_from_slice(&sequence_number.to_ne_bytes());
    bytes[16..].copy_from_slice(&value);
    bytes
}

fn value_bytes(value: u8) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[31] = value;
    bytes
}

fn run_bucket_transitions(prefill: u8) -> Vec<[u8; BUCKET_SIZE]> {
    const CAPACITY: usize = 1;
    const SEQUENCE_THRESHOLD: usize = 10;

    let mut bytes = vec![prefill; HashSet::size_in_account(CAPACITY)];
    unsafe {
        HashSetZeroCopy::from_bytes_zero_copy_init(&mut bytes, CAPACITY, SEQUENCE_THRESHOLD)
            .unwrap();
    }

    let mut snapshots = Vec::new();
    {
        let mut hash_set =
            unsafe { HashSetZeroCopy::from_bytes_zero_copy_mut(&mut bytes).unwrap() };
        assert_eq!(hash_set.insert(&BigUint::from(7_u8), 0).unwrap(), 0);
    }
    snapshots.push(bytes[BUCKET_OFFSET..].try_into().unwrap());

    {
        let mut hash_set =
            unsafe { HashSetZeroCopy::from_bytes_zero_copy_mut(&mut bytes).unwrap() };
        hash_set.mark_with_sequence_number(0, 0).unwrap();
    }
    snapshots.push(bytes[BUCKET_OFFSET..].try_into().unwrap());

    {
        let mut hash_set =
            unsafe { HashSetZeroCopy::from_bytes_zero_copy_mut(&mut bytes).unwrap() };
        assert_eq!(
            hash_set
                .insert(&BigUint::from(9_u8), SEQUENCE_THRESHOLD)
                .unwrap(),
            0
        );
    }
    snapshots.push(bytes[BUCKET_OFFSET..].try_into().unwrap());

    snapshots
}

#[test]
fn zero_copy_writes_every_bucket_byte() {
    const CAPACITY: usize = 3;
    const SEQUENCE_THRESHOLD: usize = 10;

    let initialize = |prefill| {
        let mut bytes = vec![prefill; HashSet::size_in_account(CAPACITY)];
        unsafe {
            HashSetZeroCopy::from_bytes_zero_copy_init(&mut bytes, CAPACITY, SEQUENCE_THRESHOLD)
                .unwrap();
        }
        bytes
    };

    let initialized_from_ff = initialize(0xFF);
    let initialized_from_aa = initialize(0xAA);
    assert_eq!(initialized_from_ff, initialized_from_aa);
    assert_eq!(
        &initialized_from_ff[16..BUCKET_OFFSET],
        &[0; BUCKET_OFFSET - 16]
    );
    for bucket in initialized_from_ff[BUCKET_OFFSET..].chunks_exact(BUCKET_SIZE) {
        assert_eq!(bucket, expected_bucket(2, 0, [0; 32]));
    }

    let snapshots_from_ff = run_bucket_transitions(0xFF);
    let snapshots_from_aa = run_bucket_transitions(0xAA);
    assert_eq!(snapshots_from_ff, snapshots_from_aa);
    assert_eq!(
        snapshots_from_ff,
        vec![
            expected_bucket(0, 0, value_bytes(7)),
            expected_bucket(1, SEQUENCE_THRESHOLD, value_bytes(7)),
            expected_bucket(0, 0, value_bytes(9)),
        ]
    );
}

#[test]
fn zero_copy_rejects_buffer_without_reserved_gap() {
    const CAPACITY: usize = 2;
    const SEQUENCE_THRESHOLD: usize = 10;
    let expected_size = HashSet::size_in_account(CAPACITY);
    assert_eq!(expected_size, BUCKET_OFFSET + CAPACITY * BUCKET_SIZE);

    let mut init_bytes = vec![0xAA; expected_size - 8];
    let before = init_bytes.clone();
    let error = unsafe {
        HashSetZeroCopy::from_bytes_zero_copy_init(&mut init_bytes, CAPACITY, SEQUENCE_THRESHOLD)
    }
    .unwrap_err();
    assert!(matches!(
        error,
        HashSetError::BufferSize(expected, actual)
            if expected == expected_size && actual == expected_size - 8
    ));
    assert_eq!(init_bytes, before);

    let mut load_bytes = vec![0; expected_size - 8];
    load_bytes[..8].copy_from_slice(&CAPACITY.to_le_bytes());
    load_bytes[8..16].copy_from_slice(&SEQUENCE_THRESHOLD.to_le_bytes());
    let error = unsafe { HashSetZeroCopy::from_bytes_zero_copy_mut(&mut load_bytes) }.unwrap_err();
    assert!(matches!(
        error,
        HashSetError::BufferSize(expected, actual)
            if expected == expected_size && actual == expected_size - 8
    ));
}
