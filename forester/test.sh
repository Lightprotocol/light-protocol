#!/bin/bash

../cli/test_bin/run start-prover --run-mode forester
../cli/test_bin/run test-validator --skip-prover --skip-indexer
sleep 10 
(cd ../../photon && cargo run 2>&1 > photon.log)

sleep 60 * 5

RUST_LOG=forester=debug,forester_utils=debug cargo test --package forester test_state_indexer_async_batched -- --nocapture
