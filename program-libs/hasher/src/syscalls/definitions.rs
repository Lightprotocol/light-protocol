//! This module is a partial copy from
//! [solana-program](https://github.com/solana-labs/solana/blob/master/sdk/program/src/syscalls/definitions.rs),
//! which is licensed under Apache License 2.0.

#[cfg(target_feature = "static-syscalls")]
macro_rules! define_syscall {
    (fn $name:ident($($arg:ident: $typ:ty),*) -> $ret:ty) => {
		#[inline]
        pub unsafe fn $name($($arg: $typ),*) -> $ret {
			// this enum is used to force the hash to be computed in a const context
			#[repr(usize)]
			enum Syscall {
				Code = sys_hash(stringify!($name)),
			}

            let syscall: extern "C" fn($($arg: $typ),*) -> $ret = core::mem::transmute(Syscall::Code);
            syscall($($arg),*)
        }

    };
    (fn $name:ident($($arg:ident: $typ:ty),*)) => {
        define_syscall!(fn $name($($arg: $typ),*) -> ());
    }
}

#[cfg(not(target_feature = "static-syscalls"))]
macro_rules! define_syscall {
	(fn $name:ident($($arg:ident: $typ:ty),*) -> $ret:ty) => {
		extern "C" {
			pub fn $name($($arg: $typ),*) -> $ret;
		}
	};
	(fn $name:ident($($arg:ident: $typ:ty),*)) => {
		define_syscall!(fn $name($($arg: $typ),*) -> ());
	}
}

define_syscall!(fn sol_sha256(vals: *const u8, val_len: u64, hash_result: *mut u8) -> u64);
define_syscall!(fn sol_keccak256(vals: *const u8, val_len: u64, hash_result: *mut u8) -> u64);
define_syscall!(fn sol_poseidon(parameters: u64, endianness: u64, vals: *const u8, val_len: u64, hash_result: *mut u8) -> u64);
