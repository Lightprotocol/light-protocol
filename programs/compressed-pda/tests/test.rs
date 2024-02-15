#[cfg(test)]
mod test {
    use psp_compressed_pda::ID;
    use solana_program_test::ProgramTest;

    #[tokio::test]
    async fn test_create_and_update_group() {
        let mut program_test = ProgramTest::default();
        program_test.add_program("psp_compressed_pda", ID, None);

        program_test.set_compute_max_units(1_400_000u64);

        let _context = program_test.start_with_context().await;
    }
}
