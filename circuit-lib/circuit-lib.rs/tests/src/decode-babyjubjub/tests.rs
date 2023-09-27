pub mod decode_tests {
    use std::time::Instant;
    use decode::discrete_log::*;
    
    #[test]
    #[allow(non_snake_case)]
    fn test_serialize_decode_u32_precomputation_for_G() {
        let decode_u32_precomputation_for_G = decode_u32_precomputation(G::from(BabyJubConfig::GENERATOR), 16);

        if decode_u32_precomputation_for_G.0 != DECODE_PRECOMPUTATION_FOR_G.0 {
            use std::{fs::File, io::Write, path::PathBuf};
            let mut f = File::create(PathBuf::from(
                "src/decode_lookup_table16.bincode",
            ))
            .unwrap();
            f.write_all(&bincode::serialize(&decode_u32_precomputation_for_G).unwrap())
                .unwrap();
            panic!("Rebuild and run this test again");
        }
    }

    #[test]
    fn test_decode_correctness() {
        // general case
        let amount: u64 = 120;
        let base = G::from(BASE);

        let instance = DiscreteLog::new(base,  base.mul(F::from(amount)), 16);

        // Very informal measurements for now
        let start_computation = Instant::now();
        let decoded = instance.decode_u32(16);
        let computation_secs = start_computation.elapsed().as_secs_f64();
        assert_eq!(amount, decoded.unwrap());

        println!("single thread discrete log computation secs: {computation_secs:?} sec");
    }

    #[test]
    fn test_decode_correctness_threaded() {
        // general case
        let amount: u64 = 55;
        let base = G::from(BASE);

        let mut instance = DiscreteLog::new(base,  base.mul(F::from(amount)), 16);
        instance.num_threads(4, 17).unwrap();

        // Very informal measurements for now
        let start_computation = Instant::now();
        let decoded = instance.decode_u32(16);
        let computation_secs = start_computation.elapsed().as_secs_f64();

        assert_eq!(amount, decoded.unwrap());

        println!("4 thread discrete log computation: {computation_secs:?} sec");

        // amount 0
        let amount: u64 = 0;

        let instance = DiscreteLog::new(base,  base.mul(F::from(amount)), 16);

        let decoded = instance.decode_u32(16);
        assert_eq!(amount, decoded.unwrap());

        // amount 1
        let amount: u64 = 1;

        let instance = DiscreteLog::new(base,  base.mul(F::from(amount)), 16);

        let decoded = instance.decode_u32(16);
        assert_eq!(amount, decoded.unwrap());

        // amount 2
        let amount: u64 = 2;

        let instance = DiscreteLog::new(base,  base.mul(F::from(amount)), 16);

        let decoded = instance.decode_u32(16);
        assert_eq!(amount, decoded.unwrap());

        // amount 3
        let amount: u64 = 3;

        let instance = DiscreteLog::new(base,  base.mul(F::from(amount)), 16);

        let decoded = instance.decode_u32(16);
        assert_eq!(amount, decoded.unwrap());

        // max amount
        let amount: u64 = (1_u64 << 32) - 1;

        let instance = DiscreteLog::new(base,  base.mul(F::from(amount)), 16);

        let decoded = instance.decode_u32(16);
        assert_eq!(amount, decoded.unwrap());
    }
}