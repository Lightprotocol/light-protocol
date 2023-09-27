use criterion::{criterion_group, criterion_main, Criterion, black_box};
use ark_std::ops::Mul;

use decode::discrete_log::{
    DiscreteLog,
    EdwardsAffine, 
    G, 
    F,
    GENERATOR_X,
    GENERATOR_Y,
};

const BASE: EdwardsAffine = EdwardsAffine::new_unchecked(GENERATOR_X, GENERATOR_Y);

//TODO: Create lookup table if not found
//TODO: Bench based on size env but consider the lazystatic in discrete_log
fn bench_decode_correctness_threaded(scalar: u64, threads: usize) -> u64 {
    let base = G::from(BASE);
    let mut instance = DiscreteLog::new(base,  base.mul(F::from(scalar)), 16);
    instance.num_threads(threads, 16).unwrap();

    let decoded = instance.decode_u32(16).unwrap();

    assert_eq!(scalar, decoded);
    decoded
}

pub fn criterion_benchmark(c: &mut Criterion) {
    
    for i in 0..5 {
        let thread_num = 2_u8.pow(i);
        let id = format!("Decode Baby Jubjub ECDLP => {thread_num} thread");
        c.bench_function(&id, |b| b.iter(|| bench_decode_correctness_threaded(black_box(20), thread_num as usize)));    
    }
}

criterion_group!(
    name = benches;
    config = Criterion::default();//.measurement_time(std::time::Duration::new(100, 400_000_000)).sample_size(10);
    targets = criterion_benchmark
);
criterion_main!(benches);

