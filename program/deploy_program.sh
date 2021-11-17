cargo build-bpf --manifest-path=./Cargo.toml --bpf-out-dir=dist/program
#solana program deploy /home/swen/Desktop/crypto/onchain-experiments/program/dist/program/Testing_Hardcoded_Params.so
solana program deploy /home/ananas/Light/onchain-experiments/program/dist/program/Testing_Hardcoded_Params_devnet_new.so
