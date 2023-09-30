use decode::discrete_log::{
    DecodePrecomputation, EdwardsIterator, Zero,
    F, G,
    BabyJubConfig, TECurveConfig,
    CurveGroup, Projective,
    PrimeField, BigInteger
};
use std::{fs::File, io::Write, path::PathBuf};
use std::collections::HashMap;
use indicatif::{ProgressBar, ProgressStyle};
use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(short, long, default_value_t = 16)]
    size: u32,
}
//TODO: Check if table already exists
#[allow(non_snake_case)]
pub fn build_lookup_table(args: Args) { 
    let precomputation_size = args.size;
    
    let mut f = File::create(PathBuf::from(
        format!("./decode-babyjubjub/src/decode_lookup_table{}.bincode", precomputation_size),
    )).unwrap();

    let decode_u32_precomputation_for_G = decode_u32_precomputation(
        G::from(BabyJubConfig::GENERATOR), 
        precomputation_size
    );

    f.write_all(
        &bincode::serialize(&decode_u32_precomputation_for_G).unwrap()
    ).unwrap();
    
}

/// Builds a HashMap of 2^size elements
#[allow(dead_code)]
pub fn decode_u32_precomputation(base: Projective<BabyJubConfig>, size: u32) -> DecodePrecomputation {
    let total_iterations = 2_u64.pow(size);
    let pb = ProgressBar::new(total_iterations);
    pb.set_style(
        ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed}] {bar:50.cyan/blue} {pos}/{len} [{percent}%] {msg}")            
        .unwrap()
    );

    let mut hashmap = HashMap::new();
    let range = 32 - size;
    let two_range_scalar = F::from(2_u64.pow(range + 1 as u32));

    let identity = G::zero();
    let generator = base * &two_range_scalar;

    let edwards_iter = EdwardsIterator::new((identity, 0), (generator, 1));
    for (point, x_hi) in edwards_iter.take(total_iterations as usize) {
        let key = point.into_affine().x.into_bigint().to_bytes_le().try_into().unwrap();
        hashmap.insert(key, x_hi as u32);
        pb.inc(1);
    }

    pb.finish_with_message(format!("\n\x1b[32m\u{2714}\x1b[0m Lookup table {size} created successfully.\n"));    
    DecodePrecomputation(hashmap)
}