sh build-sdk.sh;
cd light-system-programs && anchor build && cd programs/merkle_tree_program && cargo test && cd - && npm test && npm run test-merkle-tree && npm run test-verifiers && cd ..;
cd light-sdk-ts && npm test && sleep 1 && cd -;
cd light-circuits && npm run test && cd -;
cd groth16-solana && cargo test && cd -;