#[macro_export]
macro_rules! bench_sbf_start {
    ($custom_msg:literal) => {
        // Conditionally compile only if on Solana OS and feature "bench-sbf" is enabled
        #[cfg(all(target_os = "solana", feature = "bench-sbf"))]
        {
            // Log the total heap with a custom message indicating the start
            light_heap::GLOBAL_ALLOCATOR
                .log_total_heap(format!("{}_start_bench_cu", $custom_msg).as_str());
            // Log the number of compute units used
            anchor_lang::solana_program::log::sol_log_compute_units();
        }
    };
}

#[macro_export]
macro_rules! bench_sbf_end {
    ($custom_msg:literal) => {
        // Conditionally compile only if on Solana OS and feature "bench-sbf" is enabled
        #[cfg(all(target_os = "solana", feature = "bench-sbf"))]
        {
            anchor_lang::solana_program::log::sol_log_compute_units();
            // Log the total heap with a custom message indicating the end
            light_heap::GLOBAL_ALLOCATOR
                .log_total_heap(format!("{}_end_bench_cu", $custom_msg).as_str());
            // Log the number of compute units used
        }
    };
}
