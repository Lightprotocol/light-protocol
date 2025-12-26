//! Build script to compile SP1 programs to ELF binaries.

use sp1_build::{build_program_with_args, BuildArgs};

fn main() {
    // Build the batch-append program
    build_program_with_args(
        "../programs/batch-append",
        BuildArgs {
            output_directory: Some("../programs/batch-append/elf".to_string()),
            ..Default::default()
        },
    );

    // Build the batch-update program
    build_program_with_args(
        "../programs/batch-update",
        BuildArgs {
            output_directory: Some("../programs/batch-update/elf".to_string()),
            ..Default::default()
        },
    );

    // Build the batch-address-append program
    build_program_with_args(
        "../programs/batch-address-append",
        BuildArgs {
            output_directory: Some("../programs/batch-address-append/elf".to_string()),
            ..Default::default()
        },
    );
}
