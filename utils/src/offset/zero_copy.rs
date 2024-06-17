use std::mem;

/// Casts a part of provided `bytes` buffer with the given `offset` to a
/// mutable pointer to `T`.
///
/// Should be used for single values.
///
/// # Safety
///
/// This is higly unsafe. This function doesn't ensure alignment and
/// correctness of provided buffer. The responsibility of such checks is on
/// the caller.
pub unsafe fn read_ptr_at<T>(bytes: &[u8], offset: &mut usize) -> *mut T {
    let size = mem::size_of::<T>();
    let ptr = bytes[*offset..*offset + size].as_ptr() as *mut T;
    *offset += size;
    ptr
}

/// Casts a part of provided `bytes` buffer with the given `offset` to a
/// mutable pointer to `T`.
///
/// Should be used for array-type sequences.
///
/// # Safety
///
/// This is higly unsafe. This function doesn't ensure alignment and
/// correctness of provided buffer. The responsibility of such checks is on
/// the caller.
pub unsafe fn read_array_like_ptr_at<T>(bytes: &[u8], offset: &mut usize, len: usize) -> *mut T {
    let size = mem::size_of::<T>() * len;
    let ptr = bytes[*offset..*offset + size].as_ptr() as *mut T;
    *offset += size;
    ptr
}

/// Writes provided `data` into provided `bytes` buffer with the given
/// `offset`.
pub fn write_at<T>(bytes: &mut [u8], data: &[u8], offset: &mut usize) {
    let size = mem::size_of::<T>();
    bytes[*offset..*offset + size].copy_from_slice(data);
    *offset += size;
}

#[cfg(test)]
mod test {
    use super::*;

    use bytemuck::{Pod, Zeroable};
    use memoffset::offset_of;

    #[test]
    fn test_read_ptr_at() {
        #[derive(Clone, Copy, Pod, Zeroable)]
        #[repr(C)]
        struct TestStruct {
            a: isize,
            b: usize,
            c: i64,
            d: u64,
            e: i32,
            f: u32,
            g: i16,
            _padding_1: [u8; 2],
            h: u16,
            _padding_2: [u8; 2],
            i: i8,
            _padding_3: [i8; 3],
            j: u8,
            _padding_4: [i8; 3],
        }

        let mut buf = vec![0_u8; mem::size_of::<TestStruct>()];
        let s = buf.as_mut_ptr() as *mut TestStruct;

        unsafe {
            (*s).a = -9223372036854771632;
            (*s).b = 9223372036854771632;
            (*s).c = -9223372036854771632;
            (*s).d = 9223372036854771632;
            (*s).e = -2147483623;
            (*s).f = 2147483623;
            (*s).g = -32721;
            (*s).h = 32721;
            (*s).i = -127;
            (*s).j = 127;

            let mut offset = offset_of!(TestStruct, a);
            assert_eq!(offset, 0);
            assert_eq!(
                *read_ptr_at::<isize>(&buf, &mut offset),
                -9223372036854771632
            );
            assert_eq!(offset, 8);

            let mut offset = offset_of!(TestStruct, b);
            assert_eq!(offset, 8);
            assert_eq!(
                *read_ptr_at::<usize>(&buf, &mut offset),
                9223372036854771632
            );
            assert_eq!(offset, 16);

            let mut offset = offset_of!(TestStruct, c);
            assert_eq!(offset, 16);
            assert_eq!(*read_ptr_at::<i64>(&buf, &mut offset), -9223372036854771632);
            assert_eq!(offset, 24);

            let mut offset = offset_of!(TestStruct, d);
            assert_eq!(offset, 24);
            assert_eq!(*read_ptr_at::<u64>(&buf, &mut offset), 9223372036854771632);
            assert_eq!(offset, 32);

            let mut offset = offset_of!(TestStruct, e);
            assert_eq!(offset, 32);
            assert_eq!(*read_ptr_at::<i32>(&buf, &mut offset), -2147483623);
            assert_eq!(offset, 36);

            let mut offset = offset_of!(TestStruct, f);
            assert_eq!(offset, 36);
            assert_eq!(*read_ptr_at::<u32>(&buf, &mut offset), 2147483623);
            assert_eq!(offset, 40);

            let mut offset = offset_of!(TestStruct, g);
            assert_eq!(offset, 40);
            assert_eq!(*read_ptr_at::<i16>(&buf, &mut offset), -32721);
            assert_eq!(offset, 42);

            let mut offset = offset_of!(TestStruct, h);
            assert_eq!(offset, 44);
            assert_eq!(*read_ptr_at::<u16>(&buf, &mut offset), 32721);
            assert_eq!(offset, 46);

            let mut offset = offset_of!(TestStruct, i);
            assert_eq!(offset, 48);
            assert_eq!(*read_ptr_at::<i8>(&buf, &mut offset), -127);
            assert_eq!(offset, 49);

            let mut offset = offset_of!(TestStruct, j);
            assert_eq!(offset, 52);
            assert_eq!(*read_ptr_at::<u8>(&buf, &mut offset), 127);
            assert_eq!(offset, 53);
        }
    }

