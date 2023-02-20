sh build-sdk.sh;
cd light-system-programs && anchor build && npm test && npm run test-merkle-tree && npm run test-verifiers && cd ..;
cd light-sdk-ts && npm test && sleep 1 && cd -;
cd mock-app-verifier && anchor build && npm test && npm run test-verifiers && cd -;
cd light-circuits && npm run test && cd -;
cd groth16-solana && cargo test && cd -;
# && cd programs/merkle_tree_program && cargo test