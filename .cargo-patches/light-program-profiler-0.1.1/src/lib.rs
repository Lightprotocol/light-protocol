#![allow(unused_variables)]
pub use light_profiler_macro::profile;

#[inline(always)]
pub fn log_compute_units_start(id: &str, id_len: u64) {
    #[cfg(target_os = "solana")]
    unsafe {
        sol_log_compute_units_start(id.as_ptr() as u64, id_len, 0, 0, 0);
    }
}

#[inline(always)]
pub fn log_compute_units_end(id: &str, id_len: u64) {
    #[cfg(target_os = "solana")]
    unsafe {
        sol_log_compute_units_end(id.as_ptr() as u64, id_len, 0, 0, 0);
    }
}

#[cfg(feature = "profile-heap")]
pub fn log_compute_units_start_with_heap(id: &str, id_len: u64, heap_value: u64) {
    #[cfg(target_os = "solana")]
    unsafe {
        sol_log_compute_units_start(id.as_ptr() as u64, id_len, heap_value, 1, 0);
    }
}

#[cfg(feature = "profile-heap")]
pub fn log_compute_units_end_with_heap(id: &str, id_len: u64, heap_value: u64) {
    #[cfg(target_os = "solana")]
    unsafe {
        sol_log_compute_units_end(id.as_ptr() as u64, id_len, heap_value, 1, 0);
    }
}

#[cfg(target_os = "solana")]
extern "C" {
    fn sol_log_compute_units_start(
        id_addr: u64,
        id_len: u64,
        heap_value: u64,
        with_heap: u64,
        _arg5: u64,
    );
}

#[cfg(target_os = "solana")]
extern "C" {

    fn sol_log_compute_units_end(
        id_addr: u64,
        id_len: u64,
        heap_value: u64,
        with_heap: u64,
        _arg5: u64,
    );
}
