cd /Users/ananas/dev/light-protocol2/programs/compressed-token/program && bash ~/dev/agave/cargo-build-sbf --features profile-program;

RUST_BACKTRACE=1 bash ~/dev/agave/cargo-test-sbf -p compressed-token-test --test metadata -- --nocapture > /Users/ananas/dev/light-protocol2/target/bench.log 2>&1
