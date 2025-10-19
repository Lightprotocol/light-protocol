fn main() {
    // Verify we're building for the Solana BPF target or running tests
    let target = std::env::var("TARGET").unwrap_or_default();

    // When building for Solana, verify we're in no_std mode
    if target.contains("bpf") || target.contains("sbf") {
        // This will cause a compile error if std is somehow enabled
        // for the Solana target
        if cfg!(feature = "std") {
            panic!("FATAL: std feature is enabled for no_std target!");
        }
    }

    // Set a cfg flag to indicate we're in strict no_std mode
    println!("cargo:rustc-cfg=strict_nostd");
}