    #[test]
    fn test_read_array_like_ptr_at() {
        #[derive(Clone, Copy, Pod, Zeroable)]
        #[repr(C)]
        struct TestStruct {
            a: [i64; 32],
            b: [u64; 32],
        }

        let mut buf = vec![0_u8; mem::size_of::<TestStruct>()];
        let s = buf.as_mut_ptr() as *mut TestStruct;

        unsafe {
            for (i, element) in (*s).a.iter_mut().enumerate() {
                *element = -(i as i64);
            }
            for (i, element) in (*s).b.iter_mut().enumerate() {
                *element = i as u64;
            }

            let mut offset = offset_of!(TestStruct, a);
            assert_eq!(offset, 0);
            let ptr: *mut i64 = read_array_like_ptr_at(&buf, &mut offset, 32);
            for i in 0..32 {
                assert_eq!(*(ptr.add(i)), -(i as i64));
            }
            assert_eq!(offset, 256);

            let mut offset = offset_of!(TestStruct, b);
            assert_eq!(offset, 256);
            let ptr: *mut u64 = read_array_like_ptr_at(&buf, &mut offset, 32);
            for i in 0..32 {
                assert_eq!(*(ptr.add(i)), i as u64);
            }
            assert_eq!(offset, 512);
        }
    }

    #[test]
    fn test_write_at() {
        #[derive(Clone, Copy, Pod, Zeroable)]
        #[repr(C)]
        struct TestStruct {
            a: isize,
            b: usize,
            c: i64,
            d: u64,
            e: i32,
            f: u32,
            g: i16,
            _padding_1: [u8; 2],
            h: u16,
            _padding_2: [u8; 2],
            i: i8,
            _padding_3: [i8; 3],
            j: u8,
            _padding_4: [i8; 3],
        }

        let mut buf = vec![0_u8; mem::size_of::<TestStruct>()];

        let a: isize = -9223372036854771632;
        let b: usize = 9223372036854771632;
        let c: i64 = -9223372036854771632;
        let d: u64 = 9223372036854771632;
        let e: i32 = -2147483623;
        let f: u32 = 2147483623;
        let g: i16 = -32721;
        let h: u16 = 32721;
        let i: i8 = -127;
        let j: u8 = 127;

        let mut offset = offset_of!(TestStruct, a);
        assert_eq!(offset, 0);
        write_at::<isize>(&mut buf, &a.to_ne_bytes(), &mut offset);
        assert_eq!(offset, 8);

        let mut offset = offset_of!(TestStruct, b);
        assert_eq!(offset, 8);
        write_at::<usize>(&mut buf, &b.to_ne_bytes(), &mut offset);
        assert_eq!(offset, 16);

        let mut offset = offset_of!(TestStruct, c);
        assert_eq!(offset, 16);
        write_at::<i64>(&mut buf, &c.to_ne_bytes(), &mut offset);
        assert_eq!(offset, 24);

        let mut offset = offset_of!(TestStruct, d);
        assert_eq!(offset, 24);
        write_at::<u64>(&mut buf, &d.to_ne_bytes(), &mut offset);
        assert_eq!(offset, 32);

        let mut offset = offset_of!(TestStruct, e);
        assert_eq!(offset, 32);
        write_at::<i32>(&mut buf, &e.to_ne_bytes(), &mut offset);
        assert_eq!(offset, 36);

        let mut offset = offset_of!(TestStruct, f);
        assert_eq!(offset, 36);
        write_at::<u32>(&mut buf, &f.to_ne_bytes(), &mut offset);
        assert_eq!(offset, 40);

        let mut offset = offset_of!(TestStruct, g);
        assert_eq!(offset, 40);
        write_at::<i16>(&mut buf, &g.to_ne_bytes(), &mut offset);
        assert_eq!(offset, 42);

        let mut offset = offset_of!(TestStruct, h);
        assert_eq!(offset, 44);
        write_at::<u16>(&mut buf, &h.to_ne_bytes(), &mut offset);
        assert_eq!(offset, 46);

        let mut offset = offset_of!(TestStruct, i);
        assert_eq!(offset, 48);
        write_at::<i8>(&mut buf, &i.to_ne_bytes(), &mut offset);
        assert_eq!(offset, 49);

        let mut offset = offset_of!(TestStruct, j);
        assert_eq!(offset, 52);
        write_at::<u8>(&mut buf, &j.to_ne_bytes(), &mut offset);
        assert_eq!(offset, 53);

        let s = buf.as_mut_ptr() as *mut TestStruct;

        unsafe {
            assert_eq!((*s).a, a);
            assert_eq!((*s).b, b);
            assert_eq!((*s).c, c);
            assert_eq!((*s).d, d);
            assert_eq!((*s).e, e);
            assert_eq!((*s).f, f);
            assert_eq!((*s).g, g);
            assert_eq!((*s).h, h);
            assert_eq!((*s).i, i);
            assert_eq!((*s).j, j);
        }
    }
}
