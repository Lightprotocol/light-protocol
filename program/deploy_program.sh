cargo build-bpf --manifest-path=./Cargo.toml --bpf-out-dir=dist/program
solana program deploy ./dist/program/light_protocol_core.so
