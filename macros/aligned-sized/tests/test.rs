use std::mem;

use aligned_sized::aligned_sized;

#[aligned_sized]
#[derive(Debug)]
#[allow(dead_code)]
struct TestStruct {
    pub a: u64,
    pub b: u32,
    pub c: i32,
    pub d: TestStructNested,
}

#[aligned_sized]
#[derive(Debug)]
#[allow(dead_code)]
struct TestStructNested {
    pub e: usize,
    pub f: isize,
}

#[test]
fn test_aligned_sized() {
    let expected_size =
        // a
        mem::size_of::<u64>()
        // b
        + mem::size_of::<u32>()
        // c
        + mem::size_of::<i32>()
        // e
        + mem::size_of::<usize>()
        // f
        + mem::size_of::<isize>();
    assert_eq!(TestStruct::LEN, expected_size);
}

#[aligned_sized]
#[derive(Debug)]
#[allow(dead_code)]
struct TestStructWithDefinedSize {
    pub a: u64,
    #[size = 50]
    pub b: Vec<u8>,
}

#[test]
fn test_aligned_sized_defined_size() {
    let expected_size =
        // a
        mem::size_of::<u64>()
        // b
        + 50;
    assert_eq!(TestStructWithDefinedSize::LEN, expected_size);
}
