cargo build-bpf --manifest-path=./Cargo.toml --bpf-out-dir=dist/program
solana program deploy ./dist/program/Testing_Hardcoded_Params_devnet_new.so
