cargo build-bpf --manifest-path=./Cargo.toml --bpf-out-dir=dist/program
#solana program deploy /home/swen/Desktop/crypto/onchain-experiments/program_prep_inputs/dist/program/Prepare_Inputs.so
solana program deploy /home/ananas/Light/onchain-experiments/program_prep_inputs/dist/program/Prepare_Inputs.so
