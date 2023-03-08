sh build-sdk.sh;
cd light-system-programs && anchor build && cd programs/merkle_tree_program && cd - && yarn test && yarn run test-merkle-tree && yarn run test-verifiers && cd ..;
cd light-sdk-ts && yarn test && sleep 1 && cd -;
cd mock-app-verifier && anchor build && yarn test && yarn run test-verifiers && cd -;
cd light-circuits && yarn run test && cd -;
# cd groth16-solana && cargo test && cd -;

